//! Data models for Identity Server

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Prekey Bundle for X3DH
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreKeyBundle {
    pub identity_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_identity_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_registration_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_device_id: Option<u32>,
    pub signed_prekey_id: i32,
    pub signed_prekey: String,
    pub signed_prekey_signature: String,
    pub kyber_prekey_id: i32,
    pub kyber_prekey: String,
    pub kyber_prekey_signature: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_time_prekey: Option<OneTimePreKey>,
}

/// One-time prekey
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneTimePreKey {
    pub id: i32,
    pub public_key: String,
}

/// Username registration request
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub peer_id: String,
    pub public_key: String,
    pub prekey_bundle: PreKeyBundle,
    pub signature: String,
    #[serde(default)]
    pub timestamp: i64,
}

/// Username registration response
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub username: String,
    pub peer_id: String,
    pub created_at: DateTime<Utc>,
}

/// Username lookup response
#[derive(Debug, Serialize)]
pub struct LookupResponse {
    pub username: String,
    pub peer_id: String,
    pub prekey_bundle: PreKeyBundle,
    pub last_updated: DateTime<Utc>,
}

/// Update prekeys request
#[derive(Debug, Deserialize)]
pub struct UpdatePrekeysRequest {
    pub peer_id: String,
    pub prekey_bundle: PreKeyBundle,
    pub signature: String,
    #[serde(default)]
    pub timestamp: i64,
}

/// Update prekeys response
#[derive(Debug, Serialize)]
pub struct UpdatePrekeysResponse {
    pub updated_at: DateTime<Utc>,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub database: HealthStatus,
    pub redis: HealthStatus,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub latency_ms: f64,
}

/// Username database row
#[derive(Debug, FromRow)]
pub struct UsernameRow {
    pub username: String,
    pub peer_id: String,
    pub public_key: Vec<u8>,
    pub prekey_bundle: sqlx::types::JsonValue,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

impl UsernameRow {
    pub fn to_lookup_response(self) -> Result<LookupResponse, serde_json::Error> {
        Ok(LookupResponse {
            username: self.username,
            peer_id: self.peer_id,
            prekey_bundle: serde_json::from_value(self.prekey_bundle)?,
            last_updated: self.last_updated,
        })
    }
}
