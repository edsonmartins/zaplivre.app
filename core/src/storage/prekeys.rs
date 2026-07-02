//! SEC-07: persistência do pool de prekeys (cifrado com a storage key).
//! Sem isso o bundle publicado mudava a cada restart, invalidando os
//! bundles que os contatos já tinham buscado.

use super::Database;
use crate::utils::error::{MePassaError, Result};

impl Database {
    pub fn save_prekey_pool(&self, encrypted_snapshot: &[u8]) -> Result<()> {
        self.conn()
            .execute(
                "INSERT INTO identity_prekeys (id, pool, updated_at)
                 VALUES (1, ?1, strftime('%s','now'))
                 ON CONFLICT(id) DO UPDATE SET pool = excluded.pool,
                     updated_at = excluded.updated_at",
                rusqlite::params![encrypted_snapshot],
            )
            .map_err(|e| MePassaError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn load_prekey_pool(&self) -> Result<Option<Vec<u8>>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT pool FROM identity_prekeys WHERE id = 1")
            .map_err(|e| MePassaError::Storage(e.to_string()))?;
        let result = stmt
            .query_row([], |row| row.get::<_, Vec<u8>>(0))
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .map_err(|e| MePassaError::Storage(e.to_string()))?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use crate::storage::{schema::init_schema, Database};

    #[test]
    fn test_prekey_pool_roundtrip() {
        let db = Database::in_memory().unwrap();
        init_schema(&db).unwrap();

        assert!(db.load_prekey_pool().unwrap().is_none());
        db.save_prekey_pool(b"blob-1").unwrap();
        assert_eq!(db.load_prekey_pool().unwrap().unwrap(), b"blob-1");
        db.save_prekey_pool(b"blob-2").unwrap();
        assert_eq!(db.load_prekey_pool().unwrap().unwrap(), b"blob-2");
    }
}
