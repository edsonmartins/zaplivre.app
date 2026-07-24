//! Apple Push Notification Service (APNs) client
//!
//! Handles sending push notifications to iOS devices via APNs HTTP/2 API.
//!
//! # APNs Authentication
//! Uses token-based authentication with .p8 private key files (recommended by Apple).
//! JWT tokens are generated on-the-fly with 1-hour expiration.
//!
//! # References
//! - [APNs Provider API](https://developer.apple.com/documentation/usernotifications/setting_up_a_remote_notification_server/sending_notification_requests_to_apns)
//! - [Token-based Authentication](https://developer.apple.com/documentation/usernotifications/setting_up_a_remote_notification_server/establishing_a_token-based_connection_to_apns)

use anyhow::{Context, Result};
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::{Method, Request};
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

const APNS_PRODUCTION_ENDPOINT: &str = "https://api.push.apple.com";
const APNS_DEVELOPMENT_ENDPOINT: &str = "https://api.sandbox.push.apple.com";

/// JWT claims for APNs token authentication
#[derive(Debug, Serialize, Deserialize)]
struct ApnsJwtClaims {
    iss: String, // Team ID
    iat: u64,    // Issued at timestamp
}

/// APNs notification payload (JSON)
#[derive(Serialize)]
struct ApnsPayload {
    aps: Aps,
    #[serde(flatten)]
    custom: HashMap<String, String>,
}

#[derive(Serialize)]
struct Aps {
    alert: Alert,
    sound: String,
    #[serde(rename = "mutable-content")]
    mutable_content: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    badge: Option<u32>,
}

#[derive(Serialize)]
struct Alert {
    title: String,
    body: String,
}

/// APNs response from server
#[derive(Debug, Deserialize)]
struct ApnsResponse {
    reason: Option<String>,
    #[serde(rename = "apns-id")]
    #[allow(dead_code)]
    apns_id: Option<String>,
}

/// JWT token with expiration tracking
struct JwtToken {
    token: String,
    expires_at: SystemTime,
}

/// APNs Client for sending push notifications to iOS devices
pub struct ApnsClient {
    team_id: String,
    key_id: String,
    bundle_id: String,
    endpoint: String,
    encoding_key: EncodingKey,
    http_client: Client<
        hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
        Full<Bytes>,
    >,
    jwt_token: Arc<Mutex<Option<JwtToken>>>,
}

impl ApnsClient {
    /// Create a new APNs client with token-based authentication
    ///
    /// # Arguments
    /// * `key_path` - Path to .p8 private key file from Apple Developer account
    /// * `key_id` - Key ID (10 characters, e.g., "AB12CD34EF")
    /// * `team_id` - Team ID (10 characters, e.g., "XY98ZW76UV")
    /// * `bundle_id` - App bundle ID (e.g., "com.zaplivre.ios")
    /// * `production` - Use production APNs endpoint (true) or sandbox (false)
    pub fn new(
        key_path: &str,
        key_id: String,
        team_id: String,
        bundle_id: String,
        production: bool,
    ) -> Result<Self> {
        // Verify key file exists
        if !Path::new(key_path).exists() {
            anyhow::bail!("APNs private key file not found at: {}", key_path);
        }

        // Read .p8 private key file
        let key_pem = fs::read_to_string(key_path)
            .with_context(|| format!("Failed to read APNs private key from {}", key_path))?;

        // Create encoding key for JWT
        let encoding_key = EncodingKey::from_ec_pem(key_pem.as_bytes())
            .with_context(|| "Failed to parse APNs private key (must be EC P-256)")?;

        // Choose endpoint
        let endpoint = if production {
            APNS_PRODUCTION_ENDPOINT.to_string()
        } else {
            APNS_DEVELOPMENT_ENDPOINT.to_string()
        };

        // Create HTTPS connector with HTTP/2
        let https = HttpsConnectorBuilder::new()
            .with_native_roots()
            .expect("Failed to load native root certificates")
            .https_or_http()
            .enable_http2()
            .build();

        // Create HTTP client
        let http_client = Client::builder(TokioExecutor::new()).build(https);

        tracing::info!(
            "🍎 APNs client initialized - endpoint: {}, bundle: {}",
            endpoint,
            bundle_id
        );

        Ok(Self {
            team_id,
            key_id,
            bundle_id,
            endpoint,
            encoding_key,
            http_client,
            jwt_token: Arc::new(Mutex::new(None)),
        })
    }

    /// Generate or reuse JWT token for APNs authentication
    async fn get_jwt_token(&self) -> Result<String> {
        let mut token_guard = self.jwt_token.lock().await;

        // Check if we have a valid token
        if let Some(ref jwt) = *token_guard {
            if SystemTime::now() < jwt.expires_at {
                return Ok(jwt.token.clone());
            }
        }

        // Generate new token
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        let claims = ApnsJwtClaims {
            iss: self.team_id.clone(),
            iat: now,
        };

        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(self.key_id.clone());

        let token = encode(&header, &claims, &self.encoding_key)
            .with_context(|| "Failed to encode JWT token")?;

        // Token expires in 1 hour (but we'll refresh after 50 minutes)
        let expires_at = SystemTime::now() + Duration::from_secs(50 * 60);

        *token_guard = Some(JwtToken {
            token: token.clone(),
            expires_at,
        });

        Ok(token)
    }

    /// Send a push notification via APNs
    ///
    /// # Arguments
    /// * `device_token` - APNs device token (hex string, 64 characters)
    /// * `title` - Notification title
    /// * `body` - Notification body
    /// * `data` - Additional custom data (optional)
    /// * `badge` - Badge count (optional, None means don't update badge)
    pub async fn send(
        &self,
        device_token: &str,
        title: &str,
        body: &str,
        data: &HashMap<String, String>,
        badge: Option<u32>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!(
            "  🍎 Sending APNs notification - token: {}..., title: {}, body_len: {}",
            &device_token[..8],
            title,
            body.len()
        );

        // Build notification payload
        let payload = ApnsPayload {
            aps: Aps {
                alert: Alert {
                    title: title.to_string(),
                    body: body.to_string(),
                },
                sound: "default".to_string(),
                mutable_content: 1,
                badge,
            },
            custom: data.clone(),
        };

        // Serialize payload to JSON
        let payload_json = serde_json::to_string(&payload)?;

        // Get JWT token
        let jwt_token = self.get_jwt_token().await?;

        // Build APNs HTTP/2 request
        let url = format!("{}/3/device/{}", self.endpoint, device_token);

        let request = Request::builder()
            .method(Method::POST)
            .uri(&url)
            .header("authorization", format!("bearer {}", jwt_token))
            .header("apns-topic", &self.bundle_id)
            .header("apns-push-type", "alert")
            .header("apns-expiration", "0") // Immediate expiration if delivery fails
            .header("apns-priority", "10") // High priority
            .header("content-type", "application/json")
            .body(Full::new(Bytes::from(payload_json)))
            .map_err(|e| format!("Failed to build request: {}", e))?;

        // Send request
        let response = self
            .http_client
            .request(request)
            .await
            .map_err(|e| format!("APNs HTTP request failed: {}", e))?;

        let status = response.status();
        let apns_id = response
            .headers()
            .get("apns-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Read response body
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?
            .to_bytes();

        // Check response
        if status.is_success() {
            tracing::debug!(
                "  ✅ APNs notification sent successfully - apns_id: {}",
                apns_id
            );
            Ok(())
        } else {
            // Parse error response
            let error_msg = if !body_bytes.is_empty() {
                match serde_json::from_slice::<ApnsResponse>(&body_bytes) {
                    Ok(apns_resp) => {
                        format!(
                            "APNs error: {} (apns_id: {})",
                            apns_resp.reason.unwrap_or_else(|| "Unknown".to_string()),
                            apns_id
                        )
                    }
                    Err(_) => {
                        format!("APNs HTTP {} (apns_id: {})", status.as_u16(), apns_id)
                    }
                }
            } else {
                format!("APNs HTTP {} (apns_id: {})", status.as_u16(), apns_id)
            };

            tracing::error!("  ❌ {}", error_msg);
            Err(error_msg.into())
        }
    }

    /// Get APNs configuration info (for debugging)
    pub fn info(&self) -> String {
        format!(
            "APNs client - team_id: {}, key_id: {}, bundle_id: {}, endpoint: {}",
            self.team_id, self.key_id, self.bundle_id, self.endpoint
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_claims_serialization() {
        let claims = ApnsJwtClaims {
            iss: "TEAM123456".to_string(),
            iat: 1234567890,
        };

        let json = serde_json::to_string(&claims).unwrap();
        assert!(json.contains("TEAM123456"));
        assert!(json.contains("1234567890"));
    }

    #[test]
    fn test_payload_serialization() {
        let mut custom = HashMap::new();
        custom.insert("peer_id".to_string(), "12D3Koo...".to_string());

        let payload = ApnsPayload {
            aps: Aps {
                alert: Alert {
                    title: "New Message".to_string(),
                    body: "Hello from ZapLivre".to_string(),
                },
                sound: "default".to_string(),
                mutable_content: 1,
                badge: Some(5),
            },
            custom,
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("New Message"));
        assert!(json.contains("Hello from ZapLivre"));
        assert!(json.contains("peer_id"));
        assert!(json.contains("\"badge\":5"));
    }
}
