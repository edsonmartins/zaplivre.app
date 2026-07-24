//! HTTP handlers for TURN credentials service

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, Method, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{auth, config::Config};

/// Request for TURN credentials
#[derive(Debug, Deserialize)]
pub struct CredentialRequest {
    /// User ID to generate credentials for
    pub username: String,

    /// Deprecated. Credential lifetime is controlled by the server.
    #[allow(dead_code)]
    pub ttl_seconds: Option<i64>,
}

/// Response with TURN credentials
#[derive(Debug, Serialize)]
pub struct CredentialResponse {
    /// TURN username (timestamp:user_id)
    pub username: String,

    /// TURN password (HMAC-SHA1)
    pub password: String,

    /// TURN server URIs
    pub uris: Vec<String>,

    /// Time-to-live in seconds
    pub ttl: i64,
}

/// Generate TURN credentials endpoint
///
/// POST /api/turn/credentials
/// Body: { "username": "user123", "ttl_seconds": 3600 }
///
/// Returns credentials valid for the specified TTL
pub async fn generate_credentials(
    State(config): State<Config>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<CredentialResponse>, (StatusCode, String)> {
    let auth_peer =
        crate::request_auth::verify(&headers, &Method::POST, "/api/turn/credentials", &body)
            .map_err(|(status, message)| (status, message.to_string()))?;
    let req: CredentialRequest = serde_json::from_slice(&body)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid request body".to_string()))?;

    // Validate request
    if req.username.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "username cannot be empty".to_string(),
        ));
    }

    if req.username != auth_peer {
        return Err((
            StatusCode::FORBIDDEN,
            "username does not match identity".to_string(),
        ));
    }
    let ttl = config.credential_ttl_seconds;

    // Generate credentials
    let (username, password) =
        auth::generate_turn_credentials(&req.username, ttl, &config.turn_static_secret);

    tracing::info!(
        "Generated TURN credentials for user '{}' (TTL: {}s)",
        req.username,
        ttl
    );

    Ok(Json(CredentialResponse {
        username,
        password,
        uris: config.turn_uris.clone(),
        ttl,
    }))
}

/// Health check endpoint
///
/// GET /health
pub async fn health_check() -> &'static str {
    "OK"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_request_validation() {
        // Valid request
        let req = CredentialRequest {
            username: "user123".to_string(),
            ttl_seconds: Some(3600),
        };
        assert!(!req.username.is_empty());

        // Empty username should fail
        let req = CredentialRequest {
            username: "".to_string(),
            ttl_seconds: Some(3600),
        };
        assert!(req.username.is_empty());
    }
}
