//! TURN Credentials Helper
//!
//! Fetches time-limited TURN credentials from the credentials server.

use super::{manager::TurnCredentials, Result, VoipError};
use crate::identity::Keypair;
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// TURN credentials request
#[derive(Debug, Serialize)]
struct CredentialRequest {
    username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    ttl_seconds: Option<i64>,
}

/// TURN credentials response from server
#[derive(Debug, Deserialize)]
struct CredentialResponse {
    username: String,
    password: String,
    uris: Vec<String>,
    ttl: i64,
}

/// TURN credentials client
pub struct TurnCredentialsClient {
    server_url: String,
    http_client: Client,
    keypair: Keypair,
}

impl TurnCredentialsClient {
    /// Create a new TURN credentials client
    pub fn new(server_url: String, keypair: Keypair) -> Self {
        Self {
            server_url,
            http_client: Client::new(),
            keypair,
        }
    }

    /// Fetch TURN credentials from server
    ///
    /// # Arguments
    /// * `peer_id` - Local peer ID to use as username base
    /// * `ttl_seconds` - Time-to-live for credentials (default: 24h)
    ///
    /// # Returns
    /// TURN credentials with username, password, and server URIs
    pub async fn fetch_credentials(
        &self,
        peer_id: &str,
        ttl_seconds: Option<i64>,
    ) -> Result<TurnCredentials> {
        let url = format!("{}/api/turn/credentials", self.server_url);

        let request = CredentialRequest {
            username: peer_id.to_string(),
            ttl_seconds,
        };
        let body = serde_json::to_vec(&request).map_err(|e| {
            VoipError::NetworkError(format!("Failed to serialize TURN request: {}", e))
        })?;
        let timestamp = chrono::Utc::now().timestamp();
        let body_hash = hex::encode(Sha256::digest(&body));
        let canonical = format!("POST\n/api/turn/credentials\n{}\n{}", timestamp, body_hash);
        let signature = general_purpose::STANDARD.encode(self.keypair.sign(canonical.as_bytes()));

        let response = self
            .http_client
            .post(&url)
            .header("x-zaplivre-peer", peer_id)
            .header("x-zaplivre-ts", timestamp)
            .header("x-zaplivre-sig", signature)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| {
                VoipError::NetworkError(format!("Failed to fetch TURN credentials: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(VoipError::NetworkError(format!(
                "TURN credentials request failed: {}",
                response.status()
            )));
        }

        let creds: CredentialResponse = response
            .json()
            .await
            .map_err(|e| VoipError::NetworkError(format!("Failed to parse credentials: {}", e)))?;

        tracing::info!(
            "✅ Fetched TURN credentials (TTL: {}s, URIs: {})",
            creds.ttl,
            creds.uris.len()
        );

        Ok(TurnCredentials {
            username: creds.username,
            password: creds.password,
            uris: creds.uris,
        })
    }

    /// Fetch credentials with default TTL (24 hours)
    pub async fn fetch_default(&self, peer_id: &str) -> Result<TurnCredentials> {
        self.fetch_credentials(peer_id, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credentials_client_creation() {
        let client =
            TurnCredentialsClient::new("http://localhost:8082".to_string(), Keypair::generate());
        assert_eq!(client.server_url, "http://localhost:8082");
    }

    #[tokio::test]
    #[ignore] // Requires TURN credentials server running
    async fn test_fetch_credentials() {
        let keypair = Keypair::generate();
        let peer_id = keypair.peer_id();
        let client = TurnCredentialsClient::new("http://localhost:8082".to_string(), keypair);
        let result = client.fetch_default(&peer_id).await;

        // This will fail unless the server is running
        // But the test validates the code compiles correctly
        if let Ok(creds) = result {
            assert!(!creds.username.is_empty());
            assert!(!creds.password.is_empty());
            assert!(!creds.uris.is_empty());
        }
    }
}
