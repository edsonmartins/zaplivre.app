//! WebRTC Peer Connection Management
//!
//! Handles WebRTC peer connections, media tracks (audio + video), and ICE/DTLS.

use super::video::VideoCodec;
use super::VoipError;
use crate::voip::Result;
use interceptor::registry::Registry;
use std::sync::Arc;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::setting_engine::SettingEngine;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::{TrackLocal, TrackLocalWriter};

/// WebRTC peer connection wrapper
pub struct WebRTCPeer {
    peer_connection: Arc<RTCPeerConnection>,
    audio_track: Option<Arc<TrackLocalStaticRTP>>,
    video_track: Option<Arc<TrackLocalStaticRTP>>,
}

impl WebRTCPeer {
    /// Create a new WebRTC peer with STUN/TURN configuration
    pub async fn new(ice_servers: Vec<RTCIceServer>) -> Result<Self> {
        // Create a MediaEngine for audio
        let mut media_engine = MediaEngine::default();

        // Register Opus codec (standard for WebRTC audio)
        media_engine
            .register_default_codecs()
            .map_err(|e| VoipError::WebRtcError(format!("Failed to register codecs: {}", e)))?;

        // Create SettingEngine
        let mut setting_engine = SettingEngine::default();

        // Enable detach for data channel
        setting_engine.detach_data_channels();

        // Create API with MediaEngine and SettingEngine
        let mut interceptor_registry = Registry::new();

        // Register default interceptors (RTCP, NACK, etc.)
        interceptor_registry = register_default_interceptors(interceptor_registry, &mut media_engine)
            .map_err(|e| VoipError::WebRtcError(format!("Failed to register interceptors: {}", e)))?;

        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .with_setting_engine(setting_engine)
            .with_interceptor_registry(interceptor_registry)
            .build();

        // Create peer connection configuration with ICE servers
        let config = RTCConfiguration {
            ice_servers,
            ..Default::default()
        };

        // Create the peer connection
        let peer_connection = Arc::new(
            api.new_peer_connection(config)
                .await
                .map_err(|e| VoipError::WebRtcError(format!("Failed to create peer connection: {}", e)))?,
        );

        Ok(Self {
            peer_connection,
            audio_track: None,
            video_track: None,
        })
    }

    /// Register callback for remote video frames
    ///
    /// This should be called after creating the peer but before starting the call
    pub async fn on_remote_video_frame<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(Vec<u8>, u32, u32) + Send + Sync + 'static,
    {
        let pc = Arc::clone(&self.peer_connection);
        let callback = Arc::new(callback);

        // Register handler for when remote track is added
        pc.on_track(Box::new(move |track, _receiver, _transceiver| {
            let callback = Arc::clone(&callback);

            // Check if this is a video track
            if track.kind() == webrtc::rtp_transceiver::rtp_codec::RTPCodecType::Video {
                tracing::info!("📹 Remote video track received");

                Box::pin(async move {
                    // Read RTP packets from the track
                    while let Ok((rtp_packet, _)) = track.read_rtp().await {
                        // Extract payload (video frame data)
                        let payload = rtp_packet.payload.to_vec();

                        // For now, we'll pass the raw RTP payload
                        // In production, this should be decoded first
                        // Assuming VGA resolution for now (should be extracted from SDP)
                        callback(payload, 640, 480);
                    }
                })
            } else {
                Box::pin(async {})
            }
        }));

        Ok(())
    }

    /// Register callback for local ICE candidates
    pub async fn on_ice_candidate<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(String, Option<String>, Option<u16>) + Send + Sync + 'static,
    {
        let pc = Arc::clone(&self.peer_connection);
        let callback = Arc::new(callback);

        pc.on_ice_candidate(Box::new(move |candidate| {
            let callback = Arc::clone(&callback);
            Box::pin(async move {
                if let Some(candidate) = candidate {
                    if let Ok(json) = candidate.to_json() {
                        callback(json.candidate, json.sdp_mid, json.sdp_mline_index);
                    }
                }
            })
        }));

        Ok(())
    }

    /// Add audio track to the peer connection
    pub async fn add_audio_track(&mut self) -> Result<()> {
        // Create an audio track (Opus codec)
        let audio_track = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability {
                mime_type: "audio/opus".to_owned(),
                clock_rate: 48000,
                channels: 2,
                sdp_fmtp_line: "".to_owned(),
                rtcp_feedback: vec![],
            },
            "audio".to_owned(),
            "mepassa-audio".to_owned(),
        ));

        // Add track to peer connection
        let _rtp_sender = self
            .peer_connection
            .add_track(Arc::clone(&audio_track) as Arc<dyn TrackLocal + Send + Sync>)
            .await
            .map_err(|e| VoipError::WebRtcError(format!("Failed to add audio track: {}", e)))?;

        self.audio_track = Some(audio_track);

        tracing::info!("✅ Audio track added to peer connection");
        Ok(())
    }

    /// Add video track to the peer connection
    pub async fn add_video_track(&mut self, codec: VideoCodec) -> Result<()> {
        // Create a video track with specified codec
        let video_track = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability {
                mime_type: codec.mime_type().to_owned(),
                clock_rate: codec.clock_rate(),
                channels: 0, // Video has no channels
                sdp_fmtp_line: codec.fmtp_line(),
                rtcp_feedback: vec![],
            },
            "video".to_owned(),
            "mepassa-video".to_owned(),
        ));

        // Add track to peer connection
        let _rtp_sender = self
            .peer_connection
            .add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>)
            .await
            .map_err(|e| VoipError::WebRtcError(format!("Failed to add video track: {}", e)))?;

        self.video_track = Some(video_track);

        tracing::info!("✅ Video track added to peer connection - codec: {:?}", codec);
        Ok(())
    }

    /// Send video frame to remote peer
    ///
    /// Frame data should be pre-encoded (H.264 NALUs or VP8 frames)
    pub async fn send_video_frame(&self, frame: &[u8]) -> Result<()> {
        if let Some(video_track) = &self.video_track {
            // For MVP, we create a simple RTP packet with the frame data
            // In production, this should be properly packetized
            use webrtc::rtp::packet::Packet;

            let packet = Packet {
                header: webrtc::rtp::header::Header {
                    version: 2,
                    padding: false,
                    extension: false,
                    marker: true,
                    payload_type: 96, // Dynamic payload type for H.264
                    sequence_number: 0, // TODO: Maintain sequence counter
                    timestamp: 0, // TODO: Calculate proper timestamp
                    ssrc: 0, // TODO: Use proper SSRC
                    ..Default::default()
                },
                payload: frame.to_vec().into(),
            };

            video_track
                .write_rtp(&packet)
                .await
                .map_err(|e| VoipError::WebRtcError(format!("Failed to write video frame: {}", e)))?;

            Ok(())
        } else {
            Err(VoipError::InvalidState(
                "Video track not added yet".to_string(),
            ))
        }
    }

    /// Remove video track (disable camera)
    ///
    /// Triggers renegotiation to inform remote peer
    pub async fn remove_video_track(&mut self) -> Result<()> {
        if self.video_track.is_none() {
            return Ok(()); // Already removed
        }

        self.video_track = None;

        // Trigger renegotiation by creating new offer
        let _ = self.create_offer().await?;

        tracing::info!("🚫 Video track removed from peer connection");
        Ok(())
    }

    /// Check if video track is enabled
    pub fn has_video(&self) -> bool {
        self.video_track.is_some()
    }

    /// Create SDP offer
    pub async fn create_offer(&self) -> Result<String> {
        let offer = self
            .peer_connection
            .create_offer(None)
            .await
            .map_err(|e| VoipError::WebRtcError(format!("Failed to create offer: {}", e)))?;

        // Set local description
        self.peer_connection
            .set_local_description(offer.clone())
            .await
            .map_err(|e| VoipError::WebRtcError(format!("Failed to set local description: {}", e)))?;

        Ok(offer.sdp)
    }

    /// Create SDP answer
    pub async fn create_answer(&self) -> Result<String> {
        let answer = self
            .peer_connection
            .create_answer(None)
            .await
            .map_err(|e| VoipError::WebRtcError(format!("Failed to create answer: {}", e)))?;

        // Set local description
        self.peer_connection
            .set_local_description(answer.clone())
            .await
            .map_err(|e| VoipError::WebRtcError(format!("Failed to set local description: {}", e)))?;

        Ok(answer.sdp)
    }

    /// Set remote SDP description (offer or answer)
    pub async fn set_remote_description(&self, sdp: String, sdp_type: &str) -> Result<()> {
        let remote_desc = match sdp_type {
            "offer" => RTCSessionDescription::offer(sdp)
                .map_err(|e| VoipError::WebRtcError(format!("Invalid SDP offer: {}", e)))?,
            "answer" => RTCSessionDescription::answer(sdp)
                .map_err(|e| VoipError::WebRtcError(format!("Invalid SDP answer: {}", e)))?,
            _ => {
                return Err(VoipError::WebRtcError(format!(
                    "Invalid SDP type: {}",
                    sdp_type
                )))
            }
        };

        self.peer_connection
            .set_remote_description(remote_desc)
            .await
            .map_err(|e| VoipError::WebRtcError(format!("Failed to set remote description: {}", e)))?;

        Ok(())
    }

    /// Add remote ICE candidate
    pub async fn add_ice_candidate(
        &self,
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u16>,
    ) -> Result<()> {
        let ice_candidate = RTCIceCandidateInit {
            candidate,
            sdp_mid,
            sdp_mline_index,
            ..Default::default()
        };

        self.peer_connection
            .add_ice_candidate(ice_candidate)
            .await
            .map_err(|e| VoipError::WebRtcError(format!("Failed to add ICE candidate: {}", e)))?;

        Ok(())
    }

    /// Get current connection state
    pub fn connection_state(&self) -> RTCPeerConnectionState {
        self.peer_connection.connection_state()
    }

    /// Close the peer connection
    pub async fn close(&self) -> Result<()> {
        self.peer_connection
            .close()
            .await
            .map_err(|e| VoipError::WebRtcError(format!("Failed to close connection: {}", e)))?;

        tracing::info!("🔌 WebRTC peer connection closed");
        Ok(())
    }

    /// Get reference to the peer connection for event handlers
    pub fn peer_connection(&self) -> Arc<RTCPeerConnection> {
        Arc::clone(&self.peer_connection)
    }

    /// Get audio track for sending audio data
    pub fn audio_track(&self) -> Option<Arc<TrackLocalStaticRTP>> {
        self.audio_track.clone()
    }
}

/// Build TURN server configuration from credentials
pub fn build_turn_config(
    turn_uris: Vec<String>,
    username: String,
    credential: String,
) -> Vec<RTCIceServer> {
    vec![
        // STUN server (public Google STUN)
        RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        },
        // TURN server (from FASE 10)
        RTCIceServer {
            urls: turn_uris,
            username,
            credential,
            credential_type: webrtc::ice_transport::ice_credential_type::RTCIceCredentialType::Password,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_peer_connection() {
        // Use public STUN server for testing
        let ice_servers = vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }];

        let peer = WebRTCPeer::new(ice_servers).await;
        assert!(peer.is_ok());
    }

    #[tokio::test]
    async fn test_add_audio_track() {
        let ice_servers = vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }];

        let mut peer = WebRTCPeer::new(ice_servers).await.unwrap();
        let result = peer.add_audio_track().await;

        assert!(result.is_ok());
        assert!(peer.audio_track().is_some());
    }

    #[test]
    fn test_build_turn_config() {
        let config = build_turn_config(
            vec!["turn:turn.example.com:3478".to_string()],
            "user123".to_string(),
            "pass456".to_string(),
        );

        assert_eq!(config.len(), 2); // STUN + TURN
        assert!(config[0].urls[0].starts_with("stun:"));
        assert!(config[1].urls[0].starts_with("turn:"));
    }
}
