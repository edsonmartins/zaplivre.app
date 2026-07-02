//! Call State Management
//!
//! Manages the lifecycle of a WebRTC call.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

/// Call state machine
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CallState {
    /// Call is being initiated (sending offer)
    Initiating,

    /// Waiting for remote peer to accept/reject
    Ringing,

    /// Remote peer accepted, exchanging ICE candidates
    Connecting,

    /// Call is established and active
    Active,

    /// Call is being terminated
    Ending,

    /// Call has ended
    Ended { reason: CallEndReason },
}

/// Reason why a call ended
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CallEndReason {
    /// Normal hangup by either party
    Hangup,

    /// Call was rejected by recipient
    Rejected,

    /// Call hung up by local user
    LocalHangup,

    /// Call hung up by remote peer
    RemoteHangup,

    /// Call failed to connect (ICE/DTLS failure)
    ConnectionFailed,

    /// Call timed out (no answer)
    Timeout,

    /// Network error during call
    NetworkError,
}

/// Direction of the call
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CallDirection {
    /// Outgoing call (we initiated)
    Outgoing,

    /// Incoming call (remote initiated)
    Incoming,
}

/// Represents an active or historical call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Call {
    /// Unique call identifier
    pub id: String,

    /// Remote peer ID
    pub remote_peer_id: String,

    /// Call direction (incoming/outgoing)
    pub direction: CallDirection,

    /// Current call state
    pub state: CallState,

    /// When the call was initiated
    pub started_at: DateTime<Utc>,

    /// When the call connected (if it did)
    pub connected_at: Option<DateTime<Utc>>,

    /// When the call ended (if it did)
    pub ended_at: Option<DateTime<Utc>>,

    /// Is audio muted
    pub audio_muted: bool,

    /// Is on speakerphone (mobile only)
    pub speakerphone: bool,

    /// Is video enabled for this call
    pub video_enabled: bool,

    /// Negotiated video codec (when video is enabled)
    pub video_codec: Option<crate::voip::video::VideoCodec>,
}

impl Call {
    /// Create a new outgoing call
    pub fn new_outgoing(remote_peer_id: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            remote_peer_id,
            direction: CallDirection::Outgoing,
            state: CallState::Initiating,
            started_at: Utc::now(),
            connected_at: None,
            ended_at: None,
            audio_muted: false,
            speakerphone: false,
            video_enabled: false,
            video_codec: None,
        }
    }

    /// Create a new incoming call
    pub fn new_incoming(call_id: String, remote_peer_id: String) -> Self {
        Self {
            id: call_id,
            remote_peer_id,
            direction: CallDirection::Incoming,
            state: CallState::Ringing,
            started_at: Utc::now(),
            connected_at: None,
            ended_at: None,
            audio_muted: false,
            speakerphone: false,
            video_enabled: false,
            video_codec: None,
        }
    }

    /// Transition to connected state
    pub fn set_connected(&mut self) {
        self.state = CallState::Active;
        self.connected_at = Some(Utc::now());
    }

    /// End the call with a reason
    pub fn end(&mut self, reason: CallEndReason) {
        self.state = CallState::Ended { reason };
        self.ended_at = Some(Utc::now());
    }

    /// Get call duration if active
    pub fn duration(&self) -> Option<Duration> {
        self.connected_at.map(|connected| {
            let end_time = self.ended_at.unwrap_or_else(Utc::now);
            end_time.signed_duration_since(connected)
                .to_std()
                .unwrap_or_default()
        })
    }

    /// Toggle audio mute
    pub fn toggle_mute(&mut self) {
        self.audio_muted = !self.audio_muted;
    }

    /// Toggle speakerphone
    pub fn toggle_speakerphone(&mut self) {
        self.speakerphone = !self.speakerphone;
    }

    /// Check if call is active
    pub fn is_active(&self) -> bool {
        self.state == CallState::Active
    }

    /// Check if call is ended
    pub fn is_ended(&self) -> bool {
        matches!(self.state, CallState::Ended { .. })
    }
}

/// Call statistics (for quality monitoring)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CallStats {
    /// Average round-trip time (ms)
    pub avg_rtt_ms: u32,

    /// Packets sent
    pub packets_sent: u64,

    /// Packets received
    pub packets_received: u64,

    /// Packets lost
    pub packets_lost: u64,

    /// Jitter (ms)
    pub jitter_ms: u32,

    /// Audio bitrate (kbps)
    pub audio_bitrate_kbps: u32,
}

impl CallStats {
    /// Calculate packet loss rate (0.0 to 1.0)
    pub fn packet_loss_rate(&self) -> f64 {
        if self.packets_sent == 0 {
            return 0.0;
        }
        self.packets_lost as f64 / self.packets_sent as f64
    }

    /// Check if call quality is good
    pub fn is_quality_good(&self) -> bool {
        self.packet_loss_rate() < 0.05 // < 5% packet loss
            && self.avg_rtt_ms < 300 // < 300ms RTT
            && self.jitter_ms < 50 // < 50ms jitter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outgoing_call_lifecycle() {
        let mut call = Call::new_outgoing("peer123".to_string());

        assert_eq!(call.direction, CallDirection::Outgoing);
        assert_eq!(call.state, CallState::Initiating);
        assert!(!call.is_ended());

        call.set_connected();
        assert_eq!(call.state, CallState::Active);
        assert!(call.is_active());

        call.end(CallEndReason::Hangup);
        assert!(call.is_ended());
        assert_eq!(call.state, CallState::Ended { reason: CallEndReason::Hangup });
    }

    #[test]
    fn test_incoming_call() {
        let call = Call::new_incoming("call123".to_string(), "peer456".to_string());

        assert_eq!(call.direction, CallDirection::Incoming);
        assert_eq!(call.state, CallState::Ringing);
        assert_eq!(call.id, "call123");
    }

    #[test]
    fn test_call_duration() {
        let mut call = Call::new_outgoing("peer789".to_string());

        assert!(call.duration().is_none());

        call.set_connected();
        std::thread::sleep(std::time::Duration::from_millis(100));

        let duration = call.duration().unwrap();
        assert!(duration.as_millis() >= 100);
    }

    #[test]
    fn test_mute_toggle() {
        let mut call = Call::new_outgoing("peer999".to_string());

        assert!(!call.audio_muted);
        call.toggle_mute();
        assert!(call.audio_muted);
        call.toggle_mute();
        assert!(!call.audio_muted);
    }

    #[test]
    fn test_call_stats_quality() {
        let mut stats = CallStats {
            avg_rtt_ms: 50,
            packets_sent: 1000,
            packets_received: 995,
            packets_lost: 5,
            jitter_ms: 20,
            audio_bitrate_kbps: 64,
        };

        assert_eq!(stats.packet_loss_rate(), 0.005);
        assert!(stats.is_quality_good());

        // Bad quality scenario
        stats.packets_lost = 100;
        stats.avg_rtt_ms = 500;
        assert!(!stats.is_quality_good());
    }
}
