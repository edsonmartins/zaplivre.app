//! Data models for message store

use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Status of an offline message
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[serde(rename_all = "lowercase")]
pub enum MessageStatus {
    #[sqlx(rename = "pending")]
    Pending,
    #[sqlx(rename = "delivered")]
    Delivered,
    #[sqlx(rename = "expired")]
    Expired,
    #[sqlx(rename = "failed")]
    Failed,
}

impl std::fmt::Display for MessageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageStatus::Pending => write!(f, "pending"),
            MessageStatus::Delivered => write!(f, "delivered"),
            MessageStatus::Expired => write!(f, "expired"),
            MessageStatus::Failed => write!(f, "failed"),
        }
    }
}

/// Offline message stored in database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OfflineMessage {
    pub id: Uuid,
    pub recipient_peer_id: String,
    pub sender_peer_id: String,
    #[serde(with = "base64_serde")]
    pub encrypted_payload: Vec<u8>,
    pub message_type: String,
    pub message_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub status: MessageStatus,
    pub delivery_attempts: i32,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub payload_size_bytes: i32,
}

/// Request to store a new message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreMessageRequest {
    pub recipient_peer_id: String,
    pub sender_peer_id: String,
    /// Base64-encoded encrypted payload
    pub encrypted_payload: String,
    pub message_type: Option<String>,
    pub message_id: String,
}

impl StoreMessageRequest {
    /// Validate request fields
    pub fn validate(&self) -> Result<(), String> {
        if self.recipient_peer_id.is_empty() {
            return Err("recipient_peer_id is required".to_string());
        }
        if self.sender_peer_id.is_empty() {
            return Err("sender_peer_id is required".to_string());
        }
        if self.message_id.is_empty() {
            return Err("message_id is required".to_string());
        }
        if self.encrypted_payload.is_empty() {
            return Err("encrypted_payload is required".to_string());
        }

        // Validate base64
        if general_purpose::STANDARD
            .decode(&self.encrypted_payload)
            .is_err()
        {
            return Err("encrypted_payload must be valid base64".to_string());
        }

        Ok(())
    }
}

/// Response after storing a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreMessageResponse {
    pub id: Uuid,
    pub message_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Request to retrieve pending messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrieveMessagesRequest {
    pub peer_id: String,
    pub limit: Option<i32>,
}

/// Response with pending messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrieveMessagesResponse {
    pub messages: Vec<OfflineMessageDto>,
    pub total: i64,
}

/// DTO for offline message (without internal fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineMessageDto {
    pub id: Uuid,
    pub sender_peer_id: String,
    /// Base64-encoded encrypted payload
    pub encrypted_payload: String,
    pub message_type: String,
    pub message_id: String,
    pub created_at: DateTime<Utc>,
}

impl From<OfflineMessage> for OfflineMessageDto {
    fn from(msg: OfflineMessage) -> Self {
        Self {
            id: msg.id,
            sender_peer_id: msg.sender_peer_id,
            encrypted_payload: general_purpose::STANDARD.encode(&msg.encrypted_payload),
            message_type: msg.message_type,
            message_id: msg.message_id,
            created_at: msg.created_at,
        }
    }
}

/// Request to delete (acknowledge) messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteMessagesRequest {
    pub message_ids: Vec<String>,
}

/// Response after deleting messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteMessagesResponse {
    pub deleted_count: i64,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub database: String,
    pub redis: String,
    pub pending_messages: i64,
}

/// Helper module for base64 serialization
mod base64_serde {
    use base64::{engine::general_purpose, Engine as _};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&general_purpose::STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        general_purpose::STANDARD
            .decode(&s)
            .map_err(serde::de::Error::custom)
    }
}
