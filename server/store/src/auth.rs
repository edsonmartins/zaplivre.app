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

pub struct AuthError(pub &'static str);

/// Verifica a assinatura da requisição e retorna o peer ID autenticado.
pub fn verify_request(req: &HttpRequest) -> Result<String, AuthError> {
    let peer = header(req, "x-zaplivre-peer").ok_or(AuthError("missing x-zaplivre-peer"))?;
    let ts: i64 = header(req, "x-zaplivre-ts")
        .and_then(|v| v.parse().ok())
        .ok_or(AuthError("missing or invalid x-zaplivre-ts"))?;
    let sig_b64 = header(req, "x-zaplivre-sig").ok_or(AuthError("missing x-zaplivre-sig"))?;

    let now = chrono::Utc::now().timestamp();
    if (now - ts).abs() > 300 {
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

    let message = format!("{}:/api/store:{}", req.method().as_str(), ts);

    use ed25519_dalek::Verifier;
    verifying_key
        .verify(message.as_bytes(), &signature)
        .map_err(|_| AuthError("invalid signature"))?;

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
