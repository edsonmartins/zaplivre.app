//! Group Chat Types
//!
//! Data structures for group messaging.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::identity::{Keypair, PublicKey};
use crate::utils::error::{Result, ZapLivreError};

/// Group metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    /// Unique group ID (UUID v4)
    pub id: String,

    /// Group name (max 100 chars)
    pub name: String,

    /// Group description (optional, max 500 chars)
    pub description: Option<String>,

    /// Avatar hash (optional, references media storage)
    pub avatar_hash: Option<String>,

    /// Creator peer ID
    pub creator_peer_id: String,

    /// Current members (peer IDs)
    pub members: HashSet<String>,

    /// Admin peer IDs (subset of members)
    pub admins: HashSet<String>,

    /// Group creation timestamp (Unix epoch)
    pub created_at: i64,

    /// Whether local user has left the group
    pub is_left: bool,

    /// GossipSub topic name (format: "/zaplivre/group/{group_id}")
    pub topic: String,
}

impl Group {
    /// Create a new group
    pub fn new(
        id: String,
        name: String,
        description: Option<String>,
        creator_peer_id: String,
    ) -> Self {
        let topic = format!("/zaplivre/group/{}", id);
        let mut members = HashSet::new();
        members.insert(creator_peer_id.clone());

        let mut admins = HashSet::new();
        admins.insert(creator_peer_id.clone());

        Self {
            id: id.clone(),
            name,
            description,
            avatar_hash: None,
            creator_peer_id,
            members,
            admins,
            created_at: chrono::Utc::now().timestamp(),
            is_left: false,
            topic,
        }
    }

    /// Check if peer is a member
    pub fn is_member(&self, peer_id: &str) -> bool {
        self.members.contains(peer_id)
    }

    /// Check if peer is an admin
    pub fn is_admin(&self, peer_id: &str) -> bool {
        self.admins.contains(peer_id)
    }

    /// Get member count
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Get GossipSub topic hash
    pub fn topic_hash(&self) -> libp2p::gossipsub::IdentTopic {
        libp2p::gossipsub::IdentTopic::new(&self.topic)
    }
}

/// Group member metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    /// Group ID
    pub group_id: String,

    /// Peer ID
    pub peer_id: String,

    /// Member role
    pub role: GroupRole,

    /// Join timestamp (Unix epoch)
    pub joined_at: i64,

    /// Leave timestamp (Unix epoch, None if still member)
    pub left_at: Option<i64>,
}

/// Group member role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GroupRole {
    /// Group creator (can't be removed)
    Creator,

    /// Group admin (can add/remove members, change name)
    Admin,

    /// Regular member (can send messages)
    Member,
}

impl GroupRole {
    /// Check if role can perform admin actions
    pub fn can_admin(&self) -> bool {
        matches!(self, GroupRole::Creator | GroupRole::Admin)
    }

    /// Convert to string for database storage
    pub fn as_str(&self) -> &'static str {
        match self {
            GroupRole::Creator => "creator",
            GroupRole::Admin => "admin",
            GroupRole::Member => "member",
        }
    }

    /// Parse from string (database)
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "creator" => Some(GroupRole::Creator),
            "admin" => Some(GroupRole::Admin),
            "member" => Some(GroupRole::Member),
            _ => None,
        }
    }
}

/// Group message (broadcast via GossipSub)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMessage {
    /// Message ID (UUID v4)
    pub message_id: String,

    /// Group ID
    pub group_id: String,

    /// Sender peer ID
    pub sender_peer_id: String,

    /// Message type
    pub message_type: GroupMessageType,

    /// Message content (encrypted with Sender Keys)
    pub content: Vec<u8>,

    /// Message timestamp (Unix epoch)
    pub timestamp: i64,

    /// Sender signature (Ed25519)
    pub signature: Vec<u8>,
}

impl GroupMessage {
    fn signing_payload(&self) -> Result<Vec<u8>> {
        let mut clone = self.clone();
        clone.signature = Vec::new();
        serde_json::to_vec(&clone)
            .map_err(|e| ZapLivreError::Protocol(format!("Failed to encode group message: {}", e)))
    }

    pub fn sign(&mut self, keypair: &Keypair) -> Result<()> {
        let payload = self.signing_payload()?;
        self.signature = keypair.sign(&payload).to_vec();
        Ok(())
    }

    pub fn verify_signature(&self, public_key: &PublicKey) -> Result<()> {
        let payload = self.signing_payload()?;
        public_key.verify(&payload, &self.signature)
    }
}

/// Group message type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GroupMessageType {
    /// Regular text message
    Text,

    /// Media attachment (image, video, file)
    Media {
        media_hash: String,
        mime_type: String,
    },

    /// System message (member joined/left, name changed, etc.)
    System { system_type: SystemMessageType },

    /// Admin action (add/remove member, promote/demote)
    AdminAction { action: AdminAction },
}

/// System message type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemMessageType {
    /// Group created
    GroupCreated,

    /// Member joined
    MemberJoined { peer_id: String },

    /// Member left
    MemberLeft { peer_id: String },

    /// Group name changed
    NameChanged { old_name: String, new_name: String },

    /// Group description changed
    DescriptionChanged { new_description: String },

    /// Group avatar changed
    AvatarChanged { avatar_hash: String },
}

/// Admin action type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdminAction {
    /// Add member to group
    AddMember { peer_id: String },

    /// Remove member from group
    RemoveMember { peer_id: String },

    /// Promote member to admin
    PromoteToAdmin { peer_id: String },

    /// Demote admin to member
    DemoteToMember { peer_id: String },
}

/// Group event (emitted by GroupManager)
#[derive(Debug, Clone)]
pub enum GroupEvent {
    /// Group created
    GroupCreated { group: Group },

    /// Joined a group
    GroupJoined { group_id: String },

    /// Left a group
    GroupLeft { group_id: String },

    /// New message received
    MessageReceived { message: GroupMessage },

    /// Member added
    MemberAdded { group_id: String, peer_id: String },

    /// Member removed
    MemberRemoved { group_id: String, peer_id: String },

    /// Group metadata updated
    GroupUpdated { group: Group },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_group() {
        let group = Group::new(
            "group-123".to_string(),
            "Test Group".to_string(),
            Some("A test group".to_string()),
            "peer-456".to_string(),
        );

        assert_eq!(group.id, "group-123");
        assert_eq!(group.name, "Test Group");
        assert_eq!(group.creator_peer_id, "peer-456");
        assert_eq!(group.member_count(), 1);
        assert!(group.is_member("peer-456"));
        assert!(group.is_admin("peer-456"));
        assert_eq!(group.topic, "/zaplivre/group/group-123");
    }

    #[test]
    fn test_group_role() {
        assert!(GroupRole::Creator.can_admin());
        assert!(GroupRole::Admin.can_admin());
        assert!(!GroupRole::Member.can_admin());

        assert_eq!(GroupRole::Creator.as_str(), "creator");
        assert_eq!(GroupRole::from_str("admin"), Some(GroupRole::Admin));
        assert_eq!(GroupRole::from_str("invalid"), None);
    }
}
