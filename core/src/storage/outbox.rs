//! Outbound Message Retry Queue
//!
//! Persisted queue for messages that could not be delivered (peer offline and
//! no message store, or outbound request failed after the connection dropped).
//! A background worker (spawned in `ClientBuilder::build`) drains this queue
//! with exponential backoff.

use super::{Database, Result};

/// Entry in the outbound retry queue
#[derive(Debug, Clone)]
pub struct OutboundQueueEntry {
    pub id: i64,
    pub message_id: String,
    pub peer_id: String,
    pub message_type: String,
    pub proto_bytes: Vec<u8>,
    pub attempts: u32,
}

impl Database {
    /// Enqueue a message for later delivery. Idempotent per message_id.
    pub fn enqueue_outbound(
        &self,
        message_id: &str,
        peer_id: &str,
        message_type: &str,
        proto_bytes: &[u8],
        next_attempt_at: i64,
    ) -> Result<()> {
        self.conn().execute(
            r#"
            INSERT OR IGNORE INTO outbound_queue
                (message_id, peer_id, message_type, proto_bytes, attempts, next_attempt_at)
            VALUES (?1, ?2, ?3, ?4, 0, ?5)
            "#,
            rusqlite::params![message_id, peer_id, message_type, proto_bytes, next_attempt_at],
        )?;

        Ok(())
    }

    /// Fetch entries whose next attempt is due
    pub fn due_outbound(&self, now: i64, limit: usize) -> Result<Vec<OutboundQueueEntry>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, message_id, peer_id, message_type, proto_bytes, attempts
            FROM outbound_queue
            WHERE next_attempt_at <= ?1
            ORDER BY next_attempt_at ASC
            LIMIT ?2
            "#,
        )?;

        let entries = stmt
            .query_map(rusqlite::params![now, limit as i64], |row| {
                Ok(OutboundQueueEntry {
                    id: row.get(0)?,
                    message_id: row.get(1)?,
                    peer_id: row.get(2)?,
                    message_type: row.get(3)?,
                    proto_bytes: row.get(4)?,
                    attempts: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Record a failed attempt and schedule the next one
    pub fn bump_outbound_attempt(&self, id: i64, next_attempt_at: i64) -> Result<()> {
        self.conn().execute(
            "UPDATE outbound_queue SET attempts = attempts + 1, next_attempt_at = ?2 WHERE id = ?1",
            rusqlite::params![id, next_attempt_at],
        )?;

        Ok(())
    }

    /// Remove an entry (delivered or unrecoverable)
    pub fn remove_outbound(&self, id: i64) -> Result<()> {
        self.conn().execute(
            "DELETE FROM outbound_queue WHERE id = ?1",
            rusqlite::params![id],
        )?;

        Ok(())
    }

    /// Purge entries created before the cutoff (aligned with the 14-day store TTL)
    pub fn purge_expired_outbound(&self, created_before: i64) -> Result<usize> {
        let deleted = self.conn().execute(
            "DELETE FROM outbound_queue WHERE created_at < ?1",
            rusqlite::params![created_before],
        )?;

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use crate::storage::{migrate, Database};

    fn test_db() -> Database {
        let db = Database::in_memory().unwrap();
        migrate(&db).unwrap();
        db
    }

    #[test]
    fn test_enqueue_and_due() {
        let db = test_db();
        db.enqueue_outbound("msg1", "peer1", "text", b"bytes", 100)
            .unwrap();

        // Not due yet
        assert!(db.due_outbound(50, 10).unwrap().is_empty());

        // Due
        let due = db.due_outbound(100, 10).unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].message_id, "msg1");
        assert_eq!(due[0].proto_bytes, b"bytes");

        // Idempotent per message_id
        db.enqueue_outbound("msg1", "peer1", "text", b"bytes", 100)
            .unwrap();
        assert_eq!(db.due_outbound(100, 10).unwrap().len(), 1);
    }

    #[test]
    fn test_bump_and_remove() {
        let db = test_db();
        db.enqueue_outbound("msg1", "peer1", "text", b"bytes", 100)
            .unwrap();
        let entry = &db.due_outbound(100, 10).unwrap()[0];

        db.bump_outbound_attempt(entry.id, 200).unwrap();
        assert!(db.due_outbound(150, 10).unwrap().is_empty());
        let bumped = &db.due_outbound(200, 10).unwrap()[0];
        assert_eq!(bumped.attempts, 1);

        db.remove_outbound(entry.id).unwrap();
        assert!(db.due_outbound(1_000_000, 10).unwrap().is_empty());
    }
}
