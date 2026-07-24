//! Messages Storage
//!
//! CRUD operations for messages and conversations.

use rusqlite::{params, Row};

use super::{Database, Result};

/// Message status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageStatus {
    Pending,
    Sent,
    Delivered,
    Read,
    Failed,
}

impl MessageStatus {
    pub fn as_str(&self) -> &str {
        match self {
            MessageStatus::Pending => "pending",
            MessageStatus::Sent => "sent",
            MessageStatus::Delivered => "delivered",
            MessageStatus::Read => "read",
            MessageStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => MessageStatus::Pending,
            "sent" => MessageStatus::Sent,
            "delivered" => MessageStatus::Delivered,
            "read" => MessageStatus::Read,
            "failed" => MessageStatus::Failed,
            _ => MessageStatus::Pending,
        }
    }
}

/// Message record
#[derive(Debug, Clone)]
pub struct Message {
    pub id: i64,
    pub message_id: String,
    pub conversation_id: String,
    pub sender_peer_id: String,
    pub recipient_peer_id: Option<String>,
    pub message_type: String,
    pub content_encrypted: Option<Vec<u8>>,
    pub content_plaintext: Option<String>,
    pub created_at: i64,
    pub sent_at: Option<i64>,
    pub received_at: Option<i64>,
    pub read_at: Option<i64>,
    pub status: MessageStatus,
    pub is_deleted: bool,
    pub parent_message_id: Option<String>,
}

/// New message to insert
#[derive(Debug, Clone)]
pub struct NewMessage {
    pub message_id: String,
    pub conversation_id: String,
    pub sender_peer_id: String,
    pub recipient_peer_id: Option<String>,
    pub message_type: String,
    pub content_encrypted: Option<Vec<u8>>,
    pub content_plaintext: Option<String>,
    pub status: MessageStatus,
    pub parent_message_id: Option<String>,
}

/// Update message fields
#[derive(Debug, Clone, Default)]
pub struct UpdateMessage {
    pub sent_at: Option<i64>,
    pub received_at: Option<i64>,
    pub read_at: Option<i64>,
    pub status: Option<MessageStatus>,
    pub is_deleted: Option<bool>,
}

/// Conversation record
#[derive(Debug, Clone)]
pub struct Conversation {
    pub id: String,
    pub conversation_type: String,
    pub peer_id: Option<String>,
    pub group_id: Option<String>,
    pub display_name: Option<String>,
    pub avatar_hash: Option<String>,
    pub last_message_id: Option<String>,
    pub last_message_at: Option<i64>,
    pub unread_count: i32,
    pub is_muted: bool,
    pub is_archived: bool,
    pub created_at: i64,
}

impl Database {
    /// Insert a new message
    pub fn insert_message(&self, message: &NewMessage) -> Result<i64> {
        let conn = self.conn();
        conn.execute(
            r#"
            INSERT INTO messages (
                message_id, conversation_id, sender_peer_id, recipient_peer_id,
                message_type, content_encrypted, content_plaintext, status, parent_message_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                message.message_id,
                message.conversation_id,
                message.sender_peer_id,
                message.recipient_peer_id,
                message.message_type,
                message.content_encrypted,
                message.content_plaintext,
                message.status.as_str(),
                message.parent_message_id,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Get message by message_id
    pub fn get_message(&self, message_id: &str) -> Result<Message> {
        let conn = self.conn();
        conn.query_row(
            r#"
            SELECT id, message_id, conversation_id, sender_peer_id, recipient_peer_id,
                   message_type, content_encrypted, content_plaintext, created_at,
                   sent_at, received_at, read_at, status, is_deleted, parent_message_id
            FROM messages
            WHERE message_id = ?1
            "#,
            params![message_id],
            |row| self.message_from_row(row),
        )
        .map_err(Into::into)
    }

    /// Get messages for a conversation
    pub fn get_conversation_messages(
        &self,
        conversation_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Message>> {
        let conn = self.conn();
        let limit = limit.unwrap_or(50);
        let offset = offset.unwrap_or(0);

        let mut stmt = conn.prepare(
            r#"
            SELECT id, message_id, conversation_id, sender_peer_id, recipient_peer_id,
                   message_type, content_encrypted, content_plaintext, created_at,
                   sent_at, received_at, read_at, status, is_deleted, parent_message_id
            FROM messages
            WHERE conversation_id = ?1 AND is_deleted = 0
            ORDER BY created_at DESC
            LIMIT ?2 OFFSET ?3
            "#,
        )?;

        let messages = stmt
            .query_map(params![conversation_id, limit, offset], |row| {
                self.message_from_row(row)
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(messages)
    }

    /// Update message
    pub fn update_message(&self, message_id: &str, update: &UpdateMessage) -> Result<()> {
        let conn = self.conn();

        // Build dynamic SQL based on what fields are being updated
        let mut updates = Vec::new();
        let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(sent_at) = update.sent_at {
            updates.push("sent_at = ?");
            values.push(Box::new(sent_at));
        }
        if let Some(received_at) = update.received_at {
            updates.push("received_at = ?");
            values.push(Box::new(received_at));
        }
        if let Some(read_at) = update.read_at {
            updates.push("read_at = ?");
            values.push(Box::new(read_at));
        }
        if let Some(ref status) = update.status {
            updates.push("status = ?");
            values.push(Box::new(status.as_str().to_string()));
        }
        if let Some(is_deleted) = update.is_deleted {
            updates.push("is_deleted = ?");
            values.push(Box::new(is_deleted as i32));
        }

        if updates.is_empty() {
            return Ok(());
        }

        let sql = format!(
            "UPDATE messages SET {} WHERE message_id = ?",
            updates.join(", ")
        );
        values.push(Box::new(message_id.to_string()));

        let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|b| b.as_ref()).collect();
        conn.execute(&sql, params.as_slice())?;

        Ok(())
    }

    /// Delete message (soft delete)
    pub fn delete_message(&self, message_id: &str) -> Result<()> {
        let update = UpdateMessage {
            is_deleted: Some(true),
            ..Default::default()
        };
        self.update_message(message_id, &update)
    }

    /// Get or create conversation for 1:1 chat
    pub fn get_or_create_conversation(&self, peer_id: &str) -> Result<String> {
        let conversation_id = format!("1:1:{}", peer_id);

        // Try to get existing conversation
        let exists: bool = self
            .conn()
            .query_row(
                "SELECT 1 FROM conversations WHERE id = ?1",
                params![&conversation_id],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if !exists {
            // Ensure contact exists before creating conversation (due to FOREIGN KEY constraint)
            self.ensure_contact_exists(peer_id)?;

            // Create new conversation
            self.conn().execute(
                r#"
                INSERT INTO conversations (id, conversation_type, peer_id)
                VALUES (?1, '1:1', ?2)
                "#,
                params![&conversation_id, peer_id],
            )?;
        }

        Ok(conversation_id)
    }

    /// Ensure a contact exists in the database (create placeholder if not)
    fn ensure_contact_exists(&self, peer_id: &str) -> Result<()> {
        // Check if contact already exists
        let exists: bool = self
            .conn()
            .query_row(
                "SELECT 1 FROM contacts WHERE peer_id = ?1",
                params![peer_id],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if !exists {
            // Create a placeholder contact with minimal info
            // The public_key is empty but will be updated when we receive messages
            self.conn().execute(
                r#"
                INSERT INTO contacts (peer_id, public_key)
                VALUES (?1, ?2)
                "#,
                params![peer_id, Vec::<u8>::new()],
            )?;
        }

        Ok(())
    }

    /// Get conversation by ID
    pub fn get_conversation(&self, conversation_id: &str) -> Result<Conversation> {
        let conn = self.conn();
        conn.query_row(
            r#"
            SELECT id, conversation_type, peer_id, group_id, display_name, avatar_hash,
                   last_message_id, last_message_at, unread_count, is_muted, is_archived, created_at
            FROM conversations
            WHERE id = ?1
            "#,
            params![conversation_id],
            |row| self.conversation_from_row(row),
        )
        .map_err(Into::into)
    }

    /// List all conversations
    pub fn list_conversations(&self) -> Result<Vec<Conversation>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, conversation_type, peer_id, group_id, display_name, avatar_hash,
                   last_message_id, last_message_at, unread_count, is_muted, is_archived, created_at
            FROM conversations
            WHERE is_archived = 0
            ORDER BY last_message_at DESC NULLS LAST
            "#,
        )?;

        let conversations = stmt
            .query_map([], |row| self.conversation_from_row(row))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(conversations)
    }

    /// Update conversation last message
    pub fn update_conversation_last_message(
        &self,
        conversation_id: &str,
        message_id: &str,
    ) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            r#"
            UPDATE conversations
            SET last_message_id = ?1, last_message_at = unixepoch()
            WHERE id = ?2
            "#,
            params![message_id, conversation_id],
        )?;
        Ok(())
    }

    /// Mark conversation as read
    pub fn mark_conversation_read(&self, conversation_id: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE conversations SET unread_count = 0 WHERE id = ?1",
            params![conversation_id],
        )?;
        Ok(())
    }

    /// Search messages using FTS5
    pub fn search_messages(&self, query: &str, limit: Option<usize>) -> Result<Vec<Message>> {
        let conn = self.conn();
        let limit = limit.unwrap_or(50);

        let mut stmt = conn.prepare(
            r#"
            SELECT m.id, m.message_id, m.conversation_id, m.sender_peer_id, m.recipient_peer_id,
                   m.message_type, m.content_encrypted, m.content_plaintext, m.created_at,
                   m.sent_at, m.received_at, m.read_at, m.status, m.is_deleted, m.parent_message_id
            FROM messages m
            JOIN messages_fts fts ON m.id = fts.rowid
            WHERE messages_fts MATCH ?1 AND m.is_deleted = 0
            ORDER BY rank
            LIMIT ?2
            "#,
        )?;

        let messages = stmt
            .query_map(params![query, limit], |row| self.message_from_row(row))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(messages)
    }

    /// Helper: Parse message from row
    fn message_from_row(&self, row: &Row) -> rusqlite::Result<Message> {
        Ok(Message {
            id: row.get(0)?,
            message_id: row.get(1)?,
            conversation_id: row.get(2)?,
            sender_peer_id: row.get(3)?,
            recipient_peer_id: row.get(4)?,
            message_type: row.get(5)?,
            content_encrypted: row.get(6)?,
            content_plaintext: row.get(7)?,
            created_at: row.get(8)?,
            sent_at: row.get(9)?,
            received_at: row.get(10)?,
            read_at: row.get(11)?,
            status: MessageStatus::from_str(&row.get::<_, String>(12)?),
            is_deleted: row.get::<_, i32>(13)? != 0,
            parent_message_id: row.get(14)?,
        })
    }

    /// Helper: Parse conversation from row
    fn conversation_from_row(&self, row: &Row) -> rusqlite::Result<Conversation> {
        Ok(Conversation {
            id: row.get(0)?,
            conversation_type: row.get(1)?,
            peer_id: row.get(2)?,
            group_id: row.get(3)?,
            display_name: row.get(4)?,
            avatar_hash: row.get(5)?,
            last_message_id: row.get(6)?,
            last_message_at: row.get(7)?,
            unread_count: row.get(8)?,
            is_muted: row.get::<_, i32>(9)? != 0,
            is_archived: row.get::<_, i32>(10)? != 0,
            created_at: row.get(11)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::schema::init_schema;

    fn setup_test_db() -> Database {
        let db = Database::in_memory().unwrap();
        init_schema(&db).unwrap();

        // Insert test contacts (required for foreign keys)
        use crate::storage::contacts::NewContact;
        let contact1 = NewContact {
            peer_id: "peer1".to_string(),
            username: None,
            display_name: Some("Peer 1".to_string()),
            public_key: vec![1, 2, 3],
            prekey_bundle_json: None,
        };
        let contact2 = NewContact {
            peer_id: "peer2".to_string(),
            username: None,
            display_name: Some("Peer 2".to_string()),
            public_key: vec![4, 5, 6],
            prekey_bundle_json: None,
        };
        db.insert_contact(&contact1).unwrap();
        db.insert_contact(&contact2).unwrap();

        db
    }

    #[test]
    fn test_insert_and_get_message() {
        let db = setup_test_db();

        let new_msg = NewMessage {
            message_id: "msg123".to_string(),
            conversation_id: "conv1".to_string(),
            sender_peer_id: "peer1".to_string(),
            recipient_peer_id: Some("peer2".to_string()),
            message_type: "text".to_string(),
            content_encrypted: None,
            content_plaintext: Some("Hello!".to_string()),
            status: MessageStatus::Sent,
            parent_message_id: None,
        };

        let id = db.insert_message(&new_msg).unwrap();
        assert!(id > 0);

        let msg = db.get_message("msg123").unwrap();
        assert_eq!(msg.message_id, "msg123");
        assert_eq!(msg.content_plaintext, Some("Hello!".to_string()));
        assert_eq!(msg.status, MessageStatus::Sent);
    }

    #[test]
    fn test_update_message() {
        let db = setup_test_db();

        let new_msg = NewMessage {
            message_id: "msg123".to_string(),
            conversation_id: "conv1".to_string(),
            sender_peer_id: "peer1".to_string(),
            recipient_peer_id: Some("peer2".to_string()),
            message_type: "text".to_string(),
            content_encrypted: None,
            content_plaintext: Some("Hello!".to_string()),
            status: MessageStatus::Pending,
            parent_message_id: None,
        };

        db.insert_message(&new_msg).unwrap();

        let update = UpdateMessage {
            status: Some(MessageStatus::Delivered),
            received_at: Some(1234567890),
            ..Default::default()
        };
        db.update_message("msg123", &update).unwrap();

        let msg = db.get_message("msg123").unwrap();
        assert_eq!(msg.status, MessageStatus::Delivered);
        assert_eq!(msg.received_at, Some(1234567890));
    }

    #[test]
    fn test_conversation_messages() {
        let db = setup_test_db();

        // Insert multiple messages
        for i in 1..=5 {
            let msg = NewMessage {
                message_id: format!("msg{}", i),
                conversation_id: "conv1".to_string(),
                sender_peer_id: "peer1".to_string(),
                recipient_peer_id: Some("peer2".to_string()),
                message_type: "text".to_string(),
                content_encrypted: None,
                content_plaintext: Some(format!("Message {}", i)),
                status: MessageStatus::Sent,
                parent_message_id: None,
            };
            db.insert_message(&msg).unwrap();
        }

        let messages = db
            .get_conversation_messages("conv1", Some(10), None)
            .unwrap();
        assert_eq!(messages.len(), 5);
    }

    #[test]
    fn test_get_or_create_conversation() {
        let db = setup_test_db();

        let conv_id = db.get_or_create_conversation("peer1").unwrap();
        assert_eq!(conv_id, "1:1:peer1");

        // Getting again should return same ID
        let conv_id2 = db.get_or_create_conversation("peer1").unwrap();
        assert_eq!(conv_id, conv_id2);
    }
}
