//! Message Reactions Storage
//!
//! Storage for emoji reactions on messages (FASE 16).

use super::{Database, Result};
use serde::{Deserialize, Serialize};

/// Message reaction record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reaction {
    pub reaction_id: String,
    pub message_id: String,
    pub peer_id: String,
    pub emoji: String,
    pub created_at: i64,
}

/// New reaction (before insertion)
#[derive(Debug, Clone)]
pub struct NewReaction {
    pub reaction_id: String,
    pub message_id: String,
    pub peer_id: String,
    pub emoji: String,
}

impl Database {
    /// Add a reaction to a message
    ///
    /// If the same peer already reacted with the same emoji, this is a no-op.
    /// UNIQUE constraint: (message_id, peer_id, emoji)
    pub fn add_reaction(&self, reaction: &NewReaction) -> Result<()> {
        self.conn().execute(
            r#"
            INSERT OR IGNORE INTO message_reactions (reaction_id, message_id, peer_id, emoji)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            rusqlite::params![
                &reaction.reaction_id,
                &reaction.message_id,
                &reaction.peer_id,
                &reaction.emoji,
            ],
        )?;

        Ok(())
    }

    /// Remove a reaction from a message
    pub fn remove_reaction(&self, message_id: &str, peer_id: &str, emoji: &str) -> Result<()> {
        self.conn().execute(
            "DELETE FROM message_reactions WHERE message_id = ?1 AND peer_id = ?2 AND emoji = ?3",
            rusqlite::params![message_id, peer_id, emoji],
        )?;

        Ok(())
    }

    /// Get all reactions for a message
    pub fn get_message_reactions(&self, message_id: &str) -> Result<Vec<Reaction>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT reaction_id, message_id, peer_id, emoji, created_at
            FROM message_reactions
            WHERE message_id = ?1
            ORDER BY created_at ASC
            "#,
        )?;

        let reactions = stmt
            .query_map([message_id], |row| {
                Ok(Reaction {
                    reaction_id: row.get(0)?,
                    message_id: row.get(1)?,
                    peer_id: row.get(2)?,
                    emoji: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(reactions)
    }

    /// Get aggregated reaction counts for a message
    ///
    /// Returns: Vec<(emoji, count)> ordered by count DESC
    pub fn get_message_reaction_counts(&self, message_id: &str) -> Result<Vec<(String, u32)>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT emoji, COUNT(*) as count
            FROM message_reactions
            WHERE message_id = ?1
            GROUP BY emoji
            ORDER BY count DESC, emoji ASC
            "#,
        )?;

        let counts = stmt
            .query_map([message_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(counts)
    }

    /// Check if a peer has reacted to a message with a specific emoji
    pub fn has_reaction(&self, message_id: &str, peer_id: &str, emoji: &str) -> Result<bool> {
        let count: i64 = self.conn().query_row(
            "SELECT COUNT(*) FROM message_reactions WHERE message_id = ?1 AND peer_id = ?2 AND emoji = ?3",
            rusqlite::params![message_id, peer_id, emoji],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    /// Delete all reactions for a message (when message is deleted)
    pub fn delete_message_reactions(&self, message_id: &str) -> Result<()> {
        self.conn().execute(
            "DELETE FROM message_reactions WHERE message_id = ?1",
            [message_id],
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{migrate, schema::init_schema};

    fn setup_db() -> Database {
        let db = Database::in_memory().unwrap();
        init_schema(&db).unwrap();
        migrate(&db).unwrap();

        // Insert test contact and message
        db.conn()
            .execute(
                "INSERT INTO contacts (peer_id, public_key) VALUES (?1, ?2)",
                rusqlite::params!["peer1", vec![0u8; 32]],
            )
            .unwrap();

        db.conn()
            .execute(
                "INSERT INTO messages (message_id, conversation_id, sender_peer_id, message_type, content_plaintext) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params!["msg1", "conv1", "peer1", "text", "Hello"],
            )
            .unwrap();

        db
    }

    #[test]
    fn test_add_reaction() {
        let db = setup_db();

        let reaction = NewReaction {
            reaction_id: "reaction1".to_string(),
            message_id: "msg1".to_string(),
            peer_id: "peer1".to_string(),
            emoji: "👍".to_string(),
        };

        db.add_reaction(&reaction).unwrap();

        let reactions = db.get_message_reactions("msg1").unwrap();
        assert_eq!(reactions.len(), 1);
        assert_eq!(reactions[0].emoji, "👍");
    }

    #[test]
    fn test_remove_reaction() {
        let db = setup_db();

        let reaction = NewReaction {
            reaction_id: "reaction1".to_string(),
            message_id: "msg1".to_string(),
            peer_id: "peer1".to_string(),
            emoji: "👍".to_string(),
        };

        db.add_reaction(&reaction).unwrap();
        db.remove_reaction("msg1", "peer1", "👍").unwrap();

        let reactions = db.get_message_reactions("msg1").unwrap();
        assert_eq!(reactions.len(), 0);
    }

    #[test]
    fn test_duplicate_reaction_ignored() {
        let db = setup_db();

        let reaction = NewReaction {
            reaction_id: "reaction1".to_string(),
            message_id: "msg1".to_string(),
            peer_id: "peer1".to_string(),
            emoji: "👍".to_string(),
        };

        db.add_reaction(&reaction).unwrap();

        // Try to add same reaction again
        let reaction2 = NewReaction {
            reaction_id: "reaction2".to_string(),
            message_id: "msg1".to_string(),
            peer_id: "peer1".to_string(),
            emoji: "👍".to_string(),
        };

        db.add_reaction(&reaction2).unwrap();

        // Should still have only 1 reaction (UNIQUE constraint)
        let reactions = db.get_message_reactions("msg1").unwrap();
        assert_eq!(reactions.len(), 1);
    }

    #[test]
    fn test_reaction_counts() {
        let db = setup_db();

        // Add multiple contacts
        db.conn()
            .execute(
                "INSERT INTO contacts (peer_id, public_key) VALUES (?1, ?2)",
                rusqlite::params!["peer2", vec![1u8; 32]],
            )
            .unwrap();

        db.conn()
            .execute(
                "INSERT INTO contacts (peer_id, public_key) VALUES (?1, ?2)",
                rusqlite::params!["peer3", vec![2u8; 32]],
            )
            .unwrap();

        // Add reactions
        db.add_reaction(&NewReaction {
            reaction_id: "r1".to_string(),
            message_id: "msg1".to_string(),
            peer_id: "peer1".to_string(),
            emoji: "👍".to_string(),
        })
        .unwrap();

        db.add_reaction(&NewReaction {
            reaction_id: "r2".to_string(),
            message_id: "msg1".to_string(),
            peer_id: "peer2".to_string(),
            emoji: "👍".to_string(),
        })
        .unwrap();

        db.add_reaction(&NewReaction {
            reaction_id: "r3".to_string(),
            message_id: "msg1".to_string(),
            peer_id: "peer3".to_string(),
            emoji: "❤️".to_string(),
        })
        .unwrap();

        // Get counts
        let counts = db.get_message_reaction_counts("msg1").unwrap();
        assert_eq!(counts.len(), 2);
        assert_eq!(counts[0], ("👍".to_string(), 2));
        assert_eq!(counts[1], ("❤️".to_string(), 1));
    }

    #[test]
    fn test_has_reaction() {
        let db = setup_db();

        let reaction = NewReaction {
            reaction_id: "reaction1".to_string(),
            message_id: "msg1".to_string(),
            peer_id: "peer1".to_string(),
            emoji: "👍".to_string(),
        };

        db.add_reaction(&reaction).unwrap();

        assert!(db.has_reaction("msg1", "peer1", "👍").unwrap());
        assert!(!db.has_reaction("msg1", "peer1", "❤️").unwrap());
    }
}
