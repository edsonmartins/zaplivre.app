//! API handlers for Identity Server

use axum::{
    extract::{Query, State},
    Json,
};
use base64::{engine::general_purpose, Engine as _};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    db,
    error::{AppError, Result},
    models::*,
    AppState,
};

/// Register a new username
pub async fn register_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>> {
    // Decode public key from base64
    let public_key = general_purpose::STANDARD
        .decode(&req.public_key)
        .map_err(|_| AppError::InvalidSignature)?;

    // SEC-14: a assinatura cobre username + peer_id + public_key + timestamp,
    // impedindo o replay da mesma assinatura com outro peer_id/bundle
    check_timestamp(req.timestamp)?;
    let message = format!(
        "register:{}:{}:{}:{}",
        req.username, req.peer_id, req.public_key, req.timestamp
    );
    verify_signature(&public_key, &req.signature, &message)?;

    // Register username
    let response = db::register_username(
        &state.db,
        &req.username,
        &req.peer_id,
        &public_key,
        &req.prekey_bundle,
    )
    .await?;

    Ok(Json(response))
}

/// Lookup username query parameters
#[derive(Debug, Deserialize)]
pub struct LookupQuery {
    pub username: Option<String>,
    pub peer_id: Option<String>,
}

/// Lookup a username
pub async fn lookup_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LookupQuery>,
) -> Result<Json<LookupResponse>> {
    let response = match (query.username.as_deref(), query.peer_id.as_deref()) {
        (Some(username), None) => db::lookup_username(&state.db, username).await?,
        (None, Some(peer_id)) => db::lookup_peer_id(&state.db, peer_id).await?,
        _ => return Err(AppError::UsernameNotFound("missing lookup key".to_string())),
    };
    Ok(Json(response))
}

/// Update prekeys for a username
pub async fn update_prekeys_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdatePrekeysRequest>,
) -> Result<Json<UpdatePrekeysResponse>> {
    // SEC-10: verificar a assinatura contra a chave pública REGISTRADA do
    // peer - sem isso qualquer um substituía o prekey bundle de qualquer
    // usuário (vetor de MITM no X3DH)
    check_timestamp(req.timestamp)?;

    let public_key = db::get_public_key_by_peer_id(&state.db, &req.peer_id)
        .await?
        .ok_or_else(|| AppError::UsernameNotFound(req.peer_id.clone()))?;

    let message = format!("update_prekeys:{}:{}", req.peer_id, req.timestamp);
    verify_signature(&public_key, &req.signature, &message)?;

    let response = db::update_prekeys(&state.db, &req.peer_id, &req.prekey_bundle).await?;
    Ok(Json(response))
}

/// Health check endpoint
pub async fn health_handler(State(state): State<Arc<AppState>>) -> Result<Json<HealthResponse>> {
    let start = std::time::Instant::now();

    // Check database
    let db_latency = db::check_health(&state.db).await?;

    // Check Redis
    let redis_latency = check_redis_health(&state.redis).await?;

    let uptime_seconds = start.elapsed().as_secs();

    Ok(Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds,
        database: HealthStatus {
            status: "connected".to_string(),
            latency_ms: db_latency,
        },
        redis: HealthStatus {
            status: "connected".to_string(),
            latency_ms: redis_latency,
        },
        timestamp: chrono::Utc::now(),
    }))
}

/// Check request timestamp freshness (anti-replay window of 5 minutes).
/// Roda ANTES da verificação de assinatura (barato primeiro).
fn check_timestamp(timestamp: i64) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    if (now - timestamp).abs() > 300 {
        return Err(AppError::InvalidSignature);
    }
    Ok(())
}

/// Verify Ed25519 signature over a canonical message.
/// Erros de decodificação/verificação retornam 400 (InvalidSignature), não 500.
fn verify_signature(public_key: &[u8], signature_b64: &str, message: &str) -> Result<()> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    let signature_bytes = general_purpose::STANDARD
        .decode(signature_b64)
        .map_err(|_| AppError::InvalidSignature)?;

    let signature_array: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| AppError::InvalidSignature)?;

    let signature = Signature::from_bytes(&signature_array);

    let public_key_array: [u8; 32] = public_key
        .try_into()
        .map_err(|_| AppError::InvalidSignature)?;

    let verifying_key =
        VerifyingKey::from_bytes(&public_key_array).map_err(|_| AppError::InvalidSignature)?;

    verifying_key
        .verify(message.as_bytes(), &signature)
        .map_err(|_| AppError::InvalidSignature)?;

    Ok(())
}

/// Check Redis health
async fn check_redis_health(redis: &redis::aio::ConnectionManager) -> Result<f64> {
    use redis::AsyncCommands;

    let start = std::time::Instant::now();

    let mut conn = redis.clone();
    // Use a simple GET/SET command to check Redis health
    let _: () = conn
        .set("health_check", "ok")
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Redis health check failed: {}", e)))?;

    let latency = start.elapsed().as_secs_f64() * 1000.0; // Convert to ms
    Ok(latency)
}
