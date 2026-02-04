//! VoIP Network Integration
//!
//! Coordinates VoIP signaling between NetworkManager and CallManager.
//! Bridges libp2p network layer with WebRTC call management.

use libp2p::PeerId;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};

use super::{
    call::{CallDirection, CallEndReason},
    manager::{CallEvent, CallManager},
    signaling::SignalingMessage,
    Result, // Use voip::Result instead of utils::error::Result
};
use crate::network::swarm::NetworkManager;
use crate::utils::error::MePassaError;

/// VoIP network integration coordinator
///
/// Manages the flow of WebRTC signaling messages between:
/// - NetworkManager (libp2p P2P layer)
/// - CallManager (WebRTC call management)
pub struct VoIPIntegration {
    network_manager: Arc<RwLock<NetworkManager>>,
    call_manager: Arc<CallManager>,

    // Event channels
    signaling_rx: Mutex<Option<mpsc::UnboundedReceiver<(PeerId, SignalingMessage)>>>,
    signaling_tx: mpsc::UnboundedSender<(PeerId, SignalingMessage)>,

    // Call events from CallManager
    call_event_rx: Mutex<Option<broadcast::Receiver<CallEvent>>>,

    // Video frame callback (FASE 14)
    #[cfg(any(feature = "voip", feature = "video"))]
    video_frame_callback: Arc<RwLock<Option<Box<dyn crate::FfiVideoFrameCallback>>>>,

    // VoIP control events callback (mute/speaker/camera)
    voip_event_callback: Arc<RwLock<Option<Box<dyn crate::FfiVoipEventCallback>>>>,
}

impl VoIPIntegration {
    /// Create a new VoIP integration coordinator
    pub async fn new(
        network_manager: Arc<RwLock<NetworkManager>>,
        call_manager: Arc<CallManager>,
    ) -> Self {
        let (signaling_tx, signaling_rx) = mpsc::unbounded_channel();

        // Subscribe to call manager events
        let call_event_rx = call_manager.subscribe_events();

        Self {
            network_manager,
            call_manager,
            signaling_rx: Mutex::new(Some(signaling_rx)),
            signaling_tx,
            call_event_rx: Mutex::new(Some(call_event_rx)),
            #[cfg(any(feature = "voip", feature = "video"))]
            video_frame_callback: Arc::new(RwLock::new(None)),
            voip_event_callback: Arc::new(RwLock::new(None)),
        }
    }

    /// Get a sender for signaling messages (for NetworkManager to use)
    pub fn signaling_sender(&self) -> mpsc::UnboundedSender<(PeerId, SignalingMessage)> {
        self.signaling_tx.clone()
    }

    /// Register callback for receiving remote video frames (FASE 14)
    ///
    /// The callback will be invoked whenever a VideoFrameReceived event is
    /// received from the CallManager.
    #[cfg(any(feature = "voip", feature = "video"))]
    pub async fn register_video_frame_callback(
        &self,
        callback: Box<dyn crate::FfiVideoFrameCallback>,
    ) {
        let mut cb = self.video_frame_callback.write().await;
        *cb = Some(callback);
        tracing::info!("📹 Video frame callback registered");
    }

    /// Register callback for VoIP control events (mute/speaker/camera)
    pub async fn register_voip_event_callback(
        &self,
        callback: Box<dyn crate::FfiVoipEventCallback>,
    ) {
        let mut cb = self.voip_event_callback.write().await;
        *cb = Some(callback);
        tracing::info!("🎛️ VoIP event callback registered");
    }

    /// Run the integration event loop
    ///
    /// Processes:
    /// - Incoming signaling messages from network
    /// - Outgoing signaling messages from CallManager
    /// - Call state changes and events
    pub async fn spawn(self: Arc<Self>) {
        let mut signaling_rx = match self.signaling_rx.lock().await.take() {
            Some(rx) => rx,
            None => {
                tracing::warn!("VoIP integration already started (signaling_rx missing)");
                return;
            }
        };

        let mut call_event_rx = match self.call_event_rx.lock().await.take() {
            Some(rx) => rx,
            None => {
                tracing::warn!("VoIP integration already started (call_event_rx missing)");
                return;
            }
        };

        let this = Arc::clone(&self);
        tokio::spawn(async move {
            if let Err(e) = this.run_with_receivers(&mut signaling_rx, &mut call_event_rx).await {
                tracing::error!("❌ VoIP integration stopped: {}", e);
            }
        });
    }

    async fn run_with_receivers(
        &self,
        signaling_rx: &mut mpsc::UnboundedReceiver<(PeerId, SignalingMessage)>,
        call_event_rx: &mut broadcast::Receiver<CallEvent>,
    ) -> Result<()> {
        tracing::info!("🔗 VoIP integration started");

        loop {
            tokio::select! {
                // Handle incoming signaling from network
                Some((peer_id, signal)) = signaling_rx.recv() => {
                    if let Err(e) = self.handle_incoming_signal(peer_id, signal).await {
                        tracing::error!("❌ Failed to handle incoming signal: {}", e);
                    }
                }

                // Handle call events from CallManager
                result = call_event_rx.recv() => {
                    match result {
                        Ok(event) => {
                            if let Err(e) = self.handle_call_event(event).await {
                                tracing::error!("❌ Failed to handle call event: {}", e);
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            tracing::warn!("📞 Call event channel lagged, skipping events");
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::warn!("📞 Call event channel closed");
                            break;
                        }
                    }
                }

                else => {
                    tracing::warn!("⚠️ All channels closed");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle incoming signaling message from network
    async fn handle_incoming_signal(
        &self,
        peer_id: PeerId,
        signal: SignalingMessage,
    ) -> Result<()> {
        let peer_id_str = peer_id.to_string();

        tracing::info!("📞 Handling signal from {}: {:?}", peer_id_str, signal);

        match &signal {
            SignalingMessage::CallOffer { call_id, sdp } => {
                // Incoming call offer
                tracing::info!("📲 Incoming call offer from {} (call: {})", peer_id_str, call_id);

                // Create incoming call (call_id, remote_peer_id, offer_sdp)
                self.call_manager
                    .handle_incoming_call(call_id.clone(), peer_id_str.clone(), sdp.clone())
                    .await?;

                tracing::info!("✅ Incoming call created: {}", call_id);
            }

            SignalingMessage::CallAnswer { call_id, sdp } => {
                // Remote peer answered our call
                tracing::info!("✅ Call answered by {} (call: {})", peer_id_str, call_id);

                self.call_manager
                    .handle_call_answer(call_id.clone(), sdp.clone())
                    .await?;
            }

            SignalingMessage::IceCandidate {
                call_id,
                candidate,
                sdp_mid,
                sdp_m_line_index,
            } => {
                // ICE candidate from remote peer
                tracing::debug!(
                    "🧊 ICE candidate from {} for call {}: {}",
                    peer_id_str,
                    call_id,
                    candidate
                );

                self.call_manager
                    .handle_ice_candidate(
                        call_id.clone(),
                        candidate.clone(),
                        sdp_mid.clone(),
                        *sdp_m_line_index,
                    )
                    .await?;
            }

            SignalingMessage::CallReject { call_id, reason } => {
                // Remote peer rejected our call
                tracing::warn!(
                    "❌ Call rejected by {} (call: {}): {:?}",
                    peer_id_str,
                    call_id,
                    reason
                );

                self.call_manager
                    .end_call(call_id.clone(), CallEndReason::Rejected)
                    .await?;
            }

            SignalingMessage::CallHangup { call_id } => {
                // Remote peer hung up
                tracing::info!("📴 Call hung up by {} (call: {})", peer_id_str, call_id);

                self.call_manager
                    .end_call(call_id.clone(), CallEndReason::RemoteHangup)
                    .await?;
            }

            SignalingMessage::CallAccept { call_id } => {
                // Remote peer accepted call (acknowledgment)
                tracing::info!("✅ Call accepted by {} (call: {})", peer_id_str, call_id);
            }
        }

        Ok(())
    }

    /// Handle call events from CallManager
    async fn handle_call_event(&self, event: CallEvent) -> Result<()> {
        match event {
            CallEvent::SignalingOffer {
                call_id,
                to_peer_id,
                sdp,
            } => {
                tracing::info!("📤 Sending offer to {} (call: {})", to_peer_id, call_id);

                let peer_id = to_peer_id
                    .parse::<PeerId>()
                    .map_err(|e| super::VoipError::InvalidState(format!("Invalid peer ID: {}", e)))?;

                let signal = SignalingMessage::CallOffer {
                    call_id,
                    sdp,
                };

                self.send_signal(peer_id, signal).await?;
            }

            CallEvent::SignalingAnswer {
                call_id,
                to_peer_id,
                sdp,
            } => {
                tracing::info!("📤 Sending answer to {} (call: {})", to_peer_id, call_id);

                let peer_id = to_peer_id
                    .parse::<PeerId>()
                    .map_err(|e| super::VoipError::InvalidState(format!("Invalid peer ID: {}", e)))?;

                let signal = SignalingMessage::CallAnswer {
                    call_id,
                    sdp,
                };

                self.send_signal(peer_id, signal).await?;
            }

            CallEvent::SignalingIceCandidate {
                call_id,
                to_peer_id,
                candidate,
                sdp_mid,
                sdp_m_line_index,
            } => {
                tracing::debug!("📤 Sending ICE candidate to {} (call: {})", to_peer_id, call_id);

                let peer_id = to_peer_id
                    .parse::<PeerId>()
                    .map_err(|e| super::VoipError::InvalidState(format!("Invalid peer ID: {}", e)))?;

                let signal = SignalingMessage::IceCandidate {
                    call_id,
                    candidate,
                    sdp_mid,
                    sdp_m_line_index,
                };

                self.send_signal(peer_id, signal).await?;
            }

            CallEvent::Ended { call_id, reason } => {
                tracing::info!("📴 Call ended: {} ({:?})", call_id, reason);
                // Hangup signal should be sent explicitly via hangup_call()
                // This event is just for logging/cleanup
            }

            CallEvent::StateChanged { call_id, new_state } => {
                tracing::debug!("🔄 Call {} state changed to: {:?}", call_id, new_state);
                // Just log state changes
            }

            CallEvent::IncomingCall { call_id, from_peer_id } => {
                tracing::info!("📲 Incoming call: {} from {}", call_id, from_peer_id);
                // Already handled via network signals
            }

            CallEvent::AudioReceived { .. } => {
                // Audio data handled separately by audio pipeline
            }

            CallEvent::VideoFrameReceived { call_id, frame_data, width, height } => {
                // Invoke the registered video frame callback (FASE 14)
                #[cfg(any(feature = "voip", feature = "video"))]
                {
                    let cb = self.video_frame_callback.read().await;
                    if let Some(callback) = cb.as_ref() {
                        callback.on_video_frame(call_id, frame_data, width, height);
                    }
                }
            }

            CallEvent::VideoEnabled { call_id, codec } => {
                tracing::info!("📹 Video enabled for call: {} ({:?})", call_id, codec);
            }

            CallEvent::VideoDisabled { call_id } => {
                tracing::info!("🚫 Video disabled for call: {}", call_id);
            }

            CallEvent::MuteToggled { call_id, is_muted } => {
                let cb = self.voip_event_callback.read().await;
                if let Some(callback) = cb.as_ref() {
                    callback.on_mute_changed(call_id, is_muted);
                }
            }

            CallEvent::SpeakerphoneToggled { call_id, enabled } => {
                let cb = self.voip_event_callback.read().await;
                if let Some(callback) = cb.as_ref() {
                    callback.on_speakerphone_changed(call_id, enabled);
                }
            }

            CallEvent::CameraSwitchRequested { call_id } => {
                let cb = self.voip_event_callback.read().await;
                if let Some(callback) = cb.as_ref() {
                    callback.on_camera_switch_requested(call_id);
                }
            }
        }

        Ok(())
    }

    /// Send signaling message via network
    pub async fn send_signal(&self, peer_id: PeerId, signal: SignalingMessage) -> Result<()> {
        let mut network = self.network_manager.write().await;
        network
            .send_voip_signal(peer_id, signal)
            .map_err(|e| super::VoipError::NetworkError(e.to_string()))
    }

    /// Initiate a call to a peer
    pub async fn start_call(&self, to_peer_id: String) -> Result<String> {
        tracing::info!("📞 Starting call to {}", to_peer_id);

        // Start call via CallManager
        let call_id = self.call_manager.start_call(to_peer_id.clone()).await?;

        // CallManager will generate offer and emit it as event
        // Integration will send it via network when ready

        Ok(call_id)
    }

    /// Accept an incoming call
    pub async fn accept_call(&self, call_id: String) -> Result<()> {
        tracing::info!("✅ Accepting call {}", call_id);

        self.call_manager.accept_call(call_id).await?;

        // CallManager will generate answer and we'll send it via network

        Ok(())
    }

    /// Reject an incoming call
    pub async fn reject_call(&self, call_id: String, reason: Option<String>) -> Result<()> {
        tracing::info!("❌ Rejecting call {}: {:?}", call_id, reason);

        // Get remote peer ID before ending call
        let remote_peer_id = {
            let calls = self.call_manager.calls.read().await;
            calls
                .get(&call_id)
                .map(|call| call.remote_peer_id.clone())
                .ok_or_else(|| super::VoipError::InvalidState("Call not found".to_string()))?
        };

        // End call
        self.call_manager
            .end_call(call_id.clone(), CallEndReason::Rejected)
            .await?;

        // Send rejection signal to remote peer
        let peer_id = remote_peer_id
            .parse::<PeerId>()
            .map_err(|e| super::VoipError::InvalidState(format!("Invalid peer ID: {}", e)))?;

        let signal = SignalingMessage::CallReject {
            call_id,
            reason,
        };

        self.send_signal(peer_id, signal).await?;

        Ok(())
    }

    /// Hangup an active call
    pub async fn hangup_call(&self, call_id: String) -> Result<()> {
        tracing::info!("📴 Hanging up call {}", call_id);

        // Get remote peer ID before ending call
        let remote_peer_id = {
            let calls = self.call_manager.calls.read().await;
            calls
                .get(&call_id)
                .map(|call| call.remote_peer_id.clone())
                .ok_or_else(|| super::VoipError::InvalidState("Call not found".to_string()))?
        };

        // End call
        self.call_manager
            .end_call(call_id.clone(), CallEndReason::LocalHangup)
            .await?;

        // Send hangup signal to remote peer
        let peer_id = remote_peer_id
            .parse::<PeerId>()
            .map_err(|e| super::VoipError::InvalidState(format!("Invalid peer ID: {}", e)))?;

        let signal = SignalingMessage::CallHangup {
            call_id,
        };

        self.send_signal(peer_id, signal).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires full NetworkManager + CallManager setup
    async fn test_integration_creation() {
        // This test would require proper initialization
        // of both NetworkManager and CallManager
        // which is complex in unit test environment
    }
}
