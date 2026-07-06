use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};

use crate::utils::error::{ZapLivreError, Result};

pub const MEDIA_ENVELOPE_PREFIX: &str = "MP_MEDIA_V1:";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaEnvelope {
    pub version: u8,
    pub media_type: String,
    pub media_hash: String,
    pub file_name: Option<String>,
    pub mime_type: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration_seconds: Option<i32>,
    pub bytes_b64: String,
    pub thumbnail_b64: Option<String>,
}

impl MediaEnvelope {
    pub fn encode(&self) -> Result<String> {
        let json = serde_json::to_string(self)
            .map_err(|e| ZapLivreError::Protocol(format!("Failed to encode media envelope: {}", e)))?;
        Ok(format!("{}{}", MEDIA_ENVELOPE_PREFIX, json))
    }

    pub fn decode(input: &str) -> Option<Self> {
        if !input.starts_with(MEDIA_ENVELOPE_PREFIX) {
            return None;
        }
        let json = &input[MEDIA_ENVELOPE_PREFIX.len()..];
        serde_json::from_str(json).ok()
    }

    pub fn media_bytes(&self) -> Result<Vec<u8>> {
        general_purpose::STANDARD
            .decode(&self.bytes_b64)
            .map_err(|e| ZapLivreError::Protocol(format!("Invalid media base64: {}", e)))
    }

    pub fn thumbnail_bytes(&self) -> Result<Option<Vec<u8>>> {
        match &self.thumbnail_b64 {
            Some(b64) => general_purpose::STANDARD
                .decode(b64)
                .map(Some)
                .map_err(|e| ZapLivreError::Protocol(format!("Invalid thumbnail base64: {}", e))),
            None => Ok(None),
        }
    }
}
