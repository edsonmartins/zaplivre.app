//! WebRTC Signaling Protocol
//!
//! Implements signaling for WebRTC calls over libp2p.
//! Messages include SDP offer/answer and ICE candidates.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Call signaling message types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignalingMessage {
    /// Initiate a call with SDP offer
    CallOffer { call_id: String, sdp: String },

    /// Answer a call with SDP answer
    CallAnswer { call_id: String, sdp: String },

    /// Send ICE candidate for connection establishment
    IceCandidate {
        call_id: String,
        candidate: String,
        sdp_mid: Option<String>,
        sdp_m_line_index: Option<u16>,
    },

    /// Notify call rejection
    CallReject {
        call_id: String,
        reason: Option<String>,
    },

    /// Notify call hangup
    CallHangup { call_id: String },

    /// Notify call accepted (before sending answer)
    CallAccept { call_id: String },
}

impl SignalingMessage {
    /// Get the call ID from any signaling message
    pub fn call_id(&self) -> &str {
        match self {
            Self::CallOffer { call_id, .. }
            | Self::CallAnswer { call_id, .. }
            | Self::IceCandidate { call_id, .. }
            | Self::CallReject { call_id, .. }
            | Self::CallHangup { call_id, .. }
            | Self::CallAccept { call_id, .. } => call_id,
        }
    }

    /// Check if this is an offer message
    pub fn is_offer(&self) -> bool {
        matches!(self, Self::CallOffer { .. })
    }

    /// Check if this is an answer message
    pub fn is_answer(&self) -> bool {
        matches!(self, Self::CallAnswer { .. })
    }

    /// Check if this is an ICE candidate message
    pub fn is_ice_candidate(&self) -> bool {
        matches!(self, Self::IceCandidate { .. })
    }
}

impl fmt::Display for SignalingMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CallOffer { call_id, .. } => {
                write!(f, "CallOffer({})", call_id)
            }
            Self::CallAnswer { call_id, .. } => {
                write!(f, "CallAnswer({})", call_id)
            }
            Self::IceCandidate { call_id, .. } => {
                write!(f, "IceCandidate({})", call_id)
            }
            Self::CallReject { call_id, reason } => {
                write!(f, "CallReject({}, reason: {:?})", call_id, reason)
            }
            Self::CallHangup { call_id } => {
                write!(f, "CallHangup({})", call_id)
            }
            Self::CallAccept { call_id } => {
                write!(f, "CallAccept({})", call_id)
            }
        }
    }
}

/// Codec for signaling messages over libp2p request-response
#[derive(Debug, Clone, Default)]
pub struct SignalingCodec;

#[async_trait::async_trait]
impl libp2p::request_response::Codec for SignalingCodec {
    type Protocol = libp2p::StreamProtocol;
    type Request = SignalingMessage;
    type Response = SignalingMessage;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> std::io::Result<Self::Request>
    where
        T: futures::AsyncRead + Unpin + Send,
    {
        use futures::AsyncReadExt;

        // Read length prefix (4 bytes)
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        // Read message data
        let mut data = vec![0u8; len];
        io.read_exact(&mut data).await?;

        // Decode JSON
        serde_json::from_slice(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> std::io::Result<Self::Response>
    where
        T: futures::AsyncRead + Unpin + Send,
    {
        use futures::AsyncReadExt;

        // Read length prefix (4 bytes)
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        // Read message data
        let mut data = vec![0u8; len];
        io.read_exact(&mut data).await?;

        // Decode JSON
        serde_json::from_slice(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> std::io::Result<()>
    where
        T: futures::AsyncWrite + Unpin + Send,
    {
        use futures::AsyncWriteExt;

        // Encode to JSON
        let data = serde_json::to_vec(&req)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Write length prefix (4 bytes)
        let len = data.len() as u32;
        io.write_all(&len.to_be_bytes()).await?;

        // Write message data
        io.write_all(&data).await?;
        io.close().await
    }

    async fn write_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> std::io::Result<()>
    where
        T: futures::AsyncWrite + Unpin + Send,
    {
        use futures::AsyncWriteExt;

        // Encode to JSON
        let data = serde_json::to_vec(&res)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Write length prefix (4 bytes)
        let len = data.len() as u32;
        io.write_all(&len.to_be_bytes()).await?;

        // Write message data
        io.write_all(&data).await?;
        io.close().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_offer_message() {
        let msg = SignalingMessage::CallOffer {
            call_id: "call123".to_string(),
            sdp: "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\n...".to_string(),
        };

        assert_eq!(msg.call_id(), "call123");
        assert!(msg.is_offer());
        assert!(!msg.is_answer());
    }

    #[test]
    fn test_ice_candidate_message() {
        let msg = SignalingMessage::IceCandidate {
            call_id: "call123".to_string(),
            candidate: "candidate:1 1 UDP 2130706431 192.168.1.1 54321 typ host".to_string(),
            sdp_mid: Some("0".to_string()),
            sdp_m_line_index: Some(0),
        };

        assert_eq!(msg.call_id(), "call123");
        assert!(msg.is_ice_candidate());
    }

    #[test]
    fn test_codec_creation() {
        let _codec = SignalingCodec::default();
        // SignalingCodec implements libp2p::request_response::Codec
        // Full codec testing requires async I/O streams
    }

    #[test]
    fn test_message_display() {
        let msg = SignalingMessage::CallReject {
            call_id: "call123".to_string(),
            reason: Some("User busy".to_string()),
        };

        let display = format!("{}", msg);
        assert!(display.contains("CallReject"));
        assert!(display.contains("call123"));
    }
}
