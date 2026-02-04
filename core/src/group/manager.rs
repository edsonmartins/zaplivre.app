//! Group Manager
//!
//! Manages group chat functionality using GossipSub.

use super::storage;
use super::types::{Group, GroupEvent, GroupMessage, GroupRole};
use crate::identity::PublicKey;
use crate::storage::{Database, MessageStatus, NewMessage};
use crate::utils::error::{MePassaError, Result};
use libp2p::gossipsub::{self, IdentTopic, TopicHash};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Group Manager
///
/// Manages group creation, membership, and message broadcasting via GossipSub.
pub struct GroupManager {
    /// Local peer ID
    local_peer_id: String,

    /// Database for persistence
    db: Arc<Database>,

    /// Active groups (group_id -> Group)
    groups: Arc<RwLock<HashMap<String, Group>>>,

    /// Event channel for group events
    event_tx: mpsc::UnboundedSender<GroupEvent>,

    /// Event receiver (consumed by client)
    event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<GroupEvent>>>>,

    /// GossipSub topics we're subscribed to
    subscribed_topics: Arc<RwLock<HashMap<String, TopicHash>>>,
}

impl GroupManager {
    /// Create a new GroupManager
    pub fn new(local_peer_id: String, db: Arc<Database>) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let manager = Self {
            local_peer_id,
            db,
            groups: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_rx: Arc::new(RwLock::new(Some(event_rx))),
            subscribed_topics: Arc::new(RwLock::new(HashMap::new())),
        };

        Ok(manager)
    }

    /// Initialize GroupManager by loading existing groups
    pub async fn init(&self) -> Result<Vec<IdentTopic>> {
        let groups = storage::load_all_groups(&self.db)?;
        let mut topics = Vec::new();

        let mut groups_map = self.groups.write().await;

        for group in groups {
            let topic = IdentTopic::new(&group.topic);
            let topic_hash = topic.hash();
            topics.push(topic.clone());

            self.subscribed_topics
                .write()
                .await
                .insert(group.id.clone(), topic_hash);

            groups_map.insert(group.id.clone(), group);
        }

        Ok(topics)
    }

    /// Take the event receiver (can only be called once)
    pub async fn take_event_receiver(&self) -> Option<mpsc::UnboundedReceiver<GroupEvent>> {
        self.event_rx.write().await.take()
    }

    /// Create a new group
    pub async fn create_group(
        &self,
        name: String,
        description: Option<String>,
    ) -> Result<(Group, IdentTopic)> {
        // Generate group ID
        let group_id = uuid::Uuid::new_v4().to_string();

        // Create group
        let group = Group::new(
            group_id.clone(),
            name,
            description,
            self.local_peer_id.clone(),
        );

        // Save to database
        storage::save_group(&self.db, &group)?;
        self.ensure_group_conversation(&group)?;

        // Store in memory
        self.groups.write().await.insert(group_id.clone(), group.clone());

        // Get topic hash
        let topic = IdentTopic::new(&group.topic);
        let topic_hash = topic.hash();
        self.subscribed_topics
            .write()
            .await
            .insert(group_id, topic_hash.clone());

        // Emit event
        self.emit_event(GroupEvent::GroupCreated {
            group: group.clone(),
        });

        Ok((group, topic))
    }

    /// Join an existing group (invited by admin)
    pub async fn join_group(&self, group_id: String, group_name: String) -> Result<IdentTopic> {
        // Create group entry (we'll receive metadata via group messages)
        let group = Group::new(
            group_id.clone(),
            group_name,
            None,
            self.local_peer_id.clone(), // Temporary, will be updated
        );

        // Save to database
        storage::save_group(&self.db, &group)?;
        self.ensure_group_conversation(&group)?;

        // Store in memory
        self.groups.write().await.insert(group_id.clone(), group.clone());

        // Get topic hash
        let topic = IdentTopic::new(&group.topic);
        let topic_hash = topic.hash();
        self.subscribed_topics
            .write()
            .await
            .insert(group_id.clone(), topic_hash.clone());

        // Emit event
        self.emit_event(GroupEvent::GroupJoined { group_id });

        Ok(topic)
    }

    /// Leave a group
    pub async fn leave_group(&self, group_id: &str) -> Result<()> {
        // Mark as left in database
        storage::mark_group_left(&self.db, group_id)?;

        // Remove from memory
        self.groups.write().await.remove(group_id);

        // Unsubscribe from topic
        self.subscribed_topics.write().await.remove(group_id);

        // Emit event
        self.emit_event(GroupEvent::GroupLeft {
            group_id: group_id.to_string(),
        });

        Ok(())
    }

    /// Add a member to a group (admin only)
    pub async fn add_member(&self, group_id: &str, peer_id: &str) -> Result<()> {
        let mut groups = self.groups.write().await;
        let group = groups
            .get_mut(group_id)
            .ok_or_else(|| MePassaError::NotFound(format!("Group {} not found", group_id)))?;

        // Check if caller is admin
        if !group.is_admin(&self.local_peer_id) {
            return Err(MePassaError::Permission("Only admins can add members".to_string()));
        }

        // Check if already a member
        if group.is_member(peer_id) {
            return Err(MePassaError::AlreadyExists("User is already a member".to_string()));
        }

        // Add to group
        group.members.insert(peer_id.to_string());

        // Save to database
        storage::add_member(&self.db, group_id, peer_id, GroupRole::Member)?;

        // Emit event
        self.emit_event(GroupEvent::MemberAdded {
            group_id: group_id.to_string(),
            peer_id: peer_id.to_string(),
        });

        Ok(())
    }

    /// Remove a member from a group (admin only)
    pub async fn remove_member(&self, group_id: &str, peer_id: &str) -> Result<()> {
        let mut groups = self.groups.write().await;
        let group = groups
            .get_mut(group_id)
            .ok_or_else(|| MePassaError::NotFound(format!("Group {} not found", group_id)))?;

        // Check if caller is admin
        if !group.is_admin(&self.local_peer_id) {
            return Err(MePassaError::Permission("Only admins can remove members".to_string()));
        }

        // Can't remove creator
        if peer_id == group.creator_peer_id {
            return Err(MePassaError::Permission("Can't remove group creator".to_string()));
        }

        // Remove from group
        group.members.remove(peer_id);
        group.admins.remove(peer_id);

        // Save to database
        storage::remove_member(&self.db, group_id, peer_id)?;

        // Emit event
        self.emit_event(GroupEvent::MemberRemoved {
            group_id: group_id.to_string(),
            peer_id: peer_id.to_string(),
        });

        Ok(())
    }

    /// Promote member to admin (admin only)
    pub async fn promote_to_admin(&self, group_id: &str, peer_id: &str) -> Result<()> {
        let mut groups = self.groups.write().await;
        let group = groups
            .get_mut(group_id)
            .ok_or_else(|| MePassaError::NotFound(format!("Group {} not found", group_id)))?;

        // Check if caller is admin
        if !group.is_admin(&self.local_peer_id) {
            return Err(MePassaError::Permission("Only admins can promote members".to_string()));
        }

        // Check if member exists
        if !group.is_member(peer_id) {
            return Err(MePassaError::NotFound("User is not a member".to_string()));
        }

        // Add to admins
        group.admins.insert(peer_id.to_string());

        // Update database
        storage::update_member_role(&self.db, group_id, peer_id, GroupRole::Admin)?;

        Ok(())
    }

    /// Demote admin to member (admin only)
    pub async fn demote_to_member(&self, group_id: &str, peer_id: &str) -> Result<()> {
        let mut groups = self.groups.write().await;
        let group = groups
            .get_mut(group_id)
            .ok_or_else(|| MePassaError::NotFound(format!("Group {} not found", group_id)))?;

        // Check if caller is admin
        if !group.is_admin(&self.local_peer_id) {
            return Err(MePassaError::Permission("Only admins can demote members".to_string()));
        }

        // Can't demote creator
        if peer_id == group.creator_peer_id {
            return Err(MePassaError::Permission("Can't demote group creator".to_string()));
        }

        // Remove from admins
        group.admins.remove(peer_id);

        // Update database
        storage::update_member_role(&self.db, group_id, peer_id, GroupRole::Member)?;

        Ok(())
    }

    /// Update group metadata (admin only)
    pub async fn update_group(
        &self,
        group_id: &str,
        name: Option<String>,
        description: Option<String>,
        avatar_hash: Option<String>,
    ) -> Result<()> {
        let mut groups = self.groups.write().await;
        let group = groups
            .get_mut(group_id)
            .ok_or_else(|| MePassaError::NotFound(format!("Group {} not found", group_id)))?;

        // Check if caller is admin
        if !group.is_admin(&self.local_peer_id) {
            return Err(MePassaError::Permission("Only admins can update group".to_string()));
        }

        // Update fields
        if let Some(ref n) = name {
            group.name = n.clone();
        }
        if let Some(ref d) = description {
            group.description = Some(d.clone());
        }
        if let Some(ref h) = avatar_hash {
            group.avatar_hash = Some(h.clone());
        }

        // Update database
        storage::update_group_metadata(
            &self.db,
            group_id,
            name.as_deref(),
            description.as_deref(),
            avatar_hash.as_deref(),
        )?;

        // Emit event
        self.emit_event(GroupEvent::GroupUpdated {
            group: group.clone(),
        });

        Ok(())
    }

    /// Get a group by ID
    pub async fn get_group(&self, group_id: &str) -> Option<Group> {
        self.groups.read().await.get(group_id).cloned()
    }

    /// Get all groups
    pub async fn get_all_groups(&self) -> Vec<Group> {
        self.groups.read().await.values().cloned().collect()
    }

    /// Handle incoming GossipSub message
    pub async fn handle_gossipsub_message(
        &self,
        topic: &TopicHash,
        message: gossipsub::Message,
    ) -> Result<()> {
        // Deserialize message
        let group_msg: GroupMessage = serde_json::from_slice(&message.data)
            .map_err(|e| MePassaError::Protocol(format!("Invalid group message: {}", e)))?;

        // Verify message is from a group member
        let groups = self.groups.read().await;
        let group = groups.get(&group_msg.group_id);

        if let Some(group) = group {
            if !group.is_member(&group_msg.sender_peer_id) {
                return Err(MePassaError::Permission("Sender is not a group member".to_string()));
            }

            if !group_msg.signature.is_empty() {
                if let Ok(contact) = self.db.get_contact_by_peer_id(&group_msg.sender_peer_id) {
                    let public_key = PublicKey::from_bytes(&contact.public_key)?;
                    group_msg.verify_signature(&public_key)?;
                } else if group_msg.sender_peer_id != self.local_peer_id {
                    return Err(MePassaError::Permission(
                        "Missing sender public key for signature verification".to_string(),
                    ));
                }
            } else {
                return Err(MePassaError::Permission(
                    "Missing group message signature".to_string(),
                ));
            }

            self.ensure_group_conversation(group)?;
            self.store_group_message(group, &group_msg)?;

            // Emit event
            self.emit_event(GroupEvent::MessageReceived {
                message: group_msg,
            });
        }

        Ok(())
    }

    /// Emit a group event
    fn emit_event(&self, event: GroupEvent) {
        let _ = self.event_tx.send(event);
    }

    fn ensure_group_conversation(&self, group: &Group) -> Result<()> {
        let conversation_id = format!("group:{}", group.id);
        let exists: bool = self
            .db
            .conn()
            .query_row(
                "SELECT 1 FROM conversations WHERE id = ?1",
                rusqlite::params![&conversation_id],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if !exists {
            self.db.conn().execute(
                r#"
                INSERT INTO conversations (id, conversation_type, group_id, display_name)
                VALUES (?1, 'group', ?2, ?3)
                "#,
                rusqlite::params![conversation_id, &group.id, &group.name],
            )?;
        }

        Ok(())
    }

    fn store_group_message(&self, group: &Group, group_msg: &GroupMessage) -> Result<()> {
        let conversation_id = format!("group:{}", group.id);
        let content_plaintext = String::from_utf8(group_msg.content.clone()).ok();

        let new_msg = NewMessage {
            message_id: group_msg.message_id.clone(),
            conversation_id: conversation_id.clone(),
            sender_peer_id: group_msg.sender_peer_id.clone(),
            recipient_peer_id: None,
            message_type: "group_text".to_string(),
            content_encrypted: None,
            content_plaintext,
            status: MessageStatus::Delivered,
            parent_message_id: None,
        };

        self.db.insert_message(&new_msg)?;
        self.db.update_conversation_last_message(&conversation_id, &group_msg.message_id)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    #[tokio::test]
    async fn test_create_group() {
        let db = Arc::new(Database::in_memory().unwrap());
        crate::storage::schema::init_schema(&db).unwrap();

        // Insert contact first (foreign key requirement)
        db.conn().execute(
            "INSERT INTO contacts (peer_id, public_key) VALUES (?1, ?2)",
            rusqlite::params!["peer-1", vec![0u8; 32]],
        ).unwrap();

        let manager = GroupManager::new("peer-1".to_string(), db).unwrap();
        manager.init().await.unwrap();

        let (group, _topic) = manager
            .create_group("Test Group".to_string(), Some("Description".to_string()))
            .await
            .unwrap();

        assert_eq!(group.name, "Test Group");
        assert_eq!(group.member_count(), 1);
        assert!(group.is_member("peer-1"));
        assert!(group.is_admin("peer-1"));
    }

    #[tokio::test]
    async fn test_add_remove_member() {
        let db = Arc::new(Database::in_memory().unwrap());
        crate::storage::schema::init_schema(&db).unwrap();

        // Insert contacts first (foreign key requirement)
        db.conn().execute(
            "INSERT INTO contacts (peer_id, public_key) VALUES (?1, ?2)",
            rusqlite::params!["peer-1", vec![0u8; 32]],
        ).unwrap();
        db.conn().execute(
            "INSERT INTO contacts (peer_id, public_key) VALUES (?1, ?2)",
            rusqlite::params!["peer-2", vec![1u8; 32]],
        ).unwrap();

        let manager = GroupManager::new("peer-1".to_string(), db).unwrap();
        manager.init().await.unwrap();

        let (group, _) = manager
            .create_group("Test Group".to_string(), None)
            .await
            .unwrap();

        // Add member
        manager.add_member(&group.id, "peer-2").await.unwrap();

        let updated = manager.get_group(&group.id).await.unwrap();
        assert_eq!(updated.member_count(), 2);
        assert!(updated.is_member("peer-2"));

        // Remove member
        manager.remove_member(&group.id, "peer-2").await.unwrap();

        let updated = manager.get_group(&group.id).await.unwrap();
        assert_eq!(updated.member_count(), 1);
    }

    #[tokio::test]
    async fn test_admin_permissions() {
        let db = Arc::new(Database::in_memory().unwrap());
        crate::storage::schema::init_schema(&db).unwrap();

        // Insert contacts first (foreign key requirement)
        db.conn().execute(
            "INSERT INTO contacts (peer_id, public_key) VALUES (?1, ?2)",
            rusqlite::params!["peer-1", vec![0u8; 32]],
        ).unwrap();
        db.conn().execute(
            "INSERT INTO contacts (peer_id, public_key) VALUES (?1, ?2)",
            rusqlite::params!["peer-2", vec![1u8; 32]],
        ).unwrap();

        let manager = GroupManager::new("peer-1".to_string(), db).unwrap();
        manager.init().await.unwrap();

        let (group, _) = manager
            .create_group("Test Group".to_string(), None)
            .await
            .unwrap();

        // Add and promote member
        manager.add_member(&group.id, "peer-2").await.unwrap();
        manager.promote_to_admin(&group.id, "peer-2").await.unwrap();

        let updated = manager.get_group(&group.id).await.unwrap();
        assert!(updated.is_admin("peer-2"));

        // Demote
        manager.demote_to_member(&group.id, "peer-2").await.unwrap();

        let updated = manager.get_group(&group.id).await.unwrap();
        assert!(!updated.is_admin("peer-2"));
    }
}
