//! Unregister device token endpoint
//!
//! DELETE /api/v1/unregister
//! Body: { peer_id, device_id }

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, Method, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct UnregisterRequest {
    pub peer_id: String,
    pub device_id: String,
}

#[derive(Debug, Serialize)]
pub struct UnregisterResponse {
    pub success: bool,
    pub message: String,
}

/// Unregister (deactivate) a device token
///
/// Marks the token as inactive in the database.
/// The token is not deleted to maintain audit trail.
pub async fn handle(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<UnregisterResponse>, (StatusCode, String)> {
    let auth_peer =
        crate::auth::verify_peer_request(&headers, &Method::DELETE, "/api/v1/unregister", &body)
            .map_err(|(status, message)| (status, message.to_string()))?;
    let req: UnregisterRequest = serde_json::from_slice(&body)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid request body".to_string()))?;
    if req.peer_id != auth_peer {
        return Err((
            StatusCode::FORBIDDEN,
            "peer_id does not match identity".to_string(),
        ));
    }

    tracing::info!(
        "🗑️  Unregister request - peer_id: {}, device_id: {}",
        req.peer_id,
        req.device_id
    );

    // Mark token as inactive
    let result = sqlx::query(
        r#"
        UPDATE push_tokens
        SET is_active = false, last_used_at = NOW()
        WHERE peer_id = $1 AND device_id = $2
        "#,
    )
    .bind(&req.peer_id)
    .bind(&req.device_id)
    .execute(&state.db_pool)
    .await;

    match result {
        Ok(rows) if rows.rows_affected() > 0 => {
            tracing::info!(
                "✅ Token unregistered for peer {} device {}",
                req.peer_id,
                req.device_id
            );
            Ok(Json(UnregisterResponse {
                success: true,
                message: "Token unregistered successfully".to_string(),
            }))
        }
        Ok(_) => {
            tracing::warn!(
                "⚠️  No token found for peer {} device {}",
                req.peer_id,
                req.device_id
            );
            Ok(Json(UnregisterResponse {
                success: false,
                message: "Token not found".to_string(),
            }))
        }
        Err(e) => {
            tracing::error!("❌ Database error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to unregister token".to_string(),
            ))
        }
    }
}
