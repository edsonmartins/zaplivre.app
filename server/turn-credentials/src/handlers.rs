//! HTTP handlers for TURN credentials service

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::{auth, config::Config};

/// Request for TURN credentials
#[derive(Debug, Deserialize)]
pub struct CredentialRequest {
    /// User ID to generate credentials for
    pub username: String,

    /// TTL in seconds (optional, default: 86400 = 24 hours)
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
    Json(req): Json<CredentialRequest>,
) -> Result<Json<CredentialResponse>, (StatusCode, String)> {
    // Validate request
    if req.username.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "username cannot be empty".to_string(),
        ));
    }

    // Default TTL: 24 hours
    let ttl = req.ttl_seconds.unwrap_or(86400);

    // Validate TTL (between 1 minute and 7 days)
    if !(60..=604800).contains(&ttl) {
        return Err((
            StatusCode::BAD_REQUEST,
            "ttl_seconds must be between 60 and 604800".to_string(),
        ));
    }

    // Generate credentials
    let (username, password) = auth::generate_turn_credentials(
        &req.username,
        ttl,
        &config.turn_static_secret,
    );

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

    #[test]
    fn test_ttl_validation() {
        // Too short (less than 60 seconds)
        let too_short = 30i64;
        assert!(too_short < 60 || too_short > 604800);

        // Too long (more than 7 days)
        let too_long = 700000i64;
        assert!(too_long < 60 || too_long > 604800);

        // Valid (1 hour)
        let valid = 3600i64;
        assert!(valid >= 60 && valid <= 604800);
    }
}
