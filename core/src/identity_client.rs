//! Identity Server Client
//!
//! HTTP client for communicating with the Identity Server to register and lookup @usernames.

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::identity::{Identity, PreKeyBundle as CorePreKeyBundle};

/// Prekey Bundle for Identity Server API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreKeyBundle {
    pub identity_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_identity_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_registration_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_device_id: Option<u32>,
    pub signed_prekey_id: i32,
    pub signed_prekey: String,
    pub signed_prekey_signature: String,
    pub kyber_prekey_id: i32,
    pub kyber_prekey: String,
    pub kyber_prekey_signature: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_time_prekey: Option<OneTimePreKey>,
}

/// One-time prekey
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneTimePreKey {
    pub id: i32,
    pub public_key: String,
}

impl PreKeyBundle {
    /// Convert from core PreKeyBundle to API format
    pub fn from_core(bundle: &CorePreKeyBundle) -> Self {
        Self {
            identity_key: general_purpose::STANDARD.encode(bundle.identity_key),
            signal_identity_key: bundle
                .signal_identity_key
                .as_ref()
                .map(|value| general_purpose::STANDARD.encode(value)),
            signal_registration_id: bundle.signal_registration_id,
            signal_device_id: bundle.signal_device_id,
            signed_prekey_id: bundle.signed_prekey_id as i32,
            signed_prekey: general_purpose::STANDARD.encode(bundle.signed_prekey.clone()),
            signed_prekey_signature: general_purpose::STANDARD.encode(&bundle.signed_prekey_signature),
            kyber_prekey_id: bundle.kyber_prekey_id as i32,
            kyber_prekey: general_purpose::STANDARD.encode(&bundle.kyber_prekey),
            kyber_prekey_signature: general_purpose::STANDARD.encode(&bundle.kyber_prekey_signature),
            one_time_prekey: bundle.one_time_prekey.as_ref().map(|opk| OneTimePreKey {
                id: opk.id as i32,
                public_key: general_purpose::STANDARD.encode(&opk.public_key),
            }),
        }
    }

    /// Convert to core PreKeyBundle format
    pub fn to_core(&self) -> Result<CorePreKeyBundle> {
        let identity_key_bytes = general_purpose::STANDARD.decode(&self.identity_key)?;
        let signal_identity_key_bytes = match &self.signal_identity_key {
            Some(value) => Some(general_purpose::STANDARD.decode(value)?),
            None => None,
        };
        let signed_prekey_bytes = general_purpose::STANDARD.decode(&self.signed_prekey)?;
        let signed_prekey_signature_bytes = general_purpose::STANDARD.decode(&self.signed_prekey_signature)?;
        let kyber_prekey_bytes = general_purpose::STANDARD.decode(&self.kyber_prekey)?;
        let kyber_prekey_signature_bytes = general_purpose::STANDARD.decode(&self.kyber_prekey_signature)?;

        let mut identity_key = [0u8; 32];
        identity_key.copy_from_slice(&identity_key_bytes);

        let one_time_prekey = if let Some(opk) = &self.one_time_prekey {
            let public_key_bytes = general_purpose::STANDARD.decode(&opk.public_key)?;
            Some(crate::identity::prekeys::OneTimePreKey {
                id: opk.id as u32,
                public_key: public_key_bytes,
            })
        } else {
            None
        };

        Ok(CorePreKeyBundle {
            identity_key,
            signal_identity_key: signal_identity_key_bytes,
            signal_registration_id: self.signal_registration_id,
            signal_device_id: self.signal_device_id,
            signed_prekey_id: self.signed_prekey_id as u32,
            signed_prekey: signed_prekey_bytes,
            signed_prekey_signature: signed_prekey_signature_bytes,
            kyber_prekey_id: self.kyber_prekey_id as u32,
            kyber_prekey: kyber_prekey_bytes,
            kyber_prekey_signature: kyber_prekey_signature_bytes,
            one_time_prekey,
        })
    }
}

/// Register request
#[derive(Debug, Serialize)]
struct RegisterRequest {
    username: String,
    peer_id: String,
    public_key: String,
    prekey_bundle: PreKeyBundle,
    signature: String,
    timestamp: i64,
}

/// Register response
#[derive(Debug, Deserialize)]
pub struct RegisterResponse {
    pub username: String,
    pub peer_id: String,
    pub created_at: DateTime<Utc>,
}

/// Lookup response
#[derive(Debug, Deserialize)]
pub struct LookupResponse {
    pub username: String,
    pub peer_id: String,
    pub prekey_bundle: PreKeyBundle,
    pub last_updated: DateTime<Utc>,
}

/// Update prekeys request
#[derive(Debug, Serialize)]
struct UpdatePrekeysRequest {
    peer_id: String,
    prekey_bundle: PreKeyBundle,
    signature: String,
    timestamp: i64,
}

/// Update prekeys response
#[derive(Debug, Deserialize)]
pub struct UpdatePrekeysResponse {
    pub updated_at: DateTime<Utc>,
}

/// Error response from Identity Server
#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[allow(dead_code)]
    suggestions: Option<Vec<String>>,
}

/// Identity Server client
#[derive(Clone)]
pub struct IdentityClient {
    base_url: String,
    client: reqwest::Client,
}

impl IdentityClient {
    /// Create a new Identity Server client
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            base_url: base_url.into(),
            client,
        })
    }

    /// Register a new username
    ///
    /// # Arguments
    /// * `identity` - User's identity (contains keypair)
    /// * `username` - Desired username (3-20 chars, lowercase alphanumeric + underscore)
    /// * `peer_id` - libp2p peer ID
    ///
    /// # Returns
    /// RegisterResponse with created_at timestamp
    ///
    /// # Errors
    /// - `INVALID_USERNAME` - Username format invalid
    /// - `USERNAME_TAKEN` - Username already registered
    /// - `INVALID_SIGNATURE` - Signature verification failed
    /// - `RATE_LIMIT_EXCEEDED` - Too many requests (5/hour limit)
    pub async fn register_username(
        &self,
        identity: &Identity,
        username: &str,
        peer_id: &str,
    ) -> Result<RegisterResponse> {
        // Get prekey bundle
        let mut identity_mut = identity.clone();
        let prekey_bundle = identity_mut
            .prekey_pool_mut()
            .ok_or_else(|| anyhow!("No prekey pool"))?
            .get_bundle()
            .map_err(|e| anyhow!(e.to_string()))?;

        // Create signature
        let timestamp = Utc::now().timestamp();
        let message = format!("register:{}:{}", username, timestamp);
        let signature = identity.keypair().sign(message.as_bytes());

        // Build request
        let request = RegisterRequest {
            username: username.to_string(),
            peer_id: peer_id.to_string(),
            public_key: general_purpose::STANDARD.encode(identity.keypair().public_key_bytes()),
            prekey_bundle: PreKeyBundle::from_core(&prekey_bundle),
            signature: general_purpose::STANDARD.encode(signature),
            timestamp,
        };

        // Send request
        let url = format!("{}/api/v1/register", self.base_url);
        let response = self.client.post(&url).json(&request).send().await?;

        // Handle response
        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let error: ErrorResponse = response.json().await?;
            Err(anyhow!("{}: {}", error.error, error.message))
        }
    }

    /// Lookup a username
    ///
    /// # Arguments
    /// * `username` - Username to lookup
    ///
    /// # Returns
    /// LookupResponse with peer_id and prekey_bundle
    ///
    /// # Errors
    /// - `USERNAME_NOT_FOUND` - Username not registered
    /// - `RATE_LIMIT_EXCEEDED` - Too many requests (100/hour limit)
    pub async fn lookup_username(&self, username: &str) -> Result<LookupResponse> {
        let url = format!("{}/api/v1/lookup?username={}", self.base_url, username);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let error: ErrorResponse = response.json().await?;
            Err(anyhow!("{}: {}", error.error, error.message))
        }
    }

    /// Update prekeys for a peer
    ///
    /// # Arguments
    /// * `identity` - User's identity (contains keypair)
    /// * `peer_id` - libp2p peer ID
    ///
    /// # Returns
    /// UpdatePrekeysResponse with updated_at timestamp
    ///
    /// # Errors
    /// - `USERNAME_NOT_FOUND` - Peer ID not registered
    /// - `INVALID_SIGNATURE` - Signature verification failed
    /// - `RATE_LIMIT_EXCEEDED` - Too many requests (50/hour limit)
    pub async fn update_prekeys(
        &self,
        identity: &Identity,
        peer_id: &str,
    ) -> Result<UpdatePrekeysResponse> {
        // Get new prekey bundle
        let mut identity_mut = identity.clone();
        let prekey_bundle = identity_mut
            .prekey_pool_mut()
            .ok_or_else(|| anyhow!("No prekey pool"))?
            .get_bundle()
            .map_err(|e| anyhow!(e.to_string()))?;

        // Create signature
        let timestamp = Utc::now().timestamp();
        let message = format!("update_prekeys:{}:{}", peer_id, timestamp);
        let signature = identity.keypair().sign(message.as_bytes());

        // Build request
        let request = UpdatePrekeysRequest {
            peer_id: peer_id.to_string(),
            prekey_bundle: PreKeyBundle::from_core(&prekey_bundle),
            signature: general_purpose::STANDARD.encode(signature),
            timestamp,
        };

        // Send request
        let url = format!("{}/api/v1/prekeys", self.base_url);
        let response = self.client.put(&url).json(&request).send().await?;

        // Handle response
        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let error: ErrorResponse = response.json().await?;
            Err(anyhow!("{}: {}", error.error, error.message))
        }
    }

    /// Check Identity Server health
    pub async fn health_check(&self) -> Result<serde_json::Value> {
        let url = format!("{}/health", self.base_url);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(anyhow!("Health check failed: {}", response.status()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Identity;

    #[test]
    fn test_prekey_bundle_conversion() {
        // Generate identity with prekeys
        let identity = Identity::generate(1);
        let mut identity_mut = identity.clone();
        let core_bundle = identity_mut
            .prekey_pool_mut()
            .unwrap()
            .get_bundle()
            .expect("failed to get prekey bundle");

        // Convert to API format and back
        let api_bundle = PreKeyBundle::from_core(&core_bundle);
        let converted_bundle = api_bundle.to_core().unwrap();

        // Verify identity key matches
        assert_eq!(core_bundle.identity_key, converted_bundle.identity_key);

        // Verify signed prekey matches
        assert_eq!(
            core_bundle.signed_prekey_id,
            converted_bundle.signed_prekey_id
        );
        assert_eq!(core_bundle.signed_prekey, converted_bundle.signed_prekey);
    }

    #[test]
    fn test_client_creation() {
        let client = IdentityClient::new("http://localhost:8080").unwrap();
        assert_eq!(client.base_url, "http://localhost:8080");
    }

    #[tokio::test]
    async fn test_register_username_signature() {
        // This test verifies signature creation (without actually calling server)
        let identity = Identity::generate(1);
        let username = "alice";
        let peer_id = "12D3KooWTest";
        let timestamp = Utc::now().timestamp();

        // Create signature
        let message = format!("register:{}:{}", username, timestamp);
        let signature = identity.keypair().sign(message.as_bytes());

        // Verify signature
        assert!(identity
            .keypair()
            .verify(message.as_bytes(), &signature)
            .is_ok());
    }

    #[tokio::test]
    async fn test_update_prekeys_signature() {
        let identity = Identity::generate(1);
        let peer_id = "12D3KooWTest";
        let timestamp = Utc::now().timestamp();

        // Create signature
        let message = format!("update_prekeys:{}:{}", peer_id, timestamp);
        let signature = identity.keypair().sign(message.as_bytes());

        // Verify signature
        assert!(identity
            .keypair()
            .verify(message.as_bytes(), &signature)
            .is_ok());
    }

    // Integration tests (require Identity Server running)
    // Run with: cargo test --features integration-tests

    #[cfg(feature = "integration-tests")]
    mod integration {
        use super::*;

        #[tokio::test]
        async fn test_register_and_lookup() {
            let client = IdentityClient::new("http://localhost:8080").unwrap();
            let identity = Identity::generate(10);
            let username = format!("test_{}", rand::random::<u32>());
            let peer_id = format!("12D3KooW{}", rand::random::<u64>());

            // Register username
            let register_response = client
                .register_username(&identity, &username, &peer_id)
                .await
                .unwrap();

            assert_eq!(register_response.username, username);
            assert_eq!(register_response.peer_id, peer_id);

            // Lookup username
            let lookup_response = client.lookup_username(&username).await.unwrap();

            assert_eq!(lookup_response.username, username);
            assert_eq!(lookup_response.peer_id, peer_id);
        }

        #[tokio::test]
        async fn test_update_prekeys() {
            let client = IdentityClient::new("http://localhost:8080").unwrap();
            let identity = Identity::generate(10);
            let username = format!("test_{}", rand::random::<u32>());
            let peer_id = format!("12D3KooW{}", rand::random::<u64>());

            // Register first
            client
                .register_username(&identity, &username, &peer_id)
                .await
                .unwrap();

            // Update prekeys
            let update_response = client.update_prekeys(&identity, &peer_id).await.unwrap();

            assert!(update_response.updated_at > chrono::Utc::now() - chrono::Duration::seconds(5));
        }

        #[tokio::test]
        async fn test_duplicate_username_error() {
            let client = IdentityClient::new("http://localhost:8080").unwrap();
            let identity1 = Identity::generate(10);
            let identity2 = Identity::generate(10);
            let username = format!("test_{}", rand::random::<u32>());
            let peer_id1 = format!("12D3KooW{}", rand::random::<u64>());
            let peer_id2 = format!("12D3KooW{}", rand::random::<u64>());

            // Register username with first identity
            client
                .register_username(&identity1, &username, &peer_id1)
                .await
                .unwrap();

            // Try to register same username with second identity (should fail)
            let result = client
                .register_username(&identity2, &username, &peer_id2)
                .await;

            assert!(result.is_err());
            let error = result.unwrap_err().to_string();
            assert!(error.contains("USERNAME_TAKEN"));
        }

        #[tokio::test]
        async fn test_health_check() {
            let client = IdentityClient::new("http://localhost:8080").unwrap();
            let health = client.health_check().await.unwrap();

            assert_eq!(health["status"], "healthy");
            assert!(health["database"]["status"] == "connected");
            assert!(health["redis"]["status"] == "connected");
        }
    }
}
