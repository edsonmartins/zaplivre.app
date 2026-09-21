//! Wire-level message ids already processed, for idempotent delivery.
//!
//! Delivery is at-least-once: the same message can arrive through the message
//! store and again through the P2P outbox retry. Most messages are not stored
//! under their wire id (group fan-out, reactions, group control), so the
//! `messages` table cannot tell a duplicate apart, and decrypting one twice
//! fails: the Signal ratchet refuses the replay.

use super::Database;
use crate::utils::error::{Result, ZapLivreError};

/// Matches the 14-day retention of the message store and the outbox.
const RETENTION_SECS: i64 = 14 * 24 * 3600;

impl Database {
    /// Whether `message_id` from `sender_peer_id` was already processed.
    pub fn is_message_processed(&self, sender_peer_id: &str, message_id: &str) -> Result<bool> {
        self.conn()
            .query_row(
                "SELECT 1 FROM processed_messages WHERE sender_peer_id = ?1 AND message_id = ?2",
                rusqlite::params![sender_peer_id, message_id],
                |_| Ok(()),
            )
            .map(|_| true)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(false),
                other => Err(ZapLivreError::Storage(other.to_string())),
            })
    }

    /// Record a processed message and drop entries past the retention window.
    pub fn mark_message_processed(&self, sender_peer_id: &str, message_id: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "INSERT OR IGNORE INTO processed_messages (sender_peer_id, message_id)
             VALUES (?1, ?2)",
            rusqlite::params![sender_peer_id, message_id],
        )
        .map_err(|e| ZapLivreError::Storage(e.to_string()))?;
        conn.execute(
            "DELETE FROM processed_messages WHERE processed_at < unixepoch() - ?1",
            rusqlite::params![RETENTION_SECS],
        )
        .map_err(|e| ZapLivreError::Storage(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::storage::{schema::init_schema, Database};

    #[test]
    fn processed_ids_are_scoped_to_the_sender() {
        let db = Database::in_memory().unwrap();
        init_schema(&db).unwrap();

        assert!(!db.is_message_processed("alice", "m1").unwrap());
        db.mark_message_processed("alice", "m1").unwrap();
        db.mark_message_processed("alice", "m1").unwrap();
        assert!(db.is_message_processed("alice", "m1").unwrap());
        // Another peer reusing the id must not suppress Alice's message.
        assert!(!db.is_message_processed("mallory", "m1").unwrap());
    }
}
