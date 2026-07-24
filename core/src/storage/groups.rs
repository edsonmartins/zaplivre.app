//! Groups Storage
//!
//! CRUD operations for groups and group members.

use rusqlite::{params, Row};

use super::{Database, Result};

/// Group record
#[derive(Debug, Clone)]
pub struct Group {
    pub id: String,
    pub group_name: String,
    pub group_description: Option<String>,
    pub avatar_hash: Option<String>,
    pub creator_peer_id: String,
    pub created_at: i64,
    pub is_left: bool,
}

/// New group to create
#[derive(Debug, Clone)]
pub struct NewGroup {
    pub id: String,
    pub group_name: String,
    pub group_description: Option<String>,
    pub avatar_hash: Option<String>,
    pub creator_peer_id: String,
}

/// Group member record
#[derive(Debug, Clone)]
pub struct GroupMember {
    pub id: i64,
    pub group_id: String,
    pub peer_id: String,
    pub role: String,
    pub joined_at: i64,
    pub left_at: Option<i64>,
}

/// New group member to add
#[derive(Debug, Clone)]
pub struct NewGroupMember {
    pub group_id: String,
    pub peer_id: String,
    pub role: String,
}

/// Group member role
#[derive(Debug, Clone, PartialEq)]
pub enum MemberRole {
    Owner,
    Admin,
    Member,
}

impl MemberRole {
    pub fn as_str(&self) -> &str {
        match self {
            MemberRole::Owner => "owner",
            MemberRole::Admin => "admin",
            MemberRole::Member => "member",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "owner" => MemberRole::Owner,
            "admin" => MemberRole::Admin,
            "member" => MemberRole::Member,
            _ => MemberRole::Member,
        }
    }
}

impl Database {
    /// Create a new group
    pub fn create_group(&self, group: &NewGroup) -> Result<()> {
        // Insert group
        self.conn().execute(
            r#"
            INSERT INTO groups (id, group_name, group_description, avatar_hash, creator_peer_id)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![
                group.id,
                group.group_name,
                group.group_description,
                group.avatar_hash,
                group.creator_peer_id,
            ],
        )?;

        // Add creator as owner
        let creator_member = NewGroupMember {
            group_id: group.id.clone(),
            peer_id: group.creator_peer_id.clone(),
            role: MemberRole::Owner.as_str().to_string(),
        };
        self.add_group_member(&creator_member)?;

        // Create group conversation
        self.conn().execute(
            r#"
            INSERT INTO conversations (id, conversation_type, group_id, display_name)
            VALUES (?1, 'group', ?2, ?3)
            "#,
            params![format!("group:{}", group.id), group.id, group.group_name,],
        )?;

        Ok(())
    }

    /// Get group by ID
    pub fn get_group(&self, group_id: &str) -> Result<Group> {
        let conn = self.conn();
        conn.query_row(
            r#"
            SELECT id, group_name, group_description, avatar_hash, creator_peer_id, created_at, is_left
            FROM groups
            WHERE id = ?1
            "#,
            params![group_id],
            |row| self.group_from_row(row),
        )
        .map_err(Into::into)
    }

    /// List all groups (not left)
    pub fn list_groups(&self) -> Result<Vec<Group>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, group_name, group_description, avatar_hash, creator_peer_id, created_at, is_left
            FROM groups
            WHERE is_left = 0
            ORDER BY created_at DESC
            "#,
        )?;

        let groups = stmt
            .query_map([], |row| self.group_from_row(row))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(groups)
    }

    /// Update group
    pub fn update_group(
        &self,
        group_id: &str,
        group_name: Option<&str>,
        group_description: Option<&str>,
        avatar_hash: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn();

        let mut updates = Vec::new();
        let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(name) = group_name {
            updates.push("group_name = ?");
            values.push(Box::new(name.to_string()));
        }
        if let Some(desc) = group_description {
            updates.push("group_description = ?");
            values.push(Box::new(desc.to_string()));
        }
        if let Some(hash) = avatar_hash {
            updates.push("avatar_hash = ?");
            values.push(Box::new(hash.to_string()));
        }

        if updates.is_empty() {
            return Ok(());
        }

        let sql = format!("UPDATE groups SET {} WHERE id = ?", updates.join(", "));
        values.push(Box::new(group_id.to_string()));

        let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|b| b.as_ref()).collect();
        conn.execute(&sql, params.as_slice())?;

        Ok(())
    }

    /// Leave group
    pub fn leave_group(&self, group_id: &str, peer_id: &str) -> Result<()> {
        let conn = self.conn();

        // Mark group as left
        conn.execute(
            "UPDATE groups SET is_left = 1 WHERE id = ?1",
            params![group_id],
        )?;

        // Mark member as left
        conn.execute(
            "UPDATE group_members SET left_at = unixepoch() WHERE group_id = ?1 AND peer_id = ?2",
            params![group_id, peer_id],
        )?;

        Ok(())
    }

    /// Add member to group
    pub fn add_group_member(&self, member: &NewGroupMember) -> Result<i64> {
        let conn = self.conn();
        conn.execute(
            r#"
            INSERT INTO group_members (group_id, peer_id, role)
            VALUES (?1, ?2, ?3)
            "#,
            params![member.group_id, member.peer_id, member.role],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Remove member from group
    pub fn remove_group_member(&self, group_id: &str, peer_id: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE group_members SET left_at = unixepoch() WHERE group_id = ?1 AND peer_id = ?2",
            params![group_id, peer_id],
        )?;
        Ok(())
    }

    /// Get group members
    pub fn get_group_members(&self, group_id: &str) -> Result<Vec<GroupMember>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, group_id, peer_id, role, joined_at, left_at
            FROM group_members
            WHERE group_id = ?1 AND left_at IS NULL
            ORDER BY joined_at
            "#,
        )?;

        let members = stmt
            .query_map(params![group_id], |row| self.group_member_from_row(row))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(members)
    }

    /// Update member role
    pub fn update_member_role(
        &self,
        group_id: &str,
        peer_id: &str,
        role: MemberRole,
    ) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE group_members SET role = ?1 WHERE group_id = ?2 AND peer_id = ?3",
            params![role.as_str(), group_id, peer_id],
        )?;
        Ok(())
    }

    /// Check if peer is member of group
    pub fn is_group_member(&self, group_id: &str, peer_id: &str) -> Result<bool> {
        let conn = self.conn();
        let count: i64 = conn.query_row(
            r#"
            SELECT COUNT(*)
            FROM group_members
            WHERE group_id = ?1 AND peer_id = ?2 AND left_at IS NULL
            "#,
            params![group_id, peer_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Helper: Parse group from row
    fn group_from_row(&self, row: &Row) -> rusqlite::Result<Group> {
        Ok(Group {
            id: row.get(0)?,
            group_name: row.get(1)?,
            group_description: row.get(2)?,
            avatar_hash: row.get(3)?,
            creator_peer_id: row.get(4)?,
            created_at: row.get(5)?,
            is_left: row.get::<_, i32>(6)? != 0,
        })
    }

    /// Helper: Parse group member from row
    fn group_member_from_row(&self, row: &Row) -> rusqlite::Result<GroupMember> {
        Ok(GroupMember {
            id: row.get(0)?,
            group_id: row.get(1)?,
            peer_id: row.get(2)?,
            role: row.get(3)?,
            joined_at: row.get(4)?,
            left_at: row.get(5)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrations::migrate;

    fn setup_test_db() -> Database {
        let db = Database::in_memory().unwrap();
        migrate(&db).unwrap();

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
    fn test_create_and_get_group() {
        let db = setup_test_db();

        let new_group = NewGroup {
            id: "group123".to_string(),
            group_name: "Test Group".to_string(),
            group_description: Some("A test group".to_string()),
            avatar_hash: None,
            creator_peer_id: "peer1".to_string(),
        };

        db.create_group(&new_group).unwrap();

        let group = db.get_group("group123").unwrap();
        assert_eq!(group.id, "group123");
        assert_eq!(group.group_name, "Test Group");
        assert_eq!(group.creator_peer_id, "peer1");
    }

    #[test]
    fn test_add_and_get_members() {
        let db = setup_test_db();

        let new_group = NewGroup {
            id: "group123".to_string(),
            group_name: "Test Group".to_string(),
            group_description: None,
            avatar_hash: None,
            creator_peer_id: "peer1".to_string(),
        };
        db.create_group(&new_group).unwrap();

        // Add member
        let new_member = NewGroupMember {
            group_id: "group123".to_string(),
            peer_id: "peer2".to_string(),
            role: MemberRole::Member.as_str().to_string(),
        };
        db.add_group_member(&new_member).unwrap();

        let members = db.get_group_members("group123").unwrap();
        assert_eq!(members.len(), 2); // Creator + added member
    }

    #[test]
    fn test_is_group_member() {
        let db = setup_test_db();

        let new_group = NewGroup {
            id: "group123".to_string(),
            group_name: "Test Group".to_string(),
            group_description: None,
            avatar_hash: None,
            creator_peer_id: "peer1".to_string(),
        };
        db.create_group(&new_group).unwrap();

        assert!(db.is_group_member("group123", "peer1").unwrap());
        assert!(!db.is_group_member("group123", "peer2").unwrap());
    }

    #[test]
    fn test_leave_group() {
        let db = setup_test_db();

        let new_group = NewGroup {
            id: "group123".to_string(),
            group_name: "Test Group".to_string(),
            group_description: None,
            avatar_hash: None,
            creator_peer_id: "peer1".to_string(),
        };
        db.create_group(&new_group).unwrap();

        db.leave_group("group123", "peer1").unwrap();

        let group = db.get_group("group123").unwrap();
        assert!(group.is_left);
    }
}
