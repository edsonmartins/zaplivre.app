//! Register device token endpoint
//!
//! POST /api/v1/register
//! Body: { peer_id, platform, device_id, token, device_name?, app_version? }

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub peer_id: String,
    pub platform: String, // "fcm" or "apns"
    pub device_id: String,
    pub token: String,
    pub device_name: Option<String>,
    pub app_version: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub success: bool,
    pub message: String,
}

/// Register or update a device token
///
/// Stores the FCM/APNs token in the database associated with the peer_id and device_id.
/// If the token already exists, it updates it and marks as active.
pub async fn handle(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, (StatusCode, String)> {
    let platform = req.platform.trim().to_lowercase();

    tracing::info!(
        "📝 Register request - peer_id: {}, platform: {}, device_id: {}",
        req.peer_id,
        platform,
        req.device_id
    );

    // Validate platform
    if platform != "fcm" && platform != "apns" {
        tracing::warn!("❌ Invalid platform: {}", platform);
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Invalid platform: {}. Must be 'fcm' or 'apns'", platform),
        ));
    }

    // Insert or update token in database
    let device_name = req.device_name.unwrap_or_else(|| "Unknown".to_string());
    let app_version = req.app_version.unwrap_or_else(|| "0.1.0".to_string());
    let token = if platform == "apns" {
        sanitize_apns_token(&req.token)
    } else {
        req.token.trim().to_string()
    };

    let result = sqlx::query(
        r#"
        INSERT INTO push_tokens (peer_id, platform, device_id, token, device_name, app_version)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (peer_id, device_id)
        DO UPDATE SET
            token = EXCLUDED.token,
            platform = EXCLUDED.platform,
            device_name = EXCLUDED.device_name,
            app_version = EXCLUDED.app_version,
            last_used_at = NOW(),
            is_active = true
        "#,
    )
    .bind(&req.peer_id)
    .bind(&platform)
    .bind(&req.device_id)
    .bind(&token)
    .bind(&device_name)
    .bind(&app_version)
    .execute(&state.db_pool)
    .await;

    match result {
        Ok(_) => {
            tracing::info!(
                "✅ Token registered successfully for peer {} device {}",
                req.peer_id,
                req.device_id
            );
            Ok(Json(RegisterResponse {
                success: true,
                message: "Token registered successfully".to_string(),
            }))
        }
        Err(e) => {
            tracing::error!("❌ Database error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to register token: {}", e),
            ))
        }
    }
}

fn sanitize_apns_token(token: &str) -> String {
    token
        .trim()
        .trim_matches('<')
        .trim_matches('>')
        .replace(' ', "")
        .replace('\n', "")
        .replace('\t', "")
}

#[cfg(test)]
mod tests {
    use super::sanitize_apns_token;

    #[test]
    fn test_sanitize_apns_token() {
        let raw = " <abc def\n123\t> ";
        assert_eq!(sanitize_apns_token(raw), "abcdef123");
    }
}
