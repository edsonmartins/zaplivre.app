//! Storage encryption helpers
//!
//! Encrypt/decrypt message content for at-rest storage using AES-GCM.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::utils::error::{MePassaError, Result};

#[derive(Debug, Serialize, Deserialize)]
struct StorageEnvelope {
    nonce: [u8; 12],
    ciphertext: Vec<u8>,
}

pub fn encrypt_for_storage(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(key.into());
    let mut nonce_bytes = [0u8; 12];
    let mut rng = rand::rng();
    rng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| MePassaError::Crypto(format!("Storage encryption failed: {}", e)))?;

    let envelope = StorageEnvelope {
        nonce: nonce_bytes,
        ciphertext,
    };
    bincode::serialize(&envelope)
        .map_err(|e| MePassaError::Crypto(format!("Storage encrypt serialize failed: {}", e)))
}

pub fn decrypt_for_storage(key: &[u8; 32], blob: &[u8]) -> Result<Vec<u8>> {
    let envelope: StorageEnvelope = bincode::deserialize(blob)
        .map_err(|e| MePassaError::Crypto(format!("Storage decrypt deserialize failed: {}", e)))?;
    let cipher = Aes256Gcm::new(key.into());
    let nonce = Nonce::from_slice(&envelope.nonce);
    let plaintext = cipher
        .decrypt(nonce, envelope.ciphertext.as_ref())
        .map_err(|e| MePassaError::Crypto(format!("Storage decryption failed: {}", e)))?;
    Ok(plaintext)
}
