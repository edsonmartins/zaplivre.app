use axum::http::{HeaderMap, Method, StatusCode};
use base64::{engine::general_purpose, Engine as _};
use ed25519_dalek::Verifier;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const AUTH_WINDOW_SECONDS: i64 = 300;
const RATE_WINDOW_SECONDS: i64 = 60;
const MAX_REQUESTS_PER_PEER: u32 = 30;
static USED_SIGNATURES: OnceLock<Mutex<HashMap<String, i64>>> = OnceLock::new();
static PEER_REQUESTS: OnceLock<Mutex<HashMap<String, (i64, u32)>>> = OnceLock::new();

pub fn verify(
    headers: &HeaderMap,
    method: &Method,
    path: &str,
    body: &[u8],
) -> Result<String, (StatusCode, &'static str)> {
    let peer = header(headers, "x-zaplivre-peer")
        .ok_or((StatusCode::UNAUTHORIZED, "missing authentication"))?;
    let timestamp: i64 = header(headers, "x-zaplivre-ts")
        .and_then(|value| value.parse().ok())
        .ok_or((StatusCode::UNAUTHORIZED, "missing authentication"))?;
    let signature = header(headers, "x-zaplivre-sig")
        .ok_or((StatusCode::UNAUTHORIZED, "missing authentication"))?;

    if (chrono::Utc::now().timestamp() - timestamp).abs() > AUTH_WINDOW_SECONDS {
        return Err((StatusCode::UNAUTHORIZED, "expired authentication"));
    }

    let verifying_key = public_key_from_peer_id(peer)
        .ok_or((StatusCode::UNAUTHORIZED, "invalid authentication"))?;
    let signature_bytes = general_purpose::STANDARD
        .decode(signature)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid authentication"))?;
    let signature_array: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid authentication"))?;
    let signature = ed25519_dalek::Signature::from_bytes(&signature_array);

    let body_hash = hex::encode(Sha256::digest(body));
    let canonical = format!(
        "{}\n{}\n{}\n{}",
        method.as_str().to_ascii_uppercase(),
        path,
        timestamp,
        body_hash
    );
    verifying_key
        .verify(canonical.as_bytes(), &signature)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid authentication"))?;
    if !check_peer_rate(peer, chrono::Utc::now().timestamp()) {
        return Err((StatusCode::TOO_MANY_REQUESTS, "rate limit exceeded"));
    }

    if !consume_signature(
        signature.to_bytes().as_ref(),
        chrono::Utc::now().timestamp(),
    ) {
        return Err((StatusCode::UNAUTHORIZED, "replayed authentication"));
    }

    Ok(peer.to_string())
}

fn consume_signature(signature: &[u8], now: i64) -> bool {
    let cache = USED_SIGNATURES.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    cache.retain(|_, expires_at| *expires_at > now);
    let key = hex::encode(signature);
    if cache.contains_key(&key) {
        return false;
    }
    cache.insert(key, now + AUTH_WINDOW_SECONDS);
    true
}

fn check_peer_rate(peer: &str, now: i64) -> bool {
    let cache = PEER_REQUESTS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    cache.retain(|_, (started_at, _)| now - *started_at < RATE_WINDOW_SECONDS);
    let entry = cache.entry(peer.to_string()).or_insert((now, 0));
    if now - entry.0 >= RATE_WINDOW_SECONDS {
        *entry = (now, 0);
    }
    entry.1 += 1;
    entry.1 <= MAX_REQUESTS_PER_PEER
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

fn public_key_from_peer_id(peer_id_str: &str) -> Option<ed25519_dalek::VerifyingKey> {
    let peer_id: libp2p_identity::PeerId = peer_id_str.parse().ok()?;
    let multihash = peer_id.as_ref();
    if multihash.code() != 0x00 {
        return None;
    }
    let public_key = libp2p_identity::PublicKey::try_decode_protobuf(multihash.digest()).ok()?;
    let ed25519 = public_key.try_into_ed25519().ok()?;
    ed25519_dalek::VerifyingKey::from_bytes(&ed25519.to_bytes()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_missing_authentication() {
        let result = verify(
            &HeaderMap::new(),
            &Method::POST,
            "/api/turn/credentials",
            b"{}",
        );
        assert_eq!(result.unwrap_err().0, StatusCode::UNAUTHORIZED);
    }
}
