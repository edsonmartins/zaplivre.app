//! API handlers for Identity Server

use axum::{
    extract::{Path, Query, State},
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

    // The peer ID must be the one derived from the key that signed. Accepting
    // any string let an attacker register a victim's (public) peer ID under
    // their own key: the victim could never register again, and lookups by
    // peer ID returned the attacker's key and bundle.
    verify_peer_id_matches_key(&req.peer_id, &public_key)?;
    verify_bundle_binding(&req.prekey_bundle, &public_key)?;

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

#[derive(Debug, Deserialize)]
pub struct TransparencyLogQuery {
    pub from: Option<i64>,
    pub limit: Option<i64>,
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
    // The request signature only covers peer_id + timestamp, so a captured
    // request could be replayed with another bundle. The bundle must carry its
    // own proof of belonging to the registered key.
    verify_bundle_binding(&req.prekey_bundle, &public_key)?;

    let response = db::update_prekeys(&state.db, &req.peer_id, &req.prekey_bundle).await?;
    Ok(Json(response))
}

/// Fetch an independently auditable proof for a peer's current identity key.
pub async fn transparency_handler(
    State(state): State<Arc<AppState>>,
    Path(peer_id): Path<String>,
) -> Result<Json<TransparencyResponse>> {
    Ok(Json(db::transparency_for_peer(&state.db, &peer_id).await?))
}

/// Return an ordered segment of the log for an independent auditor.
pub async fn transparency_log_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TransparencyLogQuery>,
) -> Result<Json<Vec<TransparencyLogEntry>>> {
    Ok(Json(
        db::transparency_log_segment(
            &state.db,
            query.from.unwrap_or(1),
            query.limit.unwrap_or(100),
        )
        .await?,
    ))
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
/// Check that `peer_id` is the libp2p peer ID of the Ed25519 `public_key`.
fn verify_peer_id_matches_key(peer_id: &str, public_key: &[u8]) -> Result<()> {
    let expected = libp2p_identity::ed25519::PublicKey::try_from_bytes(public_key)
        .map(|key| libp2p_identity::PublicKey::from(key).to_peer_id())
        .map_err(|_| AppError::InvalidSignature)?;
    let claimed: libp2p_identity::PeerId = peer_id.parse().map_err(|_| AppError::PeerIdMismatch)?;
    if claimed != expected {
        return Err(AppError::PeerIdMismatch);
    }
    Ok(())
}

/// Check that the bundle belongs to `public_key`: same Ed25519 identity key,
/// and the Signal identity key signed by it (clients enforce the same rule and
/// refuse bundles without it, so the server must not publish one).
fn verify_bundle_binding(bundle: &crate::models::PreKeyBundle, public_key: &[u8]) -> Result<()> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    let decode = |value: &str| general_purpose::STANDARD.decode(value);
    let identity_key = decode(&bundle.identity_key)
        .map_err(|_| AppError::InvalidPrekeyBundle("identity_key is not base64"))?;
    if identity_key != public_key {
        return Err(AppError::InvalidPrekeyBundle(
            "identity_key differs from the registered public key",
        ));
    }
    let signal_identity_key = bundle
        .signal_identity_key
        .as_deref()
        .and_then(|value| decode(value).ok())
        .ok_or(AppError::InvalidPrekeyBundle("missing signal_identity_key"))?;
    let signature: [u8; 64] = bundle
        .signal_identity_signature
        .as_deref()
        .and_then(|value| decode(value).ok())
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or(AppError::InvalidPrekeyBundle(
            "missing signal_identity_signature",
        ))?;

    let mut message = b"zaplivre-signal-identity-binding-v1\0".to_vec();
    message.extend_from_slice(&signal_identity_key);

    let key_bytes: [u8; 32] = public_key
        .try_into()
        .map_err(|_| AppError::InvalidSignature)?;
    VerifyingKey::from_bytes(&key_bytes)
        .map_err(|_| AppError::InvalidSignature)?
        .verify(&message, &Signature::from_bytes(&signature))
        .map_err(|_| {
            AppError::InvalidPrekeyBundle("signal identity key is not signed by the identity key")
        })
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn keypair(seed: u8) -> (Vec<u8>, String) {
        let public = ed25519_dalek::SigningKey::from_bytes(&[seed; 32])
            .verifying_key()
            .to_bytes();
        let peer_id = libp2p_identity::PublicKey::from(
            libp2p_identity::ed25519::PublicKey::try_from_bytes(&public).unwrap(),
        )
        .to_peer_id();
        (public.to_vec(), peer_id.to_string())
    }

    fn bundle(seed: u8, signal_identity: &[u8], signed_by: u8) -> crate::models::PreKeyBundle {
        use ed25519_dalek::Signer;
        let mut message = b"zaplivre-signal-identity-binding-v1\0".to_vec();
        message.extend_from_slice(signal_identity);
        let signature = ed25519_dalek::SigningKey::from_bytes(&[signed_by; 32]).sign(&message);
        let b64 = |bytes: &[u8]| general_purpose::STANDARD.encode(bytes);
        crate::models::PreKeyBundle {
            identity_key: b64(&keypair(seed).0),
            signal_identity_key: Some(b64(signal_identity)),
            signal_identity_signature: Some(b64(&signature.to_bytes())),
            signal_registration_id: Some(1),
            signal_device_id: Some(1),
            signed_prekey_id: 1,
            signed_prekey: b64(&[1; 33]),
            signed_prekey_signature: b64(&[2; 64]),
            kyber_prekey_id: 1,
            kyber_prekey: b64(&[3; 32]),
            kyber_prekey_signature: b64(&[4; 64]),
            one_time_prekey: None,
        }
    }

    #[test]
    fn bundle_must_be_bound_to_the_registered_key() {
        let (alice_key, _) = keypair(1);

        assert!(verify_bundle_binding(&bundle(1, b"alice-signal", 1), &alice_key).is_ok());
        // Signal identity vouched for by someone else's key.
        assert!(verify_bundle_binding(&bundle(1, b"mallory-signal", 2), &alice_key).is_err());
        // Bundle of another identity altogether.
        assert!(verify_bundle_binding(&bundle(2, b"mallory-signal", 2), &alice_key).is_err());
        let mut unsigned = bundle(1, b"alice-signal", 1);
        unsigned.signal_identity_signature = None;
        assert!(verify_bundle_binding(&unsigned, &alice_key).is_err());
    }

    #[test]
    fn peer_id_must_be_derived_from_the_registering_key() {
        let (alice_key, alice_peer) = keypair(1);
        let (mallory_key, _) = keypair(2);

        assert!(verify_peer_id_matches_key(&alice_peer, &alice_key).is_ok());
        // Squatting: Mallory's key claiming Alice's peer ID.
        assert!(matches!(
            verify_peer_id_matches_key(&alice_peer, &mallory_key),
            Err(AppError::PeerIdMismatch)
        ));
        assert!(matches!(
            verify_peer_id_matches_key("12D3KooWnotapeer", &alice_key),
            Err(AppError::PeerIdMismatch)
        ));
    }
}
