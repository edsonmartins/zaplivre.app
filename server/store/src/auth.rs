//! Autenticação por assinatura Ed25519 (SEC-09)
//!
//! O cliente assina `"{METHOD}:/api/store:{timestamp}"` com a chave de
//! identidade e envia:
//!   x-zaplivre-peer: peer id (libp2p, com a chave Ed25519 inline)
//!   x-zaplivre-ts:   unix timestamp (janela de 5 min)
//!   x-zaplivre-sig:  assinatura base64
//!
//! A chave pública é extraída do próprio peer ID (multihash identity),
//! então não há registro prévio: provar posse do peer ID é a autenticação.

use actix_web::HttpRequest;
use base64::{engine::general_purpose, Engine as _};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const WINDOW_SECONDS: i64 = 300;
const RATE_WINDOW_SECONDS: i64 = 60;
const MAX_REQUESTS_PER_PEER: usize = 120;
static USED_SIGNATURES: OnceLock<Mutex<HashMap<String, i64>>> = OnceLock::new();
static PEER_REQUESTS: OnceLock<Mutex<HashMap<String, Vec<i64>>>> = OnceLock::new();

pub struct AuthError(pub &'static str);

/// Verifica a assinatura da requisição e retorna o peer ID autenticado.
pub fn verify_request(req: &HttpRequest, body: &[u8]) -> Result<String, AuthError> {
    let peer = header(req, "x-zaplivre-peer").ok_or(AuthError("missing x-zaplivre-peer"))?;
    let ts: i64 = header(req, "x-zaplivre-ts")
        .and_then(|v| v.parse().ok())
        .ok_or(AuthError("missing or invalid x-zaplivre-ts"))?;
    let sig_b64 = header(req, "x-zaplivre-sig").ok_or(AuthError("missing x-zaplivre-sig"))?;

    let now = chrono::Utc::now().timestamp();
    if (now - ts).abs() > WINDOW_SECONDS {
        return Err(AuthError("timestamp outside allowed window"));
    }

    let verifying_key =
        public_key_from_peer_id(&peer).ok_or(AuthError("peer id has no inline ed25519 key"))?;

    let sig_bytes = general_purpose::STANDARD
        .decode(&sig_b64)
        .map_err(|_| AuthError("invalid signature encoding"))?;
    let sig_array: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| AuthError("invalid signature length"))?;
    let signature = ed25519_dalek::Signature::from_bytes(&sig_array);

    let body_hash = {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(body))
    };
    let message = format!(
        "{}\n{}\n{}\n{}",
        req.method().as_str(),
        "/api/store",
        ts,
        body_hash
    );

    use ed25519_dalek::Verifier;
    verifying_key
        .verify(message.as_bytes(), &signature)
        .map_err(|_| AuthError("invalid signature"))?;

    // Prevent replay and bound per-peer request volume. This is intentionally
    // process-local; production deployments should enforce the same policy at
    // a shared gateway/Redis layer across replicas.
    let used = USED_SIGNATURES.get_or_init(|| Mutex::new(HashMap::new()));
    let signature_key = sig_b64.clone();
    let mut used_guard = used
        .lock()
        .map_err(|_| AuthError("auth state unavailable"))?;
    used_guard.retain(|_, seen_at| now - *seen_at <= WINDOW_SECONDS);
    if used_guard.insert(signature_key, now).is_some() {
        return Err(AuthError("replayed signature"));
    }
    drop(used_guard);

    let requests = PEER_REQUESTS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut requests_guard = requests
        .lock()
        .map_err(|_| AuthError("auth state unavailable"))?;
    let history = requests_guard.entry(peer.clone()).or_default();
    history.retain(|seen_at| now - *seen_at < RATE_WINDOW_SECONDS);
    if history.len() >= MAX_REQUESTS_PER_PEER {
        return Err(AuthError("rate limit exceeded"));
    }
    history.push(now);

    Ok(peer)
}

fn header(req: &HttpRequest, name: &str) -> Option<String> {
    req.headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string())
}

/// Extrai a chave pública Ed25519 embutida num peer ID libp2p
/// (peer IDs Ed25519 usam multihash identity contendo a chave em protobuf)
fn public_key_from_peer_id(peer_id_str: &str) -> Option<ed25519_dalek::VerifyingKey> {
    let peer_id: libp2p_identity::PeerId = peer_id_str.parse().ok()?;
    let multihash = peer_id.as_ref();
    if multihash.code() != 0x00 {
        // Não é multihash identity (chave não embutida - ex.: RSA)
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
    fn test_public_key_extraction_from_known_peer_id() {
        // Peer ID Ed25519 real (dht1 do projeto)
        let peer = "12D3KooWJMY3dKygHLtkruLohCshiPENpJscD5XY33GjfcmS4DKK";
        assert!(public_key_from_peer_id(peer).is_some());
    }

    #[test]
    fn test_invalid_peer_id() {
        assert!(public_key_from_peer_id("not-a-peer-id").is_none());
    }
}
