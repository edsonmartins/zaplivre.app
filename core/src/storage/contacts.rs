//! Contacts Management
//!
//! CRUD operations for contacts with @username support.

use chrono::{DateTime, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::{Database, Result, StorageError};

/// Contact information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: i64,
    pub peer_id: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub public_key: Vec<u8>,
    pub prekey_bundle_json: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
}

/// New contact data (for insertion)
#[derive(Debug, Clone)]
pub struct NewContact {
    pub peer_id: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub public_key: Vec<u8>,
    pub prekey_bundle_json: Option<String>,
}

/// Update contact data
#[derive(Debug, Clone, Default)]
pub struct UpdateContact {
    pub username: Option<Option<String>>,
    pub display_name: Option<Option<String>>,
    pub public_key: Option<Vec<u8>>,
    pub prekey_bundle_json: Option<Option<String>>,
    pub last_seen_at: Option<DateTime<Utc>>,
}

impl Database {
    /// Insert a new contact
    pub fn insert_contact(&self, contact: &NewContact) -> Result<i64> {
        self.conn()
            .execute(
                r#"
                INSERT INTO contacts (peer_id, username, display_name, public_key, prekey_bundle_json)
                VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                params![
                    &contact.peer_id,
                    &contact.username,
                    &contact.display_name,
                    &contact.public_key,
                    &contact.prekey_bundle_json,
                ],
            )
            .map_err(|e| {
                if e.to_string().contains("UNIQUE constraint failed") {
                    StorageError::DatabaseError("Contact already exists".to_string())
                } else {
                    StorageError::DatabaseError(format!("Failed to insert contact: {}", e))
                }
            })?;

        Ok(self.conn().last_insert_rowid())
    }

    /// Get contact by peer_id
    pub fn get_contact_by_peer_id(&self, peer_id: &str) -> Result<Contact> {
        self.conn()
            .query_row(
                r#"
                SELECT id, peer_id, username, display_name, public_key, prekey_bundle_json,
                       created_at, last_updated, last_seen_at
                FROM contacts
                WHERE peer_id = ?1
                "#,
                [peer_id],
                |row| {
                    Ok(Contact {
                        id: row.get(0)?,
                        peer_id: row.get(1)?,
                        username: row.get(2)?,
                        display_name: row.get(3)?,
                        public_key: row.get(4)?,
                        prekey_bundle_json: row.get(5)?,
                        created_at: DateTime::from_timestamp(row.get(6)?, 0).unwrap(),
                        last_updated: DateTime::from_timestamp(row.get(7)?, 0).unwrap(),
                        last_seen_at: row
                            .get::<_, Option<i64>>(8)?
                            .and_then(|ts| DateTime::from_timestamp(ts, 0)),
                    })
                },
            )
            .map_err(|e| {
                if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                    StorageError::NotFound(format!("Contact not found: {}", peer_id))
                } else {
                    StorageError::DatabaseError(format!("Failed to get contact: {}", e))
                }
            })
    }

    /// Get contact by username
    pub fn get_contact_by_username(&self, username: &str) -> Result<Contact> {
        self.conn()
            .query_row(
                r#"
                SELECT id, peer_id, username, display_name, public_key, prekey_bundle_json,
                       created_at, last_updated, last_seen_at
                FROM contacts
                WHERE username = ?1
                "#,
                [username],
                |row| {
                    Ok(Contact {
                        id: row.get(0)?,
                        peer_id: row.get(1)?,
                        username: row.get(2)?,
                        display_name: row.get(3)?,
                        public_key: row.get(4)?,
                        prekey_bundle_json: row.get(5)?,
                        created_at: DateTime::from_timestamp(row.get(6)?, 0).unwrap(),
                        last_updated: DateTime::from_timestamp(row.get(7)?, 0).unwrap(),
                        last_seen_at: row
                            .get::<_, Option<i64>>(8)?
                            .and_then(|ts| DateTime::from_timestamp(ts, 0)),
                    })
                },
            )
            .map_err(|e| {
                if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                    StorageError::NotFound(format!("Contact not found: @{}", username))
                } else {
                    StorageError::DatabaseError(format!("Failed to get contact: {}", e))
                }
            })
    }

    /// Update contact
    pub fn update_contact(&self, peer_id: &str, update: &UpdateContact) -> Result<()> {
        let mut sql = String::from("UPDATE contacts SET last_updated = unixepoch()");
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref username) = update.username {
            sql.push_str(", username = ?");
            params.push(Box::new(username.clone()));
        }

        if let Some(ref display_name) = update.display_name {
            sql.push_str(", display_name = ?");
            params.push(Box::new(display_name.clone()));
        }

        if let Some(ref public_key) = update.public_key {
            sql.push_str(", public_key = ?");
            params.push(Box::new(public_key.clone()));
        }

        if let Some(ref prekey_bundle_json) = update.prekey_bundle_json {
            sql.push_str(", prekey_bundle_json = ?");
            params.push(Box::new(prekey_bundle_json.clone()));
        }

        if let Some(ref last_seen_at) = update.last_seen_at {
            sql.push_str(", last_seen_at = ?");
            params.push(Box::new(last_seen_at.timestamp()));
        }

        sql.push_str(" WHERE peer_id = ?");
        params.push(Box::new(peer_id.to_string()));

        let params_refs: Vec<&dyn rusqlite::ToSql> = params
            .iter()
            .map(|p| &**p as &dyn rusqlite::ToSql)
            .collect();

        let affected = self
            .conn()
            .execute(&sql, params_refs.as_slice())
            .map_err(|e| StorageError::DatabaseError(format!("Failed to update contact: {}", e)))?;

        if affected == 0 {
            return Err(StorageError::NotFound(format!(
                "Contact not found: {}",
                peer_id
            )));
        }

        Ok(())
    }

    /// Delete contact
    pub fn delete_contact(&self, peer_id: &str) -> Result<()> {
        let affected = self
            .conn()
            .execute("DELETE FROM contacts WHERE peer_id = ?1", [peer_id])
            .map_err(|e| StorageError::DatabaseError(format!("Failed to delete contact: {}", e)))?;

        if affected == 0 {
            return Err(StorageError::NotFound(format!(
                "Contact not found: {}",
                peer_id
            )));
        }

        Ok(())
    }

    /// List all contacts
    pub fn list_contacts(&self) -> Result<Vec<Contact>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, peer_id, username, display_name, public_key, prekey_bundle_json,
                       created_at, last_updated, last_seen_at
                FROM contacts
                ORDER BY last_updated DESC
                "#,
            )
            .map_err(|e| StorageError::DatabaseError(format!("Failed to prepare query: {}", e)))?;

        let contacts = stmt
            .query_map([], |row| {
                Ok(Contact {
                    id: row.get(0)?,
                    peer_id: row.get(1)?,
                    username: row.get(2)?,
                    display_name: row.get(3)?,
                    public_key: row.get(4)?,
                    prekey_bundle_json: row.get(5)?,
                    created_at: DateTime::from_timestamp(row.get(6)?, 0).unwrap(),
                    last_updated: DateTime::from_timestamp(row.get(7)?, 0).unwrap(),
                    last_seen_at: row
                        .get::<_, Option<i64>>(8)?
                        .and_then(|ts| DateTime::from_timestamp(ts, 0)),
                })
            })
            .map_err(|e| StorageError::DatabaseError(format!("Failed to query contacts: {}", e)))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| {
                StorageError::DatabaseError(format!("Failed to collect contacts: {}", e))
            })?;

        Ok(contacts)
    }

    /// Count total contacts
    pub fn count_contacts(&self) -> Result<i64> {
        let count: i64 = self
            .conn()
            .query_row("SELECT COUNT(*) FROM contacts", [], |row| row.get(0))
            .map_err(|e| StorageError::DatabaseError(format!("Failed to count contacts: {}", e)))?;

        Ok(count)
    }

    /// Search contacts by username or display name
    pub fn search_contacts(&self, query: &str) -> Result<Vec<Contact>> {
        let search_pattern = format!("%{}%", query);

        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, peer_id, username, display_name, public_key, prekey_bundle_json,
                       created_at, last_updated, last_seen_at
                FROM contacts
                WHERE username LIKE ?1 OR display_name LIKE ?1
                ORDER BY last_updated DESC
                "#,
            )
            .map_err(|e| StorageError::DatabaseError(format!("Failed to prepare query: {}", e)))?;

        let contacts = stmt
            .query_map([&search_pattern], |row| {
                Ok(Contact {
                    id: row.get(0)?,
                    peer_id: row.get(1)?,
                    username: row.get(2)?,
                    display_name: row.get(3)?,
                    public_key: row.get(4)?,
                    prekey_bundle_json: row.get(5)?,
                    created_at: DateTime::from_timestamp(row.get(6)?, 0).unwrap(),
                    last_updated: DateTime::from_timestamp(row.get(7)?, 0).unwrap(),
                    last_seen_at: row
                        .get::<_, Option<i64>>(8)?
                        .and_then(|ts| DateTime::from_timestamp(ts, 0)),
                })
            })
            .map_err(|e| StorageError::DatabaseError(format!("Failed to query contacts: {}", e)))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| {
                StorageError::DatabaseError(format!("Failed to collect contacts: {}", e))
            })?;

        Ok(contacts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrations::migrate;

    fn setup_db() -> Database {
        let db = Database::in_memory().unwrap();
        migrate(&db).unwrap();
        db
    }

    #[test]
    fn test_insert_contact() {
        let db = setup_db();

        let contact = NewContact {
            peer_id: "12D3KooWTest".to_string(),
            username: Some("alice".to_string()),
            display_name: Some("Alice Wonderland".to_string()),
            public_key: vec![0u8; 32],
            prekey_bundle_json: None,
        };

        let id = db.insert_contact(&contact).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn test_get_contact_by_peer_id() {
        let db = setup_db();

        let new_contact = NewContact {
            peer_id: "12D3KooWTest".to_string(),
            username: Some("alice".to_string()),
            display_name: Some("Alice".to_string()),
            public_key: vec![0u8; 32],
            prekey_bundle_json: None,
        };

        db.insert_contact(&new_contact).unwrap();

        let contact = db.get_contact_by_peer_id("12D3KooWTest").unwrap();
        assert_eq!(contact.peer_id, "12D3KooWTest");
        assert_eq!(contact.username, Some("alice".to_string()));
    }

    #[test]
    fn test_get_contact_by_username() {
        let db = setup_db();

        let new_contact = NewContact {
            peer_id: "12D3KooWTest".to_string(),
            username: Some("alice".to_string()),
            display_name: None,
            public_key: vec![0u8; 32],
            prekey_bundle_json: None,
        };

        db.insert_contact(&new_contact).unwrap();

        let contact = db.get_contact_by_username("alice").unwrap();
        assert_eq!(contact.peer_id, "12D3KooWTest");
        assert_eq!(contact.username, Some("alice".to_string()));
    }

    #[test]
    fn test_update_contact_username() {
        let db = setup_db();

        let new_contact = NewContact {
            peer_id: "12D3KooWTest".to_string(),
            username: None,
            display_name: Some("Alice".to_string()),
            public_key: vec![0u8; 32],
            prekey_bundle_json: None,
        };

        db.insert_contact(&new_contact).unwrap();

        // Update username
        let update = UpdateContact {
            username: Some(Some("alice".to_string())),
            ..Default::default()
        };

        db.update_contact("12D3KooWTest", &update).unwrap();

        let contact = db.get_contact_by_peer_id("12D3KooWTest").unwrap();
        assert_eq!(contact.username, Some("alice".to_string()));
    }

    #[test]
    fn test_delete_contact() {
        let db = setup_db();

        let new_contact = NewContact {
            peer_id: "12D3KooWTest".to_string(),
            username: Some("alice".to_string()),
            display_name: None,
            public_key: vec![0u8; 32],
            prekey_bundle_json: None,
        };

        db.insert_contact(&new_contact).unwrap();
        db.delete_contact("12D3KooWTest").unwrap();

        let result = db.get_contact_by_peer_id("12D3KooWTest");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_contacts() {
        let db = setup_db();

        // Insert multiple contacts
        for i in 0..3 {
            let contact = NewContact {
                peer_id: format!("peer_{}", i),
                username: Some(format!("user_{}", i)),
                display_name: None,
                public_key: vec![i as u8; 32],
                prekey_bundle_json: None,
            };
            db.insert_contact(&contact).unwrap();
        }

        let contacts = db.list_contacts().unwrap();
        assert_eq!(contacts.len(), 3);
    }

    #[test]
    fn test_count_contacts() {
        let db = setup_db();

        assert_eq!(db.count_contacts().unwrap(), 0);

        let contact = NewContact {
            peer_id: "test".to_string(),
            username: None,
            display_name: None,
            public_key: vec![0u8; 32],
            prekey_bundle_json: None,
        };

        db.insert_contact(&contact).unwrap();
        assert_eq!(db.count_contacts().unwrap(), 1);
    }

    #[test]
    fn test_search_contacts() {
        let db = setup_db();

        let contacts = vec![
            NewContact {
                peer_id: "peer1".to_string(),
                username: Some("alice".to_string()),
                display_name: Some("Alice Wonderland".to_string()),
                public_key: vec![0u8; 32],
                prekey_bundle_json: None,
            },
            NewContact {
                peer_id: "peer2".to_string(),
                username: Some("bob".to_string()),
                display_name: Some("Bob Builder".to_string()),
                public_key: vec![1u8; 32],
                prekey_bundle_json: None,
            },
        ];

        for contact in contacts {
            db.insert_contact(&contact).unwrap();
        }

        // Search by username
        let results = db.search_contacts("alice").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].username, Some("alice".to_string()));

        // Search by display name
        let results = db.search_contacts("Builder").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].username, Some("bob".to_string()));
    }

    #[test]
    fn test_username_unique_constraint() {
        let db = setup_db();

        let contact1 = NewContact {
            peer_id: "peer1".to_string(),
            username: Some("alice".to_string()),
            display_name: None,
            public_key: vec![0u8; 32],
            prekey_bundle_json: None,
        };

        db.insert_contact(&contact1).unwrap();

        // Try to insert same username (should fail)
        let contact2 = NewContact {
            peer_id: "peer2".to_string(),
            username: Some("alice".to_string()),
            display_name: None,
            public_key: vec![1u8; 32],
            prekey_bundle_json: None,
        };

        let result = db.insert_contact(&contact2);
        assert!(result.is_err());
    }
}
