//! Database Schema Definitions
//!
//! SQL schema for MePassa local storage.

use super::{Database, Result};

/// Current schema version
pub const SCHEMA_VERSION: i32 = 7;

/// Initialize database schema (version 1)
pub fn init_schema(db: &Database) -> Result<()> {
    db.execute_batch(
        r#"
        -- Contacts table: stores peer contacts with optional @username
        CREATE TABLE IF NOT EXISTS contacts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            peer_id TEXT NOT NULL UNIQUE,
            username TEXT UNIQUE,
            display_name TEXT,
            public_key BLOB NOT NULL,
            prekey_bundle_json TEXT,
            created_at INTEGER NOT NULL DEFAULT (unixepoch()),
            last_updated INTEGER NOT NULL DEFAULT (unixepoch()),
            last_seen_at INTEGER
        );

        CREATE INDEX IF NOT EXISTS idx_contacts_peer_id ON contacts(peer_id);
        CREATE INDEX IF NOT EXISTS idx_contacts_username ON contacts(username);
        CREATE INDEX IF NOT EXISTS idx_contacts_last_updated ON contacts(last_updated);

        -- Messages table: stores all messages (sent and received)
        CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            message_id TEXT NOT NULL UNIQUE,
            conversation_id TEXT NOT NULL,
            sender_peer_id TEXT NOT NULL,
            recipient_peer_id TEXT,
            message_type TEXT NOT NULL,
            content_encrypted BLOB,
            content_plaintext TEXT,
            created_at INTEGER NOT NULL DEFAULT (unixepoch()),
            sent_at INTEGER,
            received_at INTEGER,
            read_at INTEGER,
            status TEXT NOT NULL DEFAULT 'pending',
            is_deleted INTEGER NOT NULL DEFAULT 0,
            parent_message_id TEXT,
            FOREIGN KEY (sender_peer_id) REFERENCES contacts(peer_id)
        );

        CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messages(conversation_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_messages_message_id ON messages(message_id);
        CREATE INDEX IF NOT EXISTS idx_messages_status ON messages(status);
        CREATE INDEX IF NOT EXISTS idx_messages_sender ON messages(sender_peer_id);

        -- Conversations table: metadata for conversations (1:1 and groups)
        CREATE TABLE IF NOT EXISTS conversations (
            id TEXT PRIMARY KEY,
            conversation_type TEXT NOT NULL,
            peer_id TEXT,
            group_id TEXT,
            display_name TEXT,
            avatar_hash TEXT,
            last_message_id TEXT,
            last_message_at INTEGER,
            unread_count INTEGER NOT NULL DEFAULT 0,
            is_muted INTEGER NOT NULL DEFAULT 0,
            is_archived INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL DEFAULT (unixepoch()),
            FOREIGN KEY (peer_id) REFERENCES contacts(peer_id),
            FOREIGN KEY (last_message_id) REFERENCES messages(message_id)
        );

        CREATE INDEX IF NOT EXISTS idx_conversations_type ON conversations(conversation_type);
        CREATE INDEX IF NOT EXISTS idx_conversations_peer_id ON conversations(peer_id);
        CREATE INDEX IF NOT EXISTS idx_conversations_last_message ON conversations(last_message_at DESC);

        -- Groups table: group chat metadata
        CREATE TABLE IF NOT EXISTS groups (
            id TEXT PRIMARY KEY,
            group_name TEXT NOT NULL,
            group_description TEXT,
            avatar_hash TEXT,
            creator_peer_id TEXT NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (unixepoch()),
            is_left INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (creator_peer_id) REFERENCES contacts(peer_id)
        );

        -- Group members table: members of each group
        CREATE TABLE IF NOT EXISTS group_members (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            group_id TEXT NOT NULL,
            peer_id TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'member',
            joined_at INTEGER NOT NULL DEFAULT (unixepoch()),
            left_at INTEGER,
            UNIQUE(group_id, peer_id),
            FOREIGN KEY (group_id) REFERENCES groups(id),
            FOREIGN KEY (peer_id) REFERENCES contacts(peer_id)
        );

        CREATE INDEX IF NOT EXISTS idx_group_members_group ON group_members(group_id);
        CREATE INDEX IF NOT EXISTS idx_group_members_peer ON group_members(peer_id);

        -- Group sender keys: per-sender key seeds for group encryption
        CREATE TABLE IF NOT EXISTS group_sender_keys (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            group_id TEXT NOT NULL,
            sender_peer_id TEXT NOT NULL,
            sender_key_seed BLOB NOT NULL,
            counter INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL DEFAULT (unixepoch()),
            UNIQUE(group_id, sender_peer_id),
            FOREIGN KEY (group_id) REFERENCES groups(id)
        );

        CREATE INDEX IF NOT EXISTS idx_group_sender_keys_group ON group_sender_keys(group_id);
        CREATE INDEX IF NOT EXISTS idx_group_sender_keys_sender ON group_sender_keys(sender_peer_id);

        -- Outbound retry queue: messages pending delivery (peer offline)
        CREATE TABLE IF NOT EXISTS outbound_queue (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            message_id TEXT NOT NULL UNIQUE,
            peer_id TEXT NOT NULL,
            message_type TEXT NOT NULL,
            proto_bytes BLOB NOT NULL,
            attempts INTEGER NOT NULL DEFAULT 0,
            next_attempt_at INTEGER NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (unixepoch())
        );

        CREATE INDEX IF NOT EXISTS idx_outbound_queue_next ON outbound_queue(next_attempt_at);
        CREATE INDEX IF NOT EXISTS idx_outbound_queue_peer ON outbound_queue(peer_id);

        -- Signal sessions (E2E) persisted encrypted with the storage key
        CREATE TABLE IF NOT EXISTS signal_sessions (
            address TEXT PRIMARY KEY,
            record BLOB NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT (unixepoch())
        );

        -- TOFU trusted identity keys (public keys - stored as-is)
        CREATE TABLE IF NOT EXISTS signal_identities (
            address TEXT PRIMARY KEY,
            identity_key BLOB NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (unixepoch())
        );

        -- Media table: attachments (images, videos, files)
        CREATE TABLE IF NOT EXISTS media (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            media_hash TEXT NOT NULL UNIQUE,
            message_id TEXT NOT NULL,
            media_type TEXT NOT NULL,
            file_name TEXT,
            file_size INTEGER,
            mime_type TEXT,
            local_path TEXT,
            thumbnail_path TEXT,
            width INTEGER,
            height INTEGER,
            duration_seconds INTEGER,
            created_at INTEGER NOT NULL DEFAULT (unixepoch()),
            FOREIGN KEY (message_id) REFERENCES messages(message_id)
        );

        CREATE INDEX IF NOT EXISTS idx_media_message ON media(message_id);
        CREATE INDEX IF NOT EXISTS idx_media_hash ON media(media_hash);

        -- Crypto sessions table: E2E encryption sessions
        CREATE TABLE IF NOT EXISTS crypto_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            peer_id TEXT NOT NULL,
            session_data BLOB NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (unixepoch()),
            last_used_at INTEGER NOT NULL DEFAULT (unixepoch()),
            UNIQUE(peer_id),
            FOREIGN KEY (peer_id) REFERENCES contacts(peer_id)
        );

        CREATE INDEX IF NOT EXISTS idx_crypto_sessions_peer ON crypto_sessions(peer_id);

        -- Settings table: app settings (key-value store)
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT (unixepoch())
        );

        -- Call history table: record of voice/video calls (FASE 12)
        CREATE TABLE IF NOT EXISTS call_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            call_id TEXT NOT NULL UNIQUE,
            peer_id TEXT NOT NULL,
            call_type TEXT NOT NULL DEFAULT 'audio',
            direction TEXT NOT NULL,
            status TEXT NOT NULL,
            started_at INTEGER NOT NULL,
            ended_at INTEGER,
            duration_seconds INTEGER,
            end_reason TEXT,
            FOREIGN KEY (peer_id) REFERENCES contacts(peer_id)
        );

        CREATE INDEX IF NOT EXISTS idx_call_history_peer ON call_history(peer_id, started_at DESC);
        CREATE INDEX IF NOT EXISTS idx_call_history_started ON call_history(started_at DESC);
        CREATE INDEX IF NOT EXISTS idx_call_history_status ON call_history(status);

        -- Message reactions table: emoji reactions on messages (FASE 16)
        CREATE TABLE IF NOT EXISTS message_reactions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            reaction_id TEXT NOT NULL UNIQUE,
            message_id TEXT NOT NULL,
            peer_id TEXT NOT NULL,
            emoji TEXT NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (unixepoch()),
            UNIQUE(message_id, peer_id, emoji),
            FOREIGN KEY (message_id) REFERENCES messages(message_id),
            FOREIGN KEY (peer_id) REFERENCES contacts(peer_id)
        );

        CREATE INDEX IF NOT EXISTS idx_reactions_message ON message_reactions(message_id);
        CREATE INDEX IF NOT EXISTS idx_reactions_peer ON message_reactions(peer_id);
        "#,
    )?;

    Ok(())
}

/// SQL for full-text search (FTS5) on messages
pub fn init_fts(db: &Database) -> Result<()> {
    db.execute_batch(
        r#"
        -- Full-text search virtual table for messages
        CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
            message_id UNINDEXED,
            content_plaintext,
            content=messages,
            content_rowid=id
        );

        -- Triggers to keep FTS in sync
        CREATE TRIGGER IF NOT EXISTS messages_fts_insert AFTER INSERT ON messages BEGIN
            INSERT INTO messages_fts(rowid, message_id, content_plaintext)
            VALUES (new.id, new.message_id, new.content_plaintext);
        END;

        CREATE TRIGGER IF NOT EXISTS messages_fts_update AFTER UPDATE ON messages BEGIN
            UPDATE messages_fts
            SET content_plaintext = new.content_plaintext
            WHERE rowid = new.id;
        END;

        CREATE TRIGGER IF NOT EXISTS messages_fts_delete AFTER DELETE ON messages BEGIN
            DELETE FROM messages_fts WHERE rowid = old.id;
        END;
        "#,
    )?;

    Ok(())
}

/// Drop all tables (for testing)
#[cfg(test)]
pub fn drop_all_tables(db: &Database) -> Result<()> {
    db.execute_batch(
        r#"
        DROP TABLE IF EXISTS messages_fts;
        DROP TABLE IF EXISTS settings;
        DROP TABLE IF EXISTS crypto_sessions;
        DROP TABLE IF EXISTS media;
        DROP TABLE IF EXISTS group_members;
        DROP TABLE IF EXISTS groups;
        DROP TABLE IF EXISTS conversations;
        DROP TABLE IF EXISTS messages;
        DROP TABLE IF EXISTS contacts;
        "#,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    #[test]
    fn test_init_schema() {
        let db = Database::in_memory().unwrap();
        init_schema(&db).unwrap();

        // Check all tables exist
        assert!(db.table_exists("contacts").unwrap());
        assert!(db.table_exists("messages").unwrap());
        assert!(db.table_exists("conversations").unwrap());
        assert!(db.table_exists("groups").unwrap());
        assert!(db.table_exists("group_members").unwrap());
        assert!(db.table_exists("media").unwrap());
        assert!(db.table_exists("crypto_sessions").unwrap());
        assert!(db.table_exists("settings").unwrap());
    }

    #[test]
    fn test_init_fts() {
        let db = Database::in_memory().unwrap();
        init_schema(&db).unwrap();
        init_fts(&db).unwrap();

        assert!(db.table_exists("messages_fts").unwrap());
    }

    #[test]
    fn test_contacts_table_structure() {
        let db = Database::in_memory().unwrap();
        init_schema(&db).unwrap();

        // Insert a test contact with username
        db.conn()
            .execute(
                r#"
                INSERT INTO contacts (peer_id, username, display_name, public_key)
                VALUES (?1, ?2, ?3, ?4)
                "#,
                rusqlite::params!["12D3KooWTest", "alice", "Alice", vec![0u8; 32]],
            )
            .unwrap();

        // Query it back
        let (peer_id, username): (String, Option<String>) = db
            .conn()
            .query_row(
                "SELECT peer_id, username FROM contacts WHERE username = ?1",
                ["alice"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(peer_id, "12D3KooWTest");
        assert_eq!(username, Some("alice".to_string()));
    }

    #[test]
    fn test_username_unique_constraint() {
        let db = Database::in_memory().unwrap();
        init_schema(&db).unwrap();

        // Insert first contact
        db.conn()
            .execute(
                "INSERT INTO contacts (peer_id, username, public_key) VALUES (?1, ?2, ?3)",
                rusqlite::params!["peer1", "alice", vec![0u8; 32]],
            )
            .unwrap();

        // Try to insert duplicate username (should fail)
        let result = db.conn().execute(
            "INSERT INTO contacts (peer_id, username, public_key) VALUES (?1, ?2, ?3)",
            rusqlite::params!["peer2", "alice", vec![1u8; 32]],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_drop_all_tables() {
        let db = Database::in_memory().unwrap();
        init_schema(&db).unwrap();
        drop_all_tables(&db).unwrap();

        assert!(!db.table_exists("contacts").unwrap());
        assert!(!db.table_exists("messages").unwrap());
    }
}
