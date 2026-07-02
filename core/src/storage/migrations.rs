//! Database Migrations
//!
//! Manages schema migrations for the SQLite database.

use super::{Database, Result, StorageError};
use crate::storage::schema::{init_fts, init_schema, SCHEMA_VERSION};

/// Migration definition
struct Migration {
    version: i32,
    description: &'static str,
    up: fn(&Database) -> Result<()>,
}

/// All migrations in order
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        description: "Initial schema with contacts.username support",
        up: migrate_to_v1,
    },
    Migration {
        version: 2,
        description: "Add call_history table for VoIP",
        up: migrate_to_v2,
    },
    Migration {
        version: 3,
        description: "Add message_reactions table for emoji reactions",
        up: migrate_to_v3,
    },
    Migration {
        version: 4,
        description: "Add group_sender_keys table for group encryption",
        up: migrate_to_v4,
    },
    Migration {
        version: 5,
        description: "Add outbound_queue table for offline message retry",
        up: migrate_to_v5,
    },
    Migration {
        version: 6,
        description: "Add counter column to group_sender_keys (stateless group crypto)",
        up: migrate_to_v6,
    },
];

/// Migrate database to latest version
pub fn migrate(db: &Database) -> Result<()> {
    let current_version = db.get_version()?;

    tracing::info!(
        "Database at version {}, target version {}",
        current_version,
        SCHEMA_VERSION
    );

    if current_version == SCHEMA_VERSION {
        tracing::info!("Database is up to date");
        return Ok(());
    }

    if current_version > SCHEMA_VERSION {
        return Err(StorageError::MigrationFailed(format!(
            "Database version ({}) is newer than schema version ({}). Please update the app.",
            current_version, SCHEMA_VERSION
        )));
    }

    // Run migrations in order
    for migration in MIGRATIONS {
        if migration.version > current_version {
            tracing::info!(
                "Running migration {}: {}",
                migration.version,
                migration.description
            );

            (migration.up)(db).map_err(|e| {
                StorageError::MigrationFailed(format!(
                    "Migration {} failed: {}",
                    migration.version, e
                ))
            })?;

            db.set_version(migration.version)?;

            tracing::info!("Migration {} completed", migration.version);
        }
    }

    // Ensure version is set to SCHEMA_VERSION even if no migrations ran
    if db.get_version()? < SCHEMA_VERSION {
        db.set_version(SCHEMA_VERSION)?;
        tracing::info!("Set database version to {}", SCHEMA_VERSION);
    }

    tracing::info!("All migrations completed successfully");
    Ok(())
}

/// Migration to version 1: Initial schema
fn migrate_to_v1(db: &Database) -> Result<()> {
    // Create all tables
    init_schema(db)?;

    // Create FTS tables
    init_fts(db)?;

    Ok(())
}

/// Migration to version 2: Add call_history table
fn migrate_to_v2(db: &Database) -> Result<()> {
    db.execute_batch(
        r#"
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
        "#,
    )?;

    Ok(())
}

/// Migration to version 3: Add message_reactions table
fn migrate_to_v3(db: &Database) -> Result<()> {
    db.execute_batch(
        r#"
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

/// Migration to version 4: Add group_sender_keys table
fn migrate_to_v4(db: &Database) -> Result<()> {
    db.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS group_sender_keys (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            group_id TEXT NOT NULL,
            sender_peer_id TEXT NOT NULL,
            sender_key_seed BLOB NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (unixepoch()),
            UNIQUE(group_id, sender_peer_id),
            FOREIGN KEY (group_id) REFERENCES groups(id)
        );

        CREATE INDEX IF NOT EXISTS idx_group_sender_keys_group ON group_sender_keys(group_id);
        CREATE INDEX IF NOT EXISTS idx_group_sender_keys_sender ON group_sender_keys(sender_peer_id);
        "#,
    )?;

    Ok(())
}

fn migrate_to_v5(db: &Database) -> Result<()> {
    db.execute_batch(
        r#"
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
        "#,
    )?;

    Ok(())
}

fn migrate_to_v6(db: &Database) -> Result<()> {
    // SQLite não tem ADD COLUMN IF NOT EXISTS; em DBs frescos o init_schema
    // (v1) já cria a coluna, então checar antes de alterar.
    let has_column: bool = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('group_sender_keys') WHERE name = 'counter'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .unwrap_or(false);

    if !has_column {
        db.execute_batch(
            "ALTER TABLE group_sender_keys ADD COLUMN counter INTEGER NOT NULL DEFAULT 0;",
        )?;
    }

    Ok(())
}

/// Check if database needs migration
pub fn needs_migration(db: &Database) -> Result<bool> {
    let current_version = db.get_version()?;
    Ok(current_version < SCHEMA_VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrate_fresh_database() {
        let db = Database::in_memory().unwrap();
        assert_eq!(db.get_version().unwrap(), 0);

        migrate(&db).unwrap();

        assert_eq!(db.get_version().unwrap(), SCHEMA_VERSION);
        assert!(db.table_exists("contacts").unwrap());
    }

    #[test]
    fn test_migrate_already_up_to_date() {
        let db = Database::in_memory().unwrap();
        migrate(&db).unwrap();

        // Run migration again (should be no-op)
        let result = migrate(&db);
        assert!(result.is_ok());
        assert_eq!(db.get_version().unwrap(), SCHEMA_VERSION);
    }

    #[test]
    fn test_needs_migration() {
        let db = Database::in_memory().unwrap();
        assert!(needs_migration(&db).unwrap());

        migrate(&db).unwrap();
        assert!(!needs_migration(&db).unwrap());
    }

    #[test]
    fn test_migration_creates_username_column() {
        let db = Database::in_memory().unwrap();
        migrate(&db).unwrap();

        // Verify username column exists and works
        db.conn()
            .execute(
                "INSERT INTO contacts (peer_id, username, public_key) VALUES (?1, ?2, ?3)",
                rusqlite::params!["test_peer", "alice", vec![0u8; 32]],
            )
            .unwrap();

        let username: Option<String> = db
            .conn()
            .query_row(
                "SELECT username FROM contacts WHERE peer_id = ?1",
                ["test_peer"],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(username, Some("alice".to_string()));
    }
}
