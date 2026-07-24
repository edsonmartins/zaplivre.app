//! Group Control Envelope
//!
//! Protocolo in-band de grupos: invite, distribuição de sender key e
//! mudanças de membership trafegam como envelopes tipados dentro de
//! mensagens 1:1 (E2E quando há sessão), no mesmo padrão do
//! `ReactionEnvelope`. Substitui o hack anterior de mandar a seed como
//! mensagem de texto com prefixo (`zaplivre-group-key:v1:`), que era
//! spoofável e dependia de orquestração manual dos apps.

use crate::utils::error::{Result, ZapLivreError};
use serde::{Deserialize, Serialize};

pub const GROUP_CONTROL_PREFIX: &str = "zaplivre-group-ctrl:v1:";

/// Ações do protocolo de grupo
pub mod actions {
    /// Convite: metadados do grupo + snapshot de membros + seed do remetente
    pub const INVITE: &str = "invite";
    /// Distribuição da sender key do remetente para um membro
    pub const SENDER_KEY: &str = "sender_key";
    /// Um admin adicionou um membro
    pub const MEMBER_ADDED: &str = "member_added";
    /// Um admin removeu um membro
    pub const MEMBER_REMOVED: &str = "member_removed";
    /// O remetente saiu do grupo
    pub const LEAVE: &str = "leave";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupControlEnvelope {
    pub version: u8,
    pub action: String,
    pub group_id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_name: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_description: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creator_peer_id: Option<String>,

    /// Snapshot de membership (usado no invite)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<String>>,

    /// Peer alvo de member_added/member_removed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member_peer_id: Option<String>,

    /// Sender key seed do REMETENTE do envelope, base64 (invite/sender_key)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_key_seed: Option<String>,
}

impl GroupControlEnvelope {
    pub fn encode(&self) -> Result<String> {
        let json = serde_json::to_string(self).map_err(|e| {
            ZapLivreError::Protocol(format!("Failed to encode group control envelope: {}", e))
        })?;
        Ok(format!("{}{}", GROUP_CONTROL_PREFIX, json))
    }

    pub fn decode(input: &str) -> Option<Self> {
        if !input.starts_with(GROUP_CONTROL_PREFIX) {
            return None;
        }
        let json = &input[GROUP_CONTROL_PREFIX.len()..];
        serde_json::from_str(json).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let envelope = GroupControlEnvelope {
            version: 1,
            action: actions::INVITE.to_string(),
            group_id: "g1".to_string(),
            group_name: Some("Test".to_string()),
            group_description: None,
            creator_peer_id: Some("peer-a".to_string()),
            members: Some(vec!["peer-a".to_string(), "peer-b".to_string()]),
            member_peer_id: None,
            sender_key_seed: Some("c2VlZA==".to_string()),
        };

        let encoded = envelope.encode().unwrap();
        assert!(encoded.starts_with(GROUP_CONTROL_PREFIX));

        let decoded = GroupControlEnvelope::decode(&encoded).unwrap();
        assert_eq!(decoded.action, actions::INVITE);
        assert_eq!(decoded.group_id, "g1");
        assert_eq!(decoded.members.unwrap().len(), 2);
    }

    #[test]
    fn test_decode_rejects_other_content() {
        assert!(GroupControlEnvelope::decode("hello world").is_none());
        assert!(GroupControlEnvelope::decode("zaplivre-group-key:v1:abc").is_none());
    }
}
