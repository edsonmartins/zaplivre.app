//! Group Storage Operations
//!
//! Database operations for group persistence.

use super::types::{Group, GroupRole};
use crate::storage::Database;
use crate::utils::error::{MePassaError, Result};
use std::collections::HashSet;

/// Save a group to database
pub fn save_group(db: &Database, group: &Group) -> Result<()> {
    db.conn().execute(
        r#"
        INSERT OR REPLACE INTO groups (id, group_name, group_description, avatar_hash, creator_peer_id, created_at, is_left)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        rusqlite::params![
            &group.id,
            &group.name,
            &group.description,
            &group.avatar_hash,
            &group.creator_peer_id,
            group.created_at,
            if group.is_left { 1 } else { 0 },
        ],
    )?;

    // Save all members (delete existing first for clean state)
    db.conn().execute(
        "DELETE FROM group_members WHERE group_id = ?1",
        rusqlite::params![&group.id],
    )?;

    for peer_id in &group.members {
        let role = if peer_id == &group.creator_peer_id {
            GroupRole::Creator
        } else if group.admins.contains(peer_id) {
            GroupRole::Admin
        } else {
            GroupRole::Member
        };

        db.conn().execute(
            r#"
            INSERT INTO group_members (group_id, peer_id, role, joined_at)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            rusqlite::params![
                &group.id,
                peer_id,
                role.as_str(),
                group.created_at,
            ],
        )?;
    }

    Ok(())
}

/// Load a group from database
pub fn load_group(db: &Database, group_id: &str) -> Result<Group> {
    let (id, name, description, avatar_hash, creator_peer_id, created_at, is_left): (
        String,
        String,
        Option<String>,
        Option<String>,
        String,
        i64,
        i32,
    ) = db
        .conn()
        .query_row(
            r#"
            SELECT id, group_name, group_description, avatar_hash, creator_peer_id, created_at, is_left
            FROM groups
            WHERE id = ?1
            "#,
            [group_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .map_err(|_| MePassaError::NotFound(format!("Group {} not found", group_id)))?;

    // Load members
    let mut members = HashSet::new();
    let mut admins = HashSet::new();

    let conn = db.conn();
    let mut stmt = conn.prepare(
        r#"
        SELECT peer_id, role
        FROM group_members
        WHERE group_id = ?1 AND left_at IS NULL
        "#,
    )?;

    let member_rows = stmt.query_map([group_id], |row| {
        let peer_id: String = row.get(0)?;
        let role_str: String = row.get(1)?;
        Ok((peer_id, role_str))
    })?;

    for row in member_rows {
        let (peer_id, role_str) = row?;
        members.insert(peer_id.clone());

        if let Some(role) = GroupRole::from_str(&role_str) {
            if role.can_admin() {
                admins.insert(peer_id);
            }
        }
    }

    let topic = format!("/mepassa/group/{}", id);

    Ok(Group {
        id,
        name,
        description,
        avatar_hash,
        creator_peer_id,
        members,
        admins,
        created_at,
        is_left: is_left != 0,
        topic,
    })
}

/// Load all groups from database
pub fn load_all_groups(db: &Database) -> Result<Vec<Group>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        r#"
        SELECT id
        FROM groups
        WHERE is_left = 0
        ORDER BY created_at DESC
        "#,
    )?;

    let group_ids = stmt.query_map([], |row| row.get::<_, String>(0))?;

    let mut groups = Vec::new();
    for id_result in group_ids {
        let id = id_result?;
        if let Ok(group) = load_group(db, &id) {
            groups.push(group);
        }
    }

    Ok(groups)
}

/// Add a member to a group
pub fn add_member(db: &Database, group_id: &str, peer_id: &str, role: GroupRole) -> Result<()> {
    let joined_at = chrono::Utc::now().timestamp();

    db.conn().execute(
        r#"
        INSERT OR REPLACE INTO group_members (group_id, peer_id, role, joined_at)
        VALUES (?1, ?2, ?3, ?4)
        "#,
        rusqlite::params![
            group_id,
            peer_id,
            role.as_str(),
            joined_at,
        ],
    )?;

    Ok(())
}

/// Remove a member from a group (set left_at timestamp)
pub fn remove_member(db: &Database, group_id: &str, peer_id: &str) -> Result<()> {
    let left_at = chrono::Utc::now().timestamp();

    db.conn().execute(
        r#"
        UPDATE group_members
        SET left_at = ?1
        WHERE group_id = ?2 AND peer_id = ?3
        "#,
        rusqlite::params![left_at, group_id, peer_id],
    )?;

    Ok(())
}

/// Update member role
pub fn update_member_role(db: &Database, group_id: &str, peer_id: &str, role: GroupRole) -> Result<()> {
    db.conn().execute(
        r#"
        UPDATE group_members
        SET role = ?1
        WHERE group_id = ?2 AND peer_id = ?3
        "#,
        rusqlite::params![role.as_str(), group_id, peer_id],
    )?;

    Ok(())
}

/// Mark group as left
pub fn mark_group_left(db: &Database, group_id: &str) -> Result<()> {
    db.conn().execute(
        r#"
        UPDATE groups
        SET is_left = 1
        WHERE id = ?1
        "#,
        [group_id],
    )?;

    Ok(())
}

/// Update group metadata
pub fn update_group_metadata(
    db: &Database,
    group_id: &str,
    name: Option<&str>,
    description: Option<&str>,
    avatar_hash: Option<&str>,
) -> Result<()> {
    if let Some(name) = name {
        db.conn().execute(
            "UPDATE groups SET group_name = ?1 WHERE id = ?2",
            rusqlite::params![name, group_id],
        )?;
    }

    if let Some(desc) = description {
        db.conn().execute(
            "UPDATE groups SET group_description = ?1 WHERE id = ?2",
            rusqlite::params![desc, group_id],
        )?;
    }

    if let Some(hash) = avatar_hash {
        db.conn().execute(
            "UPDATE groups SET avatar_hash = ?1 WHERE id = ?2",
            rusqlite::params![hash, group_id],
        )?;
    }

    Ok(())
}

pub fn save_sender_key_seed(
    db: &Database,
    group_id: &str,
    sender_peer_id: &str,
    sender_key_seed: &[u8; 32],
) -> Result<()> {
    db.conn().execute(
        r#"
        INSERT OR REPLACE INTO group_sender_keys (group_id, sender_peer_id, sender_key_seed)
        VALUES (?1, ?2, ?3)
        "#,
        rusqlite::params![group_id, sender_peer_id, sender_key_seed.as_slice()],
    )?;

    Ok(())
}

pub fn load_group_sender_keys(
    db: &Database,
    group_id: &str,
) -> Result<Vec<(String, [u8; 32])>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        r#"
        SELECT sender_peer_id, sender_key_seed
        FROM group_sender_keys
        WHERE group_id = ?1
        "#,
    )?;

    let rows = stmt.query_map([group_id], |row| {
        let sender_peer_id: String = row.get(0)?;
        let seed_blob: Vec<u8> = row.get(1)?;
        Ok((sender_peer_id, seed_blob))
    })?;

    let mut result = Vec::new();
    for row in rows {
        let (sender_peer_id, seed_blob) = row?;
        let seed = seed_from_blob(seed_blob)?;
        result.push((sender_peer_id, seed));
    }

    Ok(result)
}

pub fn remove_sender_key(db: &Database, group_id: &str, sender_peer_id: &str) -> Result<()> {
    db.conn().execute(
        r#"
        DELETE FROM group_sender_keys
        WHERE group_id = ?1 AND sender_peer_id = ?2
        "#,
        rusqlite::params![group_id, sender_peer_id],
    )?;

    Ok(())
}

fn seed_from_blob(seed: Vec<u8>) -> Result<[u8; 32]> {
    if seed.len() != 32 {
        return Err(MePassaError::Storage(format!(
            "Invalid sender key seed length: {}",
            seed.len()
        )));
    }

    let mut seed_array = [0u8; 32];
    seed_array.copy_from_slice(&seed);
    Ok(seed_array)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    #[test]
    fn test_save_and_load_group() {
        let db = Database::in_memory().unwrap();
        crate::storage::schema::init_schema(&db).unwrap();

        // Insert contact first (foreign key requirement)
        db.conn().execute(
            "INSERT INTO contacts (peer_id, public_key) VALUES (?1, ?2)",
            rusqlite::params!["peer-1", vec![0u8; 32]],
        ).unwrap();

        let group = Group::new(
            "group-1".to_string(),
            "Test Group".to_string(),
            Some("Description".to_string()),
            "peer-1".to_string(),
        );

        save_group(&db, &group).unwrap();

        let loaded = load_group(&db, "group-1").unwrap();
        assert_eq!(loaded.id, "group-1");
        assert_eq!(loaded.name, "Test Group");
        assert_eq!(loaded.member_count(), 1);
        assert!(loaded.is_member("peer-1"));
    }

    #[test]
    fn test_add_and_remove_member() {
        let db = Database::in_memory().unwrap();
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

        let group = Group::new(
            "group-1".to_string(),
            "Test Group".to_string(),
            None,
            "peer-1".to_string(),
        );

        save_group(&db, &group).unwrap();

        // Add member
        add_member(&db, "group-1", "peer-2", GroupRole::Member).unwrap();

        // Remove member
        remove_member(&db, "group-1", "peer-2").unwrap();

        // Verify removal
        let members: i32 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM group_members WHERE group_id = ?1 AND left_at IS NULL",
                ["group-1"],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(members, 1); // Only creator remains
    }
}
