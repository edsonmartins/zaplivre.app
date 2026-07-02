//! Firebase Cloud Messaging (FCM) client - HTTP v1 API
//!
//! A Legacy HTTP API (server key) foi desligada pelo Google em 2024.
//! Esta implementação usa a HTTP v1: autenticação OAuth2 com service
//! account (JWT RS256 -> access token, com cache) e envio via
//! `projects/{project_id}/messages:send`.
//!
//! Configuração: `FCM_SERVICE_ACCOUNT_PATH` apontando para o JSON do
//! service account do Firebase (Console > Project Settings > Service
//! accounts > Generate new private key).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Deserialize;
use tokio::sync::Mutex;

#[derive(Debug, Deserialize)]
struct ServiceAccount {
    project_id: String,
    private_key: String,
    client_email: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

struct CachedToken {
    token: String,
    expires_at: Instant,
}

/// FCM Client (HTTP v1)
pub struct FcmClient {
    http: reqwest::Client,
    service_account: ServiceAccount,
    encoding_key: jsonwebtoken::EncodingKey,
    cached_token: Arc<Mutex<Option<CachedToken>>>,
}

impl FcmClient {
    /// Cria o client a partir do JSON do service account
    pub fn from_service_account_file(
        path: &str,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read FCM service account {}: {}", path, e))?;
        let service_account: ServiceAccount = serde_json::from_str(&contents)
            .map_err(|e| format!("Invalid FCM service account JSON: {}", e))?;

        let encoding_key =
            jsonwebtoken::EncodingKey::from_rsa_pem(service_account.private_key.as_bytes())
                .map_err(|e| format!("Invalid FCM service account private key: {}", e))?;

        Ok(Self {
            http: reqwest::Client::new(),
            service_account,
            encoding_key,
            cached_token: Arc::new(Mutex::new(None)),
        })
    }

    pub fn project_id(&self) -> &str {
        &self.service_account.project_id
    }

    /// Obtém um access token OAuth2 (cacheado até ~1 min antes de expirar)
    async fn access_token(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        {
            let cached = self.cached_token.lock().await;
            if let Some(token) = cached.as_ref() {
                if token.expires_at > Instant::now() {
                    return Ok(token.token.clone());
                }
            }
        }

        #[derive(serde::Serialize)]
        struct Claims<'a> {
            iss: &'a str,
            scope: &'a str,
            aud: &'a str,
            iat: u64,
            exp: u64,
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        let claims = Claims {
            iss: &self.service_account.client_email,
            scope: "https://www.googleapis.com/auth/firebase.messaging",
            aud: "https://oauth2.googleapis.com/token",
            iat: now,
            exp: now + 3600,
        };

        let jwt = jsonwebtoken::encode(
            &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256),
            &claims,
            &self.encoding_key,
        )?;

        let response = self
            .http
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", jwt.as_str()),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("OAuth token exchange failed ({}): {}", status, body).into());
        }

        let token: TokenResponse = response.json().await?;
        let access_token = token.access_token.clone();

        let mut cached = self.cached_token.lock().await;
        *cached = Some(CachedToken {
            token: token.access_token,
            expires_at: Instant::now() + Duration::from_secs(token.expires_in.saturating_sub(60)),
        });

        Ok(access_token)
    }

    /// Send a push notification via FCM HTTP v1
    pub async fn send(
        &self,
        token: &str,
        title: &str,
        body: &str,
        data: &HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!(
            "  🔥 Sending FCM v1 notification - title: {}, body_len: {}",
            title,
            body.len()
        );

        let access_token = self.access_token().await?;

        let mut message = serde_json::json!({
            "message": {
                "token": token,
                "notification": {
                    "title": title,
                    "body": body,
                },
                "android": {
                    "priority": "high"
                }
            }
        });
        if !data.is_empty() {
            message["message"]["data"] = serde_json::to_value(data)?;
        }

        let url = format!(
            "https://fcm.googleapis.com/v1/projects/{}/messages:send",
            self.service_account.project_id
        );

        let response = self
            .http
            .post(&url)
            .bearer_auth(access_token)
            .json(&message)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("  ❌ FCM v1 error ({}): {}", status, body);
            return Err(format!("FCM v1 error ({}): {}", status, body).into());
        }

        tracing::debug!("  ✅ FCM v1 notification sent successfully");
        Ok(())
    }
}
