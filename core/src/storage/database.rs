//! SQLite Database Connection
//!
//! Manages the local SQLite database for storing messages, contacts, and groups.

use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::sync::{Arc, Mutex};

use super::{Result, StorageError};

/// SQLite database wrapper (thread-safe)
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
        }
    }
}

impl Database {
    /// Open or create a database at the specified path
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the SQLite database file
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use zaplivre_core::storage::Database;
    ///
    /// let db = Database::open("./data/zaplivre.db").unwrap();
    /// ```
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                StorageError::DatabaseError(format!("Failed to create data directory: {}", e))
            })?;
        }

        // Open database with create flag
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| StorageError::DatabaseError(format!("Failed to open database: {}", e)))?;

        // Enable WAL mode for better concurrency
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| {
                StorageError::DatabaseError(format!("Failed to enable WAL mode: {}", e))
            })?;

        // Enable foreign keys
        conn.execute_batch("PRAGMA foreign_keys=ON;").map_err(|e| {
            StorageError::DatabaseError(format!("Failed to enable foreign keys: {}", e))
        })?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Open an in-memory database (for testing)
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| StorageError::DatabaseError(format!("Failed to open database: {}", e)))?;

        conn.execute_batch("PRAGMA foreign_keys=ON;").map_err(|e| {
            StorageError::DatabaseError(format!("Failed to enable foreign keys: {}", e))
        })?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Execute a SQL statement
    pub fn execute(&self, sql: &str) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| {
            StorageError::DatabaseError(format!("Failed to lock connection: {}", e))
        })?;
        conn.execute(sql, [])
            .map_err(|e| StorageError::DatabaseError(format!("Execute failed: {}", e)))
    }

    /// Execute a batch of SQL statements
    pub fn execute_batch(&self, sql: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| {
            StorageError::DatabaseError(format!("Failed to lock connection: {}", e))
        })?;
        conn.execute_batch(sql)
            .map_err(|e| StorageError::DatabaseError(format!("Execute batch failed: {}", e)))
    }

    /// Check if a table exists
    pub fn table_exists(&self, table_name: &str) -> Result<bool> {
        let conn = self.conn.lock().map_err(|e| {
            StorageError::DatabaseError(format!("Failed to lock connection: {}", e))
        })?;
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name=?1")
            .map_err(|e| StorageError::DatabaseError(format!("Prepare failed: {}", e)))?;

        let exists = stmt
            .exists([table_name])
            .map_err(|e| StorageError::DatabaseError(format!("Query failed: {}", e)))?;

        Ok(exists)
    }

    /// Get current database version
    pub fn get_version(&self) -> Result<i32> {
        let conn = self.conn.lock().map_err(|e| {
            StorageError::DatabaseError(format!("Failed to lock connection: {}", e))
        })?;
        let version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|e| StorageError::DatabaseError(format!("Failed to get version: {}", e)))?;

        Ok(version)
    }

    /// Set database version
    pub fn set_version(&self, version: i32) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| {
            StorageError::DatabaseError(format!("Failed to lock connection: {}", e))
        })?;
        conn.execute(&format!("PRAGMA user_version = {}", version), [])
            .map_err(|e| StorageError::DatabaseError(format!("Failed to set version: {}", e)))?;

        Ok(())
    }

    /// Close the database connection
    pub fn close(self) -> Result<()> {
        let conn = Arc::try_unwrap(self.conn)
            .map_err(|_| {
                StorageError::DatabaseError(
                    "Cannot close: connection still has references".to_string(),
                )
            })?
            .into_inner()
            .map_err(|e| {
                StorageError::DatabaseError(format!("Failed to lock connection: {}", e))
            })?;
        conn.close()
            .map_err(|(_, e)| StorageError::DatabaseError(format!("Failed to close: {}", e)))
    }

    /// Get access to the connection (for internal storage module use)
    pub(crate) fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn
            .lock()
            .expect("Failed to lock database connection")
    }

    /// Execute a query with the connection (for testing/internal use)
    #[cfg(test)]
    pub(crate) fn with_connection<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&Connection) -> rusqlite::Result<R>,
    {
        let conn = self.conn.lock().map_err(|e| {
            StorageError::DatabaseError(format!("Failed to lock connection: {}", e))
        })?;
        f(&conn).map_err(|e| StorageError::DatabaseError(format!("Query failed: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_database() {
        let db = Database::in_memory().unwrap();
        assert_eq!(db.get_version().unwrap(), 0);
    }

    #[test]
    fn test_set_version() {
        let db = Database::in_memory().unwrap();
        db.set_version(1).unwrap();
        assert_eq!(db.get_version().unwrap(), 1);
    }

    #[test]
    fn test_table_exists() {
        let db = Database::in_memory().unwrap();

        // Create a test table
        db.execute_batch("CREATE TABLE test (id INTEGER PRIMARY KEY)")
            .unwrap();

        assert!(db.table_exists("test").unwrap());
        assert!(!db.table_exists("nonexistent").unwrap());
    }

    #[test]
    fn test_wal_mode() {
        let db = Database::in_memory().unwrap();

        let mode: String = db
            .with_connection(|conn| conn.query_row("PRAGMA journal_mode", [], |row| row.get(0)))
            .unwrap();

        // In-memory databases can't use WAL, but file-based databases should
        assert!(mode == "memory" || mode == "wal");
    }

    #[test]
    fn test_foreign_keys() {
        let db = Database::in_memory().unwrap();

        let enabled: i32 = db
            .with_connection(|conn| conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0)))
            .unwrap();

        assert_eq!(enabled, 1);
    }
}
