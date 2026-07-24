use anyhow::Result;
use std::path::PathBuf;

/// Configuration for the Bootstrap Node
#[derive(Debug, Clone)]
pub struct Config {
    /// Port for libp2p listener
    pub p2p_port: u16,

    /// Port for HTTP health check
    pub health_port: u16,

    /// Seed for generating deterministic peer ID
    pub peer_id_seed: String,

    /// Directory for persistent storage
    pub data_dir: PathBuf,

    /// Log level
    pub log_level: String,

    /// Enable relay server functionality
    pub relay_enabled: bool,

    /// Maximum number of concurrent relay circuits
    pub relay_max_circuits: usize,

    /// Maximum circuits per peer
    pub relay_max_per_peer: usize,

    /// Maximum bytes per second per circuit
    pub relay_max_bytes_per_second: u64,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        Ok(Config {
            p2p_port: std::env::var("BOOTSTRAP_PORT")
                .unwrap_or_else(|_| "4001".to_string())
                .parse()?,

            health_port: std::env::var("HEALTH_PORT")
                .unwrap_or_else(|_| "8000".to_string())
                .parse()?,

            peer_id_seed: std::env::var("PEER_ID_SEED")
                .unwrap_or_else(|_| "bootstrap-1".to_string()),

            data_dir: std::env::var("DATA_DIR")
                .unwrap_or_else(|_| "/app/data".to_string())
                .into(),

            log_level: std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),

            relay_enabled: std::env::var("RELAY_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()?,

            relay_max_circuits: std::env::var("RELAY_MAX_CIRCUITS")
                .unwrap_or_else(|_| "100".to_string())
                .parse()?,

            relay_max_per_peer: std::env::var("RELAY_MAX_PER_PEER")
                .unwrap_or_else(|_| "10".to_string())
                .parse()?,

            relay_max_bytes_per_second: std::env::var("RELAY_MAX_BYTES_PER_SEC")
                .unwrap_or_else(|_| "1000000".to_string())
                .parse()?,
        })
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        if self.peer_id_seed.is_empty() {
            anyhow::bail!("PEER_ID_SEED cannot be empty");
        }

        if !self.data_dir.exists() {
            std::fs::create_dir_all(&self.data_dir)?;
        }

        Ok(())
    }
}
