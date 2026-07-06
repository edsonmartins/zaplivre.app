//! Message Codec
//!
//! Encodes and decodes Protocol Buffer messages.

use prost::Message as ProstMessage;

use super::pb::Message;
use crate::utils::error::{ZapLivreError, Result};

/// Encode a message to bytes (Protocol Buffer format)
pub fn encode(message: &Message) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    message
        .encode(&mut buf)
        .map_err(|e| ZapLivreError::Protocol(format!("Failed to encode message: {}", e)))?;
    Ok(buf)
}

/// Decode a message from bytes (Protocol Buffer format)
pub fn decode(data: &[u8]) -> Result<Message> {
    Message::decode(data)
        .map_err(|e| ZapLivreError::Protocol(format!("Failed to decode message: {}", e)))
}

/// Encode a message to a length-delimited format
/// Format: [4 bytes length][protobuf data]
pub fn encode_length_delimited(message: &Message) -> Result<Vec<u8>> {
    let data = encode(message)?;
    let len = data.len() as u32;
    let mut buf = Vec::with_capacity(4 + data.len());
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(&data);
    Ok(buf)
}

/// Decode a message from length-delimited format
pub fn decode_length_delimited(data: &[u8]) -> Result<Message> {
    if data.len() < 4 {
        return Err(ZapLivreError::Protocol(
            "Data too short for length-delimited message".to_string(),
        ));
    }

    let len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
    if data.len() < 4 + len {
        return Err(ZapLivreError::Protocol(format!(
            "Expected {} bytes but got {}",
            4 + len,
            data.len()
        )));
    }

    decode(&data[4..4 + len])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::pb::{message::Payload, MessageType, TextMessage};
    use uuid::Uuid;

    fn create_test_message() -> Message {
        Message {
            id: Uuid::new_v4().to_string(),
            sender_peer_id: "sender123".to_string(),
            recipient_peer_id: "recipient456".to_string(),
            timestamp: 1234567890,
            r#type: MessageType::Text as i32,
            payload: Some(Payload::Text(TextMessage {
                content: "Hello, World!".to_string(),
                reply_to_id: String::new(),
                metadata: std::collections::HashMap::new(),
            })),
        }
    }

    #[test]
    fn test_encode_decode() {
        let original = create_test_message();
        let encoded = encode(&original).unwrap();
        let decoded = decode(&encoded).unwrap();

        assert_eq!(original.id, decoded.id);
        assert_eq!(original.sender_peer_id, decoded.sender_peer_id);
        assert_eq!(original.recipient_peer_id, decoded.recipient_peer_id);
        assert_eq!(original.timestamp, decoded.timestamp);
        assert_eq!(original.r#type, decoded.r#type);
    }

    #[test]
    fn test_encode_length_delimited() {
        let message = create_test_message();
        let encoded = encode_length_delimited(&message).unwrap();

        // Check that first 4 bytes are length
        let len = u32::from_be_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]) as usize;
        assert_eq!(len, encoded.len() - 4);

        // Decode and verify
        let decoded = decode_length_delimited(&encoded).unwrap();
        assert_eq!(message.id, decoded.id);
    }

    #[test]
    fn test_decode_invalid_data() {
        let result = decode(&[0xFF, 0xFF, 0xFF]);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_length_delimited_too_short() {
        let result = decode_length_delimited(&[0x00, 0x00]);
        assert!(result.is_err());
    }
}
