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

    /// Server-controlled credential lifetime
    pub credential_ttl_seconds: i64,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let turn_static_secret = std::env::var("TURN_STATIC_SECRET")
            .map_err(|_| anyhow::anyhow!("TURN_STATIC_SECRET must be set"))?;

        // The host is handed to phones on the internet, so it must be the
        // public name/IP of the TURN server. There is no safe default: the
        // docker service name (`coturn`) only resolves inside the compose
        // network, and a credential pointing at it silently breaks every call
        // behind NAT.
        let turn_host = parse_turn_host(std::env::var("TURN_HOST").ok())?;

        // `turns:` (TLS) só é anunciado quando o coturn tem TLS habilitado
        // (TURN_TLS_ENABLED=true). O coturn default usa `no-tls`/`no-dtls`
        // (P1-6): anunciar `turns:` sem listener TLS faz o client tentar uma
        // conexão que nunca completa.
        let tls_enabled = std::env::var("TURN_TLS_ENABLED")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        let mut turn_uris = vec![
            format!("turn:{}:3478?transport=udp", turn_host),
            format!("turn:{}:3478?transport=tcp", turn_host),
        ];
        if tls_enabled {
            turn_uris.push(format!("turns:{}:5349?transport=tcp", turn_host));
        }

        let server_port = std::env::var("SERVER_PORT")
            .unwrap_or_else(|_| "8082".to_string())
            .parse()?;
        let credential_ttl_seconds = std::env::var("TURN_CREDENTIAL_TTL_SECONDS")
            .unwrap_or_else(|_| "3600".to_string())
            .parse()?;

        Ok(Config {
            turn_static_secret,
            turn_uris,
            server_port,
            credential_ttl_seconds,
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
        if !(300..=86400).contains(&self.credential_ttl_seconds) {
            anyhow::bail!("TURN_CREDENTIAL_TTL_SECONDS must be between 300 and 86400");
        }

        Ok(())
    }
}

/// Validate the publicly announced TURN host.
fn parse_turn_host(value: Option<String>) -> Result<String> {
    let host = value.map(|v| v.trim().to_string()).unwrap_or_default();
    if host.is_empty() {
        anyhow::bail!("TURN_HOST must be set to the public hostname or IP of the TURN server");
    }
    if host == "coturn" {
        anyhow::bail!(
            "TURN_HOST=coturn is the internal docker name and is unreachable by clients; \
             use the public hostname or IP"
        );
    }
    Ok(host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation() {
        let config = Config {
            turn_static_secret: "test-secret".to_string(),
            turn_uris: vec!["turn:localhost:3478".to_string()],
            server_port: 8082,
            credential_ttl_seconds: 3600,
        };
        assert!(config.validate().is_ok());

        let invalid_config = Config {
            turn_static_secret: "".to_string(),
            turn_uris: vec![],
            server_port: 8082,
            credential_ttl_seconds: 3600,
        };

        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn turn_host_must_be_public() {
        assert!(parse_turn_host(None).is_err());
        assert!(parse_turn_host(Some("  ".to_string())).is_err());
        assert!(parse_turn_host(Some("coturn".to_string())).is_err());
        assert_eq!(
            parse_turn_host(Some(" turn.zaplivre.app ".to_string())).unwrap(),
            "turn.zaplivre.app"
        );
    }
}
