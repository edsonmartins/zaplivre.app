use serde::{Deserialize, Serialize};

use crate::utils::error::{MePassaError, Result};

pub const REACTION_ENVELOPE_PREFIX: &str = "MP_REACTION_V1:";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionEnvelope {
    pub version: u8,
    pub action: String,
    pub message_id: String,
    pub emoji: String,
}

impl ReactionEnvelope {
    pub fn encode(&self) -> Result<String> {
        let json = serde_json::to_string(self).map_err(|e| {
            MePassaError::Protocol(format!("Failed to encode reaction envelope: {}", e))
        })?;
        Ok(format!("{}{}", REACTION_ENVELOPE_PREFIX, json))
    }

    pub fn decode(input: &str) -> Option<Self> {
        if !input.starts_with(REACTION_ENVELOPE_PREFIX) {
            return None;
        }
        let json = &input[REACTION_ENVELOPE_PREFIX.len()..];
        serde_json::from_str(json).ok()
    }
}
