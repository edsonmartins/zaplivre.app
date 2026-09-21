//! End-to-end encrypted transfer of media too large to travel inline.
//!
//! The file is sealed with AES-256-GCM under a random per-file key. Only the
//! sealed blob crosses the chunk protocol; the key travels in a
//! [`MediaOfferEnvelope`] inside the Signal session, like any text message.
//! The media is identified by the SHA-256 of the *sealed* blob, so whoever
//! relays or observes chunks learns neither the content nor its plaintext hash.

use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::utils::error::{Result, ZapLivreError};

pub const MEDIA_OFFER_PREFIX: &str = "MP_MEDIA_OFFER_V1:";

/// Bytes a sealed blob adds to the plaintext (the GCM tag).
pub const SEAL_OVERHEAD: usize = 16;

/// Domain separation: a media blob can never be opened as anything else.
const SEAL_AAD: &[u8] = b"zaplivre-media-transfer-v1";

/// Offer of a sealed media file, carried inside the Signal session.
#[derive(Clone, Serialize, Deserialize)]
pub struct MediaOfferEnvelope {
    pub version: u8,
    pub media_type: String,
    /// SHA-256 (hex) of the sealed blob; this is what chunks are requested by.
    pub media_hash: String,
    pub file_key_b64: String,
    pub nonce_b64: String,
    pub file_name: Option<String>,
    pub mime_type: Option<String>,
    /// Size of the plaintext file.
    pub file_size: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration_seconds: Option<i32>,
    pub thumbnail_b64: Option<String>,
}

// Manual impl: the envelope holds the file key.
impl std::fmt::Debug for MediaOfferEnvelope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MediaOfferEnvelope")
            .field("media_type", &self.media_type)
            .field("media_hash", &self.media_hash)
            .field("file_key_b64", &"<redacted>")
            .field("file_size", &self.file_size)
            .finish_non_exhaustive()
    }
}

impl MediaOfferEnvelope {
    pub fn encode(&self) -> Result<String> {
        let json = serde_json::to_string(self)
            .map_err(|e| ZapLivreError::Protocol(format!("Failed to encode media offer: {}", e)))?;
        Ok(format!("{}{}", MEDIA_OFFER_PREFIX, json))
    }

    pub fn decode(input: &str) -> Option<Self> {
        let json = input.strip_prefix(MEDIA_OFFER_PREFIX)?;
        serde_json::from_str(json).ok()
    }

    /// Size of the sealed blob the receiver should expect.
    pub fn sealed_size(&self) -> Option<u64> {
        u64::try_from(self.file_size)
            .ok()?
            .checked_add(SEAL_OVERHEAD as u64)
    }

    pub fn thumbnail_bytes(&self) -> Result<Option<Vec<u8>>> {
        self.thumbnail_b64
            .as_deref()
            .map(|b64| {
                general_purpose::STANDARD.decode(b64).map_err(|e| {
                    ZapLivreError::Protocol(format!("Invalid thumbnail base64: {}", e))
                })
            })
            .transpose()
    }

    fn key_and_nonce(&self) -> Result<([u8; 32], [u8; 12])> {
        let invalid = || ZapLivreError::Protocol("Invalid media offer key material".to_string());
        let key = general_purpose::STANDARD
            .decode(&self.file_key_b64)
            .ok()
            .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
            .ok_or_else(invalid)?;
        let nonce = general_purpose::STANDARD
            .decode(&self.nonce_b64)
            .ok()
            .and_then(|bytes| <[u8; 12]>::try_from(bytes).ok())
            .ok_or_else(invalid)?;
        Ok((key, nonce))
    }

    /// Re-create the sealed blob from the local plaintext file. Sealing is
    /// deterministic for a given key and nonce, so the sender serves chunks
    /// without keeping a second, sealed copy of every file on disk.
    pub fn seal(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let (key, nonce) = self.key_and_nonce()?;
        seal_with(&key, &nonce, plaintext)
    }

    /// Verify and open a fully reassembled sealed blob.
    pub fn open(&self, sealed: &[u8]) -> Result<Vec<u8>> {
        if sealed_hash(sealed) != self.media_hash {
            return Err(ZapLivreError::Crypto(
                "Media integrity verification failed - sealed blob hash mismatch".to_string(),
            ));
        }
        let (key, nonce) = self.key_and_nonce()?;
        Aes256Gcm::new((&key).into())
            .decrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: sealed,
                    aad: SEAL_AAD,
                },
            )
            .map_err(|_| {
                ZapLivreError::Crypto("Media integrity verification failed - cannot open".into())
            })
    }
}

/// Metadata of a media file about to be offered.
pub struct MediaOfferMeta<'a> {
    pub media_type: &'a str,
    pub file_name: Option<String>,
    pub mime_type: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration_seconds: Option<i32>,
    pub thumbnail: Option<&'a [u8]>,
}

/// Seal `plaintext` under a fresh random key and describe it in an offer.
pub fn new_offer(meta: MediaOfferMeta<'_>, plaintext: &[u8]) -> Result<MediaOfferEnvelope> {
    let mut key = [0u8; 32];
    let mut nonce = [0u8; 12];
    let mut rng = rand::rng();
    rng.fill_bytes(&mut key);
    rng.fill_bytes(&mut nonce);

    let sealed = seal_with(&key, &nonce, plaintext)?;
    Ok(MediaOfferEnvelope {
        version: 1,
        media_type: meta.media_type.to_string(),
        media_hash: sealed_hash(&sealed),
        file_key_b64: general_purpose::STANDARD.encode(key),
        nonce_b64: general_purpose::STANDARD.encode(nonce),
        file_name: meta.file_name,
        mime_type: meta.mime_type,
        file_size: plaintext.len() as i64,
        width: meta.width,
        height: meta.height,
        duration_seconds: meta.duration_seconds,
        thumbnail_b64: meta
            .thumbnail
            .map(|data| general_purpose::STANDARD.encode(data)),
    })
}

fn seal_with(key: &[u8; 32], nonce: &[u8; 12], plaintext: &[u8]) -> Result<Vec<u8>> {
    Aes256Gcm::new(key.into())
        .encrypt(
            Nonce::from_slice(nonce),
            Payload {
                msg: plaintext,
                aad: SEAL_AAD,
            },
        )
        .map_err(|e| ZapLivreError::Crypto(format!("Media sealing failed: {}", e)))
}

fn sealed_hash(sealed: &[u8]) -> String {
    format!("{:x}", Sha256::digest(sealed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offer(plaintext: &[u8]) -> MediaOfferEnvelope {
        new_offer(
            MediaOfferMeta {
                media_type: "document",
                file_name: Some("contrato.pdf".to_string()),
                mime_type: Some("application/pdf".to_string()),
                width: None,
                height: None,
                duration_seconds: None,
                thumbnail: None,
            },
            plaintext,
        )
        .unwrap()
    }

    #[test]
    fn sealed_blob_round_trips_and_hides_the_plaintext_hash() {
        let plaintext = vec![7u8; 1024 * 1024];
        let offer = offer(&plaintext);
        let sealed = offer.seal(&plaintext).unwrap();

        assert_eq!(sealed.len() as u64, offer.sealed_size().unwrap());
        assert_ne!(
            offer.media_hash,
            format!("{:x}", Sha256::digest(&plaintext))
        );
        assert_eq!(offer.open(&sealed).unwrap(), plaintext);

        // The envelope survives the wire format.
        let decoded = MediaOfferEnvelope::decode(&offer.encode().unwrap()).unwrap();
        assert_eq!(decoded.open(&sealed).unwrap(), plaintext);
    }

    #[test]
    fn the_same_file_sent_twice_has_unrelated_hashes() {
        let plaintext = b"mesma foto".to_vec();
        assert_ne!(offer(&plaintext).media_hash, offer(&plaintext).media_hash);
    }

    #[test]
    fn tampered_or_foreign_blobs_are_refused() {
        let plaintext = b"conteudo legitimo".to_vec();
        let offer = offer(&plaintext);
        let mut sealed = offer.seal(&plaintext).unwrap();
        sealed[0] ^= 0xFF;
        assert!(offer.open(&sealed).is_err());

        // Right hash, wrong key: a blob sealed for another offer.
        let other = self::offer(&plaintext);
        let mut forged = other.clone();
        forged.file_key_b64 = offer.file_key_b64.clone();
        assert!(forged.open(&other.seal(&plaintext).unwrap()).is_err());
    }

    #[test]
    fn debug_does_not_leak_the_file_key() {
        let offer = offer(b"x");
        assert!(!format!("{:?}", offer).contains(&offer.file_key_b64));
    }
}
