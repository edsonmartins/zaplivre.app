//! Send push notification endpoint
//!
//! POST /api/v1/send
//! Body: { peer_id, title, body, data? }

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct SendRequest {
    pub peer_id: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub data: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct SendResponse {
    pub success: bool,
    pub sent_count: usize,
    pub failed_count: usize,
    pub message: String,
}

/// Send push notification to all devices of a peer
///
/// Retrieves all active tokens for the given peer_id and sends
/// push notifications via FCM or APNs depending on the platform.
pub async fn handle(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SendRequest>,
) -> Result<Json<SendResponse>, (StatusCode, String)> {
    crate::auth::verify_service_request(&headers, &state.service_secret)
        .map_err(|(status, message)| (status, message.to_string()))?;

    tracing::info!(
        "📤 Send notification request - peer_id: {}, title: {}",
        req.peer_id,
        req.title
    );

    let mut data = req.data.clone();
    data.entry("peer_id".to_string())
        .or_insert_with(|| req.peer_id.clone());

    // Get all active tokens for this peer
    let tokens = sqlx::query_as::<_, (String, String, String)>(
        r#"
        SELECT token, platform, device_id
        FROM push_tokens
        WHERE peer_id = $1 AND is_active = true
        "#,
    )
    .bind(&req.peer_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("❌ Database error: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to fetch tokens".to_string(),
        )
    })?;

    if tokens.is_empty() {
        tracing::warn!("⚠️  No active tokens found for peer {}", req.peer_id);
        return Ok(Json(SendResponse {
            success: true,
            sent_count: 0,
            failed_count: 0,
            message: "No active tokens found for this peer".to_string(),
        }));
    }

    tracing::info!("📱 Found {} active device(s)", tokens.len());

    let mut sent_count = 0;
    let mut failed_count = 0;

    // Send notification to each device
    for token_row in tokens {
        let token = &token_row.0;
        let platform = &token_row.1;
        let device_id = &token_row.2;

        tracing::debug!("  Sending to device {} ({})", device_id, platform);

        match platform.as_str() {
            "fcm" => {
                // Send via FCM (HTTP v1) - opcional como o APNs
                let Some(fcm_client) = state.fcm_client.as_ref() else {
                    tracing::warn!("  ⚠️ FCM not configured, skipping device {}", device_id);
                    failed_count += 1;
                    continue;
                };
                match fcm_client.send(token, &req.title, &req.body, &data).await {
                    Ok(_) => {
                        tracing::info!("  ✅ FCM notification sent to {}", device_id);
                        sent_count += 1;

                        // Update last_used_at
                        let _ = sqlx::query(
                            "UPDATE push_tokens SET last_used_at = NOW() WHERE peer_id = $1 AND device_id = $2"
                        )
                        .bind(&req.peer_id)
                        .bind(device_id)
                        .execute(&state.db_pool)
                        .await;
                    }
                    Err(e) => {
                        tracing::error!("  ❌ FCM failed for {}: {}", device_id, e);
                        failed_count += 1;

                        // Mark as inactive if token is invalid
                        if fcm_token_is_dead(&e.to_string()) {
                            tracing::warn!("  🔄 Marking token as inactive for {}", device_id);
                            let _ = sqlx::query(
                                "UPDATE push_tokens SET is_active = false WHERE peer_id = $1 AND device_id = $2"
                            )
                            .bind(&req.peer_id)
                            .bind(device_id)
                            .execute(&state.db_pool)
                            .await;
                        }
                    }
                }
            }
            "apns" => {
                // Send via APNs
                match &state.apns_client {
                    Some(apns_client) => {
                        match apns_client
                            .send(token, &req.title, &req.body, &data, Some(1))
                            .await
                        {
                            Ok(_) => {
                                tracing::info!("  ✅ APNs notification sent to {}", device_id);
                                sent_count += 1;

                                // Update last_used_at
                                let _ = sqlx::query(
                                    "UPDATE push_tokens SET last_used_at = NOW() WHERE peer_id = $1 AND device_id = $2"
                                )
                                .bind(&req.peer_id)
                                .bind(device_id)
                                .execute(&state.db_pool)
                                .await;
                            }
                            Err(e) => {
                                tracing::error!("  ❌ APNs failed for {}: {}", device_id, e);
                                failed_count += 1;

                                // Mark as inactive if token is invalid
                                if apns_token_is_dead(&e.to_string()) {
                                    tracing::warn!(
                                        "  🔄 Marking token as inactive for {}",
                                        device_id
                                    );
                                    let _ = sqlx::query(
                                        "UPDATE push_tokens SET is_active = false WHERE peer_id = $1 AND device_id = $2"
                                    )
                                    .bind(&req.peer_id)
                                    .bind(device_id)
                                    .execute(&state.db_pool)
                                    .await;
                                }
                            }
                        }
                    }
                    None => {
                        tracing::warn!(
                            "  ⚠️  APNs client not configured - cannot send to {}",
                            device_id
                        );
                        failed_count += 1;
                    }
                }
            }
            _ => {
                tracing::error!("  ❌ Unknown platform: {}", platform);
                failed_count += 1;
            }
        }
    }

    let message = format!(
        "Sent {} notification(s), {} failed",
        sent_count, failed_count
    );

    tracing::info!("✅ {}", message);

    Ok(Json(SendResponse {
        success: sent_count > 0,
        sent_count,
        failed_count,
        message,
    }))
}

/// Whether an FCM HTTP v1 error means the device token no longer exists.
///
/// v1 reports `UNREGISTERED` (HTTP 404). The legacy API strings
/// (`NotRegistered`, `InvalidRegistration`) never appear in v1 responses, so
/// dead tokens were never retired. `INVALID_ARGUMENT` is deliberately not
/// included: it is also returned for a malformed payload, which is our bug.
fn fcm_token_is_dead(error: &str) -> bool {
    error.contains("UNREGISTERED")
}

/// Whether an APNs rejection means the device token is no longer valid.
///
/// `InvalidProviderToken` / `ExpiredProviderToken` are about OUR signing key,
/// not the device: treating them as dead tokens deactivated every iOS user on
/// the first send after a bad `.p8` was deployed.
fn apns_token_is_dead(error: &str) -> bool {
    error.contains("BadDeviceToken") || error.contains("Unregistered")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_errors_do_not_retire_device_tokens() {
        assert!(apns_token_is_dead("APNs error 410: Unregistered"));
        assert!(apns_token_is_dead("APNs error 400: BadDeviceToken"));
        assert!(!apns_token_is_dead("APNs error 403: InvalidProviderToken"));
        assert!(!apns_token_is_dead("APNs error 403: ExpiredProviderToken"));
    }

    #[test]
    fn fcm_v1_unregistered_retires_the_token() {
        let body = r#"FCM v1 error (404 Not Found): {"error":{"status":"NOT_FOUND","details":[{"errorCode":"UNREGISTERED"}]}}"#;
        assert!(fcm_token_is_dead(body));
        assert!(!fcm_token_is_dead(
            r#"FCM v1 error (400 Bad Request): {"error":{"status":"INVALID_ARGUMENT"}}"#
        ));
        assert!(!fcm_token_is_dead("FCM v1 error (401 Unauthorized): ..."));
    }
}
