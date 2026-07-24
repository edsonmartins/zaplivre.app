//! Persistent DHT Storage using SQLite
//!
//! Stores peer addresses in SQLite to survive restarts and maintain
//! the DHT routing table across sessions.

use anyhow::Result;
use libp2p::{Multiaddr, PeerId};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

/// Persistent storage for DHT peer records
#[derive(Clone)]
pub struct DhtStorage {
    conn: Arc<Mutex<Connection>>,
}

impl DhtStorage {
    /// Create a new storage instance
    ///
    /// Opens or creates the SQLite database at the specified path
    /// and initializes the schema if needed.
    pub async fn new(db_path: PathBuf) -> Result<Self> {
        info!("📂 Opening DHT storage at: {:?}", db_path);

        // Open database in blocking task
        let conn = tokio::task::spawn_blocking(move || {
            let conn = Connection::open(db_path)?;

            // Enable WAL mode for better concurrency
            conn.pragma_update(None, "journal_mode", "WAL")?;

            // Initialize schema
            conn.execute(
                r#"
                CREATE TABLE IF NOT EXISTS dht_peers (
                    peer_id TEXT NOT NULL,
                    multiaddr TEXT NOT NULL,
                    first_seen INTEGER NOT NULL,
                    last_seen INTEGER NOT NULL,
                    PRIMARY KEY (peer_id, multiaddr)
                )
                "#,
                [],
            )?;

            // Index for efficient queries
            conn.execute(
                r#"
                CREATE INDEX IF NOT EXISTS idx_last_seen
                ON dht_peers(last_seen)
                "#,
                [],
            )?;

            Ok::<_, anyhow::Error>(conn)
        })
        .await??;

        info!("✅ DHT storage ready");
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Add or update a peer address
    pub async fn add_peer(&self, peer_id: &PeerId, addr: &Multiaddr) -> Result<()> {
        let peer_id_str = peer_id.to_string();
        let addr_str = addr.to_string();
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || {
            let now = current_timestamp();
            let conn = conn.lock().unwrap();

            conn.execute(
                r#"
                INSERT INTO dht_peers (peer_id, multiaddr, first_seen, last_seen)
                VALUES (?1, ?2, ?3, ?4)
                ON CONFLICT(peer_id, multiaddr)
                DO UPDATE SET last_seen = ?5
                "#,
                params![&peer_id_str, &addr_str, now, now, now],
            )?;

            tracing::debug!("💾 Saved peer: {} → {}", peer_id_str, addr_str);
            Ok::<_, anyhow::Error>(())
        })
        .await?
    }

    /// Remove a specific peer address
    pub async fn remove_peer(&self, peer_id: &PeerId, addr: &Multiaddr) -> Result<()> {
        let peer_id_str = peer_id.to_string();
        let addr_str = addr.to_string();
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();

            conn.execute(
                r#"
                DELETE FROM dht_peers
                WHERE peer_id = ?1 AND multiaddr = ?2
                "#,
                params![&peer_id_str, &addr_str],
            )?;

            tracing::debug!("🗑️ Removed peer: {} → {}", peer_id_str, addr_str);
            Ok::<_, anyhow::Error>(())
        })
        .await?
    }

    /// Load all stored peers
    ///
    /// Returns a vector of (PeerId, Vec<Multiaddr>) tuples
    pub async fn load_peers(&self) -> Result<Vec<(PeerId, Vec<Multiaddr>)>> {
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare(
                r#"
                SELECT peer_id, multiaddr
                FROM dht_peers
                ORDER BY peer_id
                "#,
            )?;

            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;

            // Group multiaddrs by peer_id
            let mut peers: std::collections::HashMap<PeerId, Vec<Multiaddr>> =
                std::collections::HashMap::new();

            for row in rows {
                let (peer_id_str, addr_str) = row?;

                // Parse peer_id
                let peer_id = match PeerId::from_str(&peer_id_str) {
                    Ok(id) => id,
                    Err(e) => {
                        warn!("❌ Invalid peer_id in database: {} - {}", peer_id_str, e);
                        continue;
                    }
                };

                // Parse multiaddr
                let addr = match Multiaddr::from_str(&addr_str) {
                    Ok(a) => a,
                    Err(e) => {
                        warn!("❌ Invalid multiaddr in database: {} - {}", addr_str, e);
                        continue;
                    }
                };

                peers.entry(peer_id).or_default().push(addr);
            }

            let peer_count = peers.len();
            let addr_count: usize = peers.values().map(|v| v.len()).sum();

            info!(
                "📥 Loaded {} peers with {} addresses from storage",
                peer_count, addr_count
            );

            Ok::<_, anyhow::Error>(peers.into_iter().collect())
        })
        .await?
    }

    /// Remove stale peers (not seen for more than max_age seconds)
    pub async fn cleanup_stale(&self, max_age_secs: i64) -> Result<usize> {
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || {
            let cutoff = current_timestamp() - max_age_secs;
            let conn = conn.lock().unwrap();

            let deleted = conn.execute(
                r#"
                DELETE FROM dht_peers
                WHERE last_seen < ?1
                "#,
                params![cutoff],
            )?;

            if deleted > 0 {
                info!(
                    "🧹 Cleaned up {} stale peer records (older than {}s)",
                    deleted, max_age_secs
                );
            }

            Ok::<_, anyhow::Error>(deleted)
        })
        .await?
    }

    /// Get statistics about stored peers
    pub async fn get_stats(&self) -> Result<StorageStats> {
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();

            let peer_count: i64 =
                conn.query_row("SELECT COUNT(DISTINCT peer_id) FROM dht_peers", [], |row| {
                    row.get(0)
                })?;

            let address_count: i64 =
                conn.query_row("SELECT COUNT(*) FROM dht_peers", [], |row| row.get(0))?;

            Ok::<_, anyhow::Error>(StorageStats {
                peer_count: peer_count as usize,
                address_count: address_count as usize,
            })
        })
        .await?
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub peer_count: usize,
    pub address_count: usize,
}

/// Get current unix timestamp in seconds
fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_storage_basic() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test.db");

        let storage = DhtStorage::new(db_path).await?;

        // Add a peer
        let peer_id = PeerId::random();
        let addr: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse()?;

        storage.add_peer(&peer_id, &addr).await?;

        // Load peers
        let peers = storage.load_peers().await?;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].0, peer_id);
        assert_eq!(peers[0].1[0], addr);

        Ok(())
    }
}
