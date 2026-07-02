//! Call Manager
//!
//! Orchestrates WebRTC calls, signaling, and audio I/O.

use super::{
    call::{Call, CallDirection, CallEndReason, CallState},
    video::VideoCodec,
    webrtc::{build_turn_config, WebRTCPeer},
    Result, VoipError,
};
#[cfg(feature = "voip")]
use super::codec::{OpusConfig, OpusEncoder};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
#[cfg(feature = "voip")]
use tokio::sync::Mutex;
use webrtc::ice_transport::ice_server::RTCIceServer;
#[cfg(feature = "voip")]
use webrtc::track::track_local::TrackLocalWriter;

/// TURN credentials from server
#[derive(Debug, Clone)]
pub struct TurnCredentials {
    pub username: String,
    pub password: String,
    pub uris: Vec<String>,
}

/// Call manager events
#[derive(Debug, Clone)]
pub enum CallEvent {
    /// Incoming call received
    IncomingCall {
        call_id: String,
        from_peer_id: String,
    },

    /// Call state changed
    StateChanged {
        call_id: String,
        new_state: CallState,
    },

    /// Call ended
    Ended {
        call_id: String,
        reason: CallEndReason,
    },

    /// Remote audio received (ready to play)
    AudioReceived {
        call_id: String,
        data: Vec<u8>,
        sample_rate: u32,
        channels: u32,
    },

    /// Video enabled for call
    VideoEnabled {
        call_id: String,
        codec: VideoCodec,
    },

    /// Video disabled for call
    VideoDisabled {
        call_id: String,
    },

    /// Remote video frame received
    VideoFrameReceived {
        call_id: String,
        frame_data: Vec<u8>,
        width: u32,
        height: u32,
    },

    /// Mute state toggled
    MuteToggled {
        call_id: String,
        is_muted: bool,
    },

    /// Speakerphone state toggled
    SpeakerphoneToggled {
        call_id: String,
        enabled: bool,
    },

    /// Camera switch requested (platform should handle)
    CameraSwitchRequested {
        call_id: String,
    },

    /// Signaling: Need to send offer to remote peer
    SignalingOffer {
        call_id: String,
        to_peer_id: String,
        sdp: String,
    },

    /// Signaling: Need to send answer to remote peer
    SignalingAnswer {
        call_id: String,
        to_peer_id: String,
        sdp: String,
    },

    /// Signaling: Need to send ICE candidate to remote peer
    SignalingIceCandidate {
        call_id: String,
        to_peer_id: String,
        candidate: String,
        sdp_mid: Option<String>,
        sdp_m_line_index: Option<u16>,
    },
}

/// Manages all active calls
pub struct CallManager {
    /// Active calls by call_id
    pub(crate) calls: Arc<RwLock<HashMap<String, Call>>>,

    /// WebRTC peers by call_id (wrapped in RwLock for video track mutations)
    peers: Arc<RwLock<HashMap<String, Arc<RwLock<WebRTCPeer>>>>>,

    /// Video enabled state by call_id
    video_enabled: Arc<RwLock<HashMap<String, bool>>>,

    /// Opus encoder per call (audio input from platform)
    #[cfg(feature = "voip")]
    audio_encoders: Arc<RwLock<HashMap<String, Arc<Mutex<OpusEncoder>>>>>,

    /// Event broadcaster
    event_tx: broadcast::Sender<CallEvent>,

    /// TURN credentials (cached)
    turn_credentials: Arc<RwLock<Option<TurnCredentials>>>,

    /// ICE candidates que chegaram antes do peer existir (race ICE-antes-do-offer)
    pending_ice: Arc<RwLock<HashMap<String, Vec<(String, Option<String>, Option<u16>)>>>>,
}

impl CallManager {
    /// Create a new call manager
    pub fn new() -> Self {
        // 1024: sinalização + frames de evento podem acumular sob lag; 128
        // derrubava eventos silenciosamente
        let (event_tx, _event_rx) = broadcast::channel(1024);

        Self {
            calls: Arc::new(RwLock::new(HashMap::new())),
            peers: Arc::new(RwLock::new(HashMap::new())),
            video_enabled: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            turn_credentials: Arc::new(RwLock::new(None)),
            pending_ice: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(feature = "voip")]
            audio_encoders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set TURN credentials (fetched from server)
    pub async fn set_turn_credentials(&self, credentials: TurnCredentials) {
        let mut turn = self.turn_credentials.write().await;
        *turn = Some(credentials);
        tracing::info!("✅ TURN credentials configured");
    }

    /// Get ICE servers configuration
    async fn get_ice_servers(&self) -> Vec<RTCIceServer> {
        let turn = self.turn_credentials.read().await;

        if let Some(creds) = turn.as_ref() {
            build_turn_config(
                creds.uris.clone(),
                creds.username.clone(),
                creds.password.clone(),
            )
        } else {
            // Fallback to public STUN only
            vec![RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                ..Default::default()
            }]
        }
    }

    /// Start an outgoing call
    pub async fn start_call(&self, remote_peer_id: String) -> Result<String> {
        let call = Call::new_outgoing(remote_peer_id.clone());
        let call_id = call.id.clone();

        // Create WebRTC peer connection
        let ice_servers = self.get_ice_servers().await;
        let mut peer = WebRTCPeer::new(ice_servers).await?;

        // Register callback for remote video frames
        let event_tx = self.event_tx.clone();
        let call_id_for_callback = call_id.clone();
        peer.on_remote_video_frame(move |frame_data, width, height| {
            let _ = event_tx.send(CallEvent::VideoFrameReceived {
                call_id: call_id_for_callback.clone(),
                frame_data,
                width,
                height,
            });
        })
        .await?;

        // Register callback for remote audio frames
        let audio_event_tx = self.event_tx.clone();
        let call_id_for_audio = call_id.clone();
        peer.on_remote_audio_frame(move |pcm_data, sample_rate, channels| {
            let _ = audio_event_tx.send(CallEvent::AudioReceived {
                call_id: call_id_for_audio.clone(),
                data: pcm_data,
                sample_rate,
                channels,
            });
        })
        .await?;

        // Register callback for local ICE candidates
        let ice_event_tx = self.event_tx.clone();
        let call_id_for_ice = call_id.clone();
        let remote_peer_for_ice = remote_peer_id.clone();
        peer.on_ice_candidate(move |candidate, sdp_mid, sdp_m_line_index| {
            let _ = ice_event_tx.send(CallEvent::SignalingIceCandidate {
                call_id: call_id_for_ice.clone(),
                to_peer_id: remote_peer_for_ice.clone(),
                candidate,
                sdp_mid,
                sdp_m_line_index,
            });
        })
        .await?;

        // Add audio track (only with voip feature - video-only calls don't need audio)
        #[cfg(feature = "voip")]
        peer.add_audio_track().await?;

        // Create offer
        let offer_sdp = peer.create_offer().await?;

        // Store call and peer
        {
            let mut calls = self.calls.write().await;
            calls.insert(call_id.clone(), call);
        }
        {
            let mut peers = self.peers.write().await;
            peers.insert(call_id.clone(), Arc::new(RwLock::new(peer)));
        }
        // Aplicar ICE candidates que chegaram antes do peer existir
        self.drain_pending_ice(&call_id).await;

        // Emit state change event
        let _ = self.event_tx.send(CallEvent::StateChanged {
            call_id: call_id.clone(),
            new_state: CallState::Initiating,
        });

        // Emit signaling offer event (for network layer to send)
        let _ = self.event_tx.send(CallEvent::SignalingOffer {
            call_id: call_id.clone(),
            to_peer_id: remote_peer_id.clone(),
            sdp: offer_sdp,
        });

        tracing::info!("📞 Started outgoing call to {}", remote_peer_id);

        Ok(call_id)
    }

    /// Handle incoming call (from signaling)
    pub async fn handle_incoming_call(
        &self,
        call_id: String,
        remote_peer_id: String,
        offer_sdp: String,
    ) -> Result<()> {
        let call = Call::new_incoming(call_id.clone(), remote_peer_id.clone());

        // Create WebRTC peer connection
        let ice_servers = self.get_ice_servers().await;
        let mut peer = WebRTCPeer::new(ice_servers).await?;

        // Register callback for remote video frames
        let event_tx = self.event_tx.clone();
        let call_id_for_callback = call_id.clone();
        peer.on_remote_video_frame(move |frame_data, width, height| {
            let _ = event_tx.send(CallEvent::VideoFrameReceived {
                call_id: call_id_for_callback.clone(),
                frame_data,
                width,
                height,
            });
        })
        .await?;

        // Register callback for remote audio frames
        let audio_event_tx = self.event_tx.clone();
        let call_id_for_audio = call_id.clone();
        peer.on_remote_audio_frame(move |pcm_data, sample_rate, channels| {
            let _ = audio_event_tx.send(CallEvent::AudioReceived {
                call_id: call_id_for_audio.clone(),
                data: pcm_data,
                sample_rate,
                channels,
            });
        })
        .await?;

        // Register callback for local ICE candidates
        let ice_event_tx = self.event_tx.clone();
        let call_id_for_ice = call_id.clone();
        let remote_peer_for_ice = remote_peer_id.clone();
        peer.on_ice_candidate(move |candidate, sdp_mid, sdp_m_line_index| {
            let _ = ice_event_tx.send(CallEvent::SignalingIceCandidate {
                call_id: call_id_for_ice.clone(),
                to_peer_id: remote_peer_for_ice.clone(),
                candidate,
                sdp_mid,
                sdp_m_line_index,
            });
        })
        .await?;

        // Add audio track (only with voip feature - video-only calls don't need audio)
        #[cfg(feature = "voip")]
        peer.add_audio_track().await?;

        // Set remote offer
        peer.set_remote_description(offer_sdp, "offer").await?;

        // Store call and peer
        {
            let mut calls = self.calls.write().await;
            calls.insert(call_id.clone(), call);
        }
        {
            let mut peers = self.peers.write().await;
            peers.insert(call_id.clone(), Arc::new(RwLock::new(peer)));
        }
        // Aplicar ICE candidates que chegaram antes do peer existir
        self.drain_pending_ice(&call_id).await;

        // Emit incoming call event
        let _ = self.event_tx.send(CallEvent::IncomingCall {
            call_id: call_id.clone(),
            from_peer_id: remote_peer_id,
        });

        tracing::info!("📲 Incoming call: {}", call_id);

        Ok(())
    }

    /// Accept an incoming call
    pub async fn accept_call(&self, call_id: String) -> Result<String> {
        let peers = self.peers.read().await;
        let peer_lock = peers
            .get(&call_id)
            .ok_or_else(|| VoipError::InvalidState("Call not found".to_string()))?;

        let peer = peer_lock.read().await;

        // Create answer
        let answer_sdp = peer.create_answer().await?;

        // Update call state
        {
            let mut calls = self.calls.write().await;
            if let Some(call) = calls.get_mut(&call_id) {
                call.state = CallState::Connecting;
            }
        }

        // Emit state change event
        let _ = self.event_tx.send(CallEvent::StateChanged {
            call_id: call_id.clone(),
            new_state: CallState::Connecting,
        });

        // Get remote peer ID for signaling event
        let remote_peer_id = {
            let calls = self.calls.read().await;
            calls
                .get(&call_id)
                .map(|call| call.remote_peer_id.clone())
                .ok_or_else(|| VoipError::InvalidState("Call not found".to_string()))?
        };

        // Emit signaling answer event (for network layer to send)
        let _ = self.event_tx.send(CallEvent::SignalingAnswer {
            call_id: call_id.clone(),
            to_peer_id: remote_peer_id,
            sdp: answer_sdp.clone(),
        });

        tracing::info!("✅ Accepted call: {}", call_id);

        Ok(answer_sdp)
    }

    /// Handle incoming answer (for outgoing call)
    pub async fn handle_answer(&self, call_id: String, answer_sdp: String) -> Result<()> {
        let peers = self.peers.read().await;
        let peer_lock = peers
            .get(&call_id)
            .ok_or_else(|| VoipError::InvalidState("Call not found".to_string()))?;

        let peer = peer_lock.read().await;
        peer.set_remote_description(answer_sdp, "answer").await?;

        // Update state to connecting
        {
            let mut calls = self.calls.write().await;
            if let Some(call) = calls.get_mut(&call_id) {
                call.state = CallState::Connecting;
            }
        }

        let _ = self.event_tx.send(CallEvent::StateChanged {
            call_id: call_id.clone(),
            new_state: CallState::Connecting,
        });

        tracing::info!("🔗 Call connecting: {}", call_id);

        Ok(())
    }

    /// Add ICE candidate
    ///
    /// Candidatos podem chegar antes do offer criar o peer (race normal de
    /// signaling); nesse caso ficam bufferizados e são aplicados quando o
    /// peer é registrado (drain_pending_ice).
    pub async fn add_ice_candidate(
        &self,
        call_id: String,
        candidate: String,
        sdp_mid: Option<String>,
        sdp_m_line_index: Option<u16>,
    ) -> Result<()> {
        let peer_lock = {
            let peers = self.peers.read().await;
            peers.get(&call_id).cloned()
        };

        let Some(peer_lock) = peer_lock else {
            const MAX_PENDING_ICE: usize = 64;
            let mut pending = self.pending_ice.write().await;
            let entry = pending.entry(call_id.clone()).or_default();
            if entry.len() < MAX_PENDING_ICE {
                entry.push((candidate, sdp_mid, sdp_m_line_index));
                tracing::debug!(
                    "🧊 ICE candidate buffered for call {} ({} pending)",
                    call_id,
                    entry.len()
                );
            } else {
                tracing::warn!("🧊 Pending ICE buffer full for call {}, dropping", call_id);
            }
            return Ok(());
        };

        let peer = peer_lock.read().await;
        peer.add_ice_candidate(candidate, sdp_mid, sdp_m_line_index)
            .await?;

        tracing::debug!("🧊 ICE candidate added for call: {}", call_id);

        Ok(())
    }

    /// Aplica candidatos ICE bufferizados assim que o peer da chamada existe
    async fn drain_pending_ice(&self, call_id: &str) {
        let buffered = {
            let mut pending = self.pending_ice.write().await;
            pending.remove(call_id).unwrap_or_default()
        };
        if buffered.is_empty() {
            return;
        }

        let peer_lock = {
            let peers = self.peers.read().await;
            peers.get(call_id).cloned()
        };
        let Some(peer_lock) = peer_lock else { return };
        let peer = peer_lock.read().await;

        let count = buffered.len();
        for (candidate, sdp_mid, sdp_m_line_index) in buffered {
            if let Err(e) = peer
                .add_ice_candidate(candidate, sdp_mid, sdp_m_line_index)
                .await
            {
                tracing::warn!("🧊 Failed to apply buffered ICE candidate: {}", e);
            }
        }
        tracing::info!("🧊 Applied {} buffered ICE candidates for call {}", count, call_id);
    }

    /// Reject an incoming call
    pub async fn reject_call(&self, call_id: String) -> Result<()> {
        // Close peer connection
        {
            let mut peers = self.peers.write().await;
            if let Some(peer_lock) = peers.remove(&call_id) {
                let peer = peer_lock.read().await;
                let _ = peer.close().await;
            }
        }

        // Remove call
        {
            let mut calls = self.calls.write().await;
            calls.remove(&call_id);
        }

        let _ = self.event_tx.send(CallEvent::Ended {
            call_id: call_id.clone(),
            reason: CallEndReason::Rejected,
        });

        tracing::info!("❌ Call rejected: {}", call_id);

        Ok(())
    }

    /// Hang up an active call
    pub async fn hangup_call(&self, call_id: String) -> Result<()> {
        // Close peer connection
        {
            let mut peers = self.peers.write().await;
            if let Some(peer_lock) = peers.remove(&call_id) {
                let peer = peer_lock.read().await;
                let _ = peer.close().await;
            }
        }

        // Update call state
        {
            let mut calls = self.calls.write().await;
            calls.remove(&call_id);
        }

        let _ = self.event_tx.send(CallEvent::Ended {
            call_id: call_id.clone(),
            reason: CallEndReason::Hangup,
        });

        tracing::info!("📴 Call ended: {}", call_id);

        Ok(())
    }

    /// Enable video for an active call
    pub async fn enable_video(&self, call_id: &str, codec: VideoCodec) -> Result<()> {
        // A packetization VP9 ainda não é conforme com a RFC (delega a VP8);
        // restringir a negociação a H.264/VP8 até implementar corretamente.
        let codec = if codec == VideoCodec::VP9 {
            tracing::warn!("📹 VP9 not fully supported yet; falling back to VP8");
            VideoCodec::VP8
        } else {
            codec
        };

        let peers = self.peers.read().await;
        let peer_lock = peers
            .get(call_id)
            .ok_or_else(|| VoipError::InvalidState("Call not found".to_string()))?;

        // Get mutable access to add video track
        let mut peer = peer_lock.write().await;
        peer.add_video_track(codec).await?;

        // Update video enabled state
        {
            let mut video_enabled = self.video_enabled.write().await;
            video_enabled.insert(call_id.to_string(), true);
        }
        {
            let mut calls = self.calls.write().await;
            if let Some(call) = calls.get_mut(call_id) {
                call.video_enabled = true;
                call.video_codec = Some(codec);
            }
        }

        // Emit event
        let _ = self.event_tx.send(CallEvent::VideoEnabled {
            call_id: call_id.to_string(),
            codec,
        });

        tracing::info!("📹 Video enabled for call: {} (codec: {:?})", call_id, codec);

        Ok(())
    }

    /// Disable video for an active call
    pub async fn disable_video(&self, call_id: &str) -> Result<()> {
        let peers = self.peers.read().await;
        let peer_lock = peers
            .get(call_id)
            .ok_or_else(|| VoipError::InvalidState("Call not found".to_string()))?;

        // Get mutable access to remove video track
        let mut peer = peer_lock.write().await;
        peer.remove_video_track().await?;

        // Update video enabled state
        {
            let mut video_enabled = self.video_enabled.write().await;
            video_enabled.insert(call_id.to_string(), false);
        }
        {
            let mut calls = self.calls.write().await;
            if let Some(call) = calls.get_mut(call_id) {
                call.video_enabled = false;
                call.video_codec = None;
            }
        }

        // Emit event
        let _ = self.event_tx.send(CallEvent::VideoDisabled {
            call_id: call_id.to_string(),
        });

        tracing::info!("🚫 Video disabled for call: {}", call_id);

        Ok(())
    }

    /// Check if video is enabled for a call
    pub async fn is_video_enabled(&self, call_id: &str) -> bool {
        let video_enabled = self.video_enabled.read().await;
        video_enabled.get(call_id).copied().unwrap_or(false)
    }

    /// Send video frame to remote peer
    pub async fn send_video_frame(&self, call_id: &str, frame_data: &[u8]) -> Result<()> {
        let peers = self.peers.read().await;
        let peer_lock = peers
            .get(call_id)
            .ok_or_else(|| VoipError::InvalidState("Call not found".to_string()))?;

        let peer = peer_lock.read().await;
        peer.send_video_frame(frame_data).await?;

        Ok(())
    }

    /// Send raw PCM audio frame to remote peer (Opus encoded in core)
    #[cfg(feature = "voip")]
    pub async fn send_audio_frame(
        &self,
        call_id: &str,
        pcm_data: &[u8],
        sample_rate: u32,
        channels: u32,
    ) -> Result<()> {
        if sample_rate != 48_000 {
            return Err(VoipError::CodecError(
                "Unsupported sample rate (expected 48000)".to_string(),
            ));
        }
        if channels != 1 {
            return Err(VoipError::CodecError(
                "Unsupported channel count (expected mono)".to_string(),
            ));
        }
        if pcm_data.len() % 2 != 0 {
            return Err(VoipError::CodecError(
                "PCM data must be 16-bit aligned".to_string(),
            ));
        }

        let peers = self.peers.read().await;
        let peer_lock = peers
            .get(call_id)
            .ok_or_else(|| VoipError::InvalidState("Call not found".to_string()))?;
        let peer = peer_lock.read().await;
        let audio_track = peer
            .audio_track()
            .ok_or_else(|| VoipError::CallSetupFailed("No audio track available".to_string()))?;

        let encoder = {
            let mut encoders = self.audio_encoders.write().await;
            match encoders.entry(call_id.to_string()) {
                std::collections::hash_map::Entry::Occupied(entry) => entry.get().clone(),
                std::collections::hash_map::Entry::Vacant(entry) => {
                    let config = OpusConfig::default();
                    let encoder = OpusEncoder::new(config).map_err(|e| {
                        VoipError::CallSetupFailed(format!("Failed to create Opus encoder: {}", e))
                    })?;
                    entry.insert(Arc::new(Mutex::new(encoder))).clone()
                }
            }
        };

        let mut samples = Vec::with_capacity(pcm_data.len() / 2);
        for chunk in pcm_data.chunks_exact(2) {
            let value = i16::from_le_bytes([chunk[0], chunk[1]]);
            samples.push(value as f32 / 32768.0);
        }

        let mut encoder = encoder.lock().await;
        if let Some(packet) = encoder.encode(&samples)? {
            audio_track
                .write(&packet)
                .await
                .map_err(|e| VoipError::NetworkError(format!("Failed to write RTP: {}", e)))?;
        }

        while encoder.buffered_samples() >= encoder.frame_size() {
            if let Some(packet) = encoder.encode(&[])? {
                audio_track
                    .write(&packet)
                    .await
                    .map_err(|e| VoipError::NetworkError(format!("Failed to write RTP: {}", e)))?;
            } else {
                break;
            }
        }

        Ok(())
    }

    /// Switch camera (front/back) during video call
    ///
    /// This method doesn't directly control the camera - it emits an event
    /// that the platform-specific camera manager should handle.
    /// The actual camera switching is done by:
    /// - Android: CameraManager.switchCamera()
    /// - iOS: CameraManager.switchCamera()
    pub async fn switch_camera(&self, call_id: &str) -> Result<()> {
        // Verify call exists
        let calls = self.calls.read().await;
        calls
            .get(call_id)
            .ok_or_else(|| VoipError::InvalidState("Call not found".to_string()))?;

        tracing::info!("📸 Switch camera requested for call: {}", call_id);

        let _ = self.event_tx.send(CallEvent::CameraSwitchRequested {
            call_id: call_id.to_string(),
        });

        Ok(())
    }

    /// Toggle mute for an active call
    pub async fn toggle_mute(&self, call_id: String) -> Result<()> {
        let mut calls = self.calls.write().await;
        let call = calls
            .get_mut(&call_id)
            .ok_or_else(|| VoipError::InvalidState("Call not found".to_string()))?;

        // Toggle mute state
        call.toggle_mute();
        let is_muted = call.audio_muted;

        tracing::info!(
            "🔇 Audio {} for call: {}",
            if is_muted { "muted" } else { "unmuted" },
            call_id
        );

        let _ = self.event_tx.send(CallEvent::MuteToggled {
            call_id: call_id.clone(),
            is_muted,
        });

        Ok(())
    }

    /// Toggle speakerphone for an active call
    pub async fn toggle_speakerphone(&self, call_id: String) -> Result<()> {
        let mut calls = self.calls.write().await;
        let call = calls
            .get_mut(&call_id)
            .ok_or_else(|| VoipError::InvalidState("Call not found".to_string()))?;

        // Toggle speakerphone state
        call.toggle_speakerphone();
        let is_speaker = call.speakerphone;

        tracing::info!(
            "🔊 Speakerphone {} for call: {}",
            if is_speaker { "enabled" } else { "disabled" },
            call_id
        );

        let _ = self.event_tx.send(CallEvent::SpeakerphoneToggled {
            call_id: call_id.clone(),
            enabled: is_speaker,
        });

        Ok(())
    }

    /// Get current active calls
    pub async fn get_active_calls(&self) -> Vec<String> {
        let calls = self.calls.read().await;
        calls.keys().cloned().collect()
    }

    /// Get call state
    pub async fn get_call_state(&self, call_id: &str) -> Option<CallState> {
        let calls = self.calls.read().await;
        calls.get(call_id).map(|call| call.state.clone())
    }

    /// Subscribe to call events
    pub fn subscribe_events(&self) -> broadcast::Receiver<CallEvent> {
        self.event_tx.subscribe()
    }

    // === Compatibility wrappers for VoIPIntegration ===

    /// Handle call answer (alias for handle_answer)
    pub async fn handle_call_answer(&self, call_id: String, answer_sdp: String) -> Result<()> {
        self.handle_answer(call_id, answer_sdp).await
    }

    /// Handle ICE candidate (alias for add_ice_candidate)
    pub async fn handle_ice_candidate(
        &self,
        call_id: String,
        candidate: String,
        sdp_mid: Option<String>,
        sdp_m_line_index: Option<u16>,
    ) -> Result<()> {
        self.add_ice_candidate(call_id, candidate, sdp_mid, sdp_m_line_index)
            .await
    }

    /// End a call with specific reason
    pub async fn end_call(&self, call_id: String, reason: CallEndReason) -> Result<()> {
        // Close peer connection
        {
            let mut peers = self.peers.write().await;
            if let Some(peer_lock) = peers.remove(&call_id) {
                let peer = peer_lock.read().await;
                let _ = peer.close().await;
            }
        }

        // Remove call
        {
            let mut calls = self.calls.write().await;
            calls.remove(&call_id);
        }

        // Emit event with specific reason
        let _ = self.event_tx.send(CallEvent::Ended {
            call_id: call_id.clone(),
            reason: reason.clone(),
        });

        tracing::info!("📴 Call {} ended: {:?}", call_id, reason);

        Ok(())
    }
}

impl Default for CallManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_call_manager_creation() {
        let manager = CallManager::new();
        let active = manager.get_active_calls().await;
        assert_eq!(active.len(), 0);
    }

    #[tokio::test]
    async fn test_turn_credentials() {
        let manager = CallManager::new();

        let creds = TurnCredentials {
            username: "user123".to_string(),
            password: "pass456".to_string(),
            uris: vec!["turn:example.com:3478".to_string()],
        };

        manager.set_turn_credentials(creds).await;

        let ice_servers = manager.get_ice_servers().await;
        assert_eq!(ice_servers.len(), 2); // STUN + TURN
    }
}
