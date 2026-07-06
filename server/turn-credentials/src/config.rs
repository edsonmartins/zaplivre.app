//! Configuration for TURN credentials service

use anyhow::Result;

/// Configuration for TURN credentials service
#[derive(Clone, Debug)]
pub struct Config {
    /// Static secret shared with coturn server
    pub turn_static_secret: String,

    /// TURN server URIs
    pub turn_uris: Vec<String>,

    /// Server port
    pub server_port: u16,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let turn_static_secret = std::env::var("TURN_STATIC_SECRET")
            .unwrap_or_else(|_| "zaplivre_turn_dev_secret".to_string());

        // In production, these should be the actual external IPs/domains
        let turn_host = std::env::var("TURN_HOST")
            .unwrap_or_else(|_| "coturn".to_string());

        let turn_uris = vec![
            format!("turn:{}:3478?transport=udp", turn_host),
            format!("turn:{}:3478?transport=tcp", turn_host),
            format!("turns:{}:5349?transport=tcp", turn_host),
        ];

        let server_port = std::env::var("SERVER_PORT")
            .unwrap_or_else(|_| "8082".to_string())
            .parse()?;

        Ok(Config {
            turn_static_secret,
            turn_uris,
            server_port,
        })
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.turn_static_secret.is_empty() {
            anyhow::bail!("TURN_STATIC_SECRET cannot be empty");
        }

        if self.turn_uris.is_empty() {
            anyhow::bail!("TURN_URIS cannot be empty");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::from_env().unwrap();

        assert!(!config.turn_static_secret.is_empty());
        assert!(!config.turn_uris.is_empty());
        assert!(config.server_port > 0);
    }

    #[test]
    fn test_validation() {
        let config = Config::from_env().unwrap();
        assert!(config.validate().is_ok());

        let invalid_config = Config {
            turn_static_secret: "".to_string(),
            turn_uris: vec![],
            server_port: 8082,
        };

        assert!(invalid_config.validate().is_err());
    }
}
