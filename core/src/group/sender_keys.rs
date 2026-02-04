//! Sender Keys for Group E2E Encryption
//!
//! Implementation of Signal Protocol Sender Keys for group messaging.
//!
//! Sender Keys allow efficient group encryption:
//! - Each sender has one key shared with all group members
//! - No need for N pairwise sessions in a group of N members
//! - Forward secrecy through ratcheting
//!
//! References:
//! - https://signal.org/docs/specifications/doubleratchet/#sender-keys
//! - https://signal.org/docs/specifications/sesame/

use crate::utils::error::{MePassaError, Result};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use ed25519_dalek::SigningKey;
use hkdf::Hkdf;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

// HKDF info strings (as per Signal Protocol spec)
const HKDF_INFO_CHAIN_KEY: &[u8] = b"MePassaSenderKeyChain";
const HKDF_INFO_MESSAGE_KEY: &[u8] = b"MePassaSenderKeyMessage";

/// Sender Key for group encryption
///
/// Each group member has a sender key that they use to encrypt messages.
/// All other members receive and store this sender key to decrypt messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenderKey {
    /// Group ID
    pub group_id: String,

    /// Sender peer ID
    pub sender_peer_id: String,

    /// Chain key (ratcheted forward for each message)
    chain_key: Vec<u8>,

    /// Message key derivation counter
    iteration: u32,

    /// Public signing key (Ed25519)
    signing_key: Vec<u8>,
}

impl SenderKey {
    /// Generate a new sender key
    pub fn generate(group_id: String, sender_peer_id: String) -> Result<Self> {
        // Generate random chain key (32 bytes)
        let chain_key = rand::random::<[u8; 32]>().to_vec();

        // Generate signing key (public key stored; private key handled by sender)
        let signing_key = SigningKey::generate(&mut OsRng).verifying_key().to_bytes().to_vec();

        Ok(Self {
            group_id,
            sender_peer_id,
            chain_key,
            iteration: 0,
            signing_key,
        })
    }

    /// Encrypt a message with this sender key
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>> {
        // Derive message key from current chain key
        let message_key = self.derive_message_key()?;

        // Increment iteration BEFORE ratcheting
        let current_iteration = self.iteration;
        self.iteration += 1;

        // Ratchet forward to get new chain key
        self.ratchet_forward()?;

        // Encrypt with AES-256-GCM
        let cipher = Aes256Gcm::new_from_slice(&message_key)
            .map_err(|e| MePassaError::Crypto(format!("Failed to create cipher: {}", e)))?;

        // Generate nonce from iteration (12 bytes for GCM)
        let nonce_bytes = self.iteration_to_nonce(current_iteration);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| MePassaError::Crypto(format!("Encryption failed: {}", e)))?;

        // Prepend iteration (4 bytes) to ciphertext for recipient
        let mut result = current_iteration.to_be_bytes().to_vec();
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// Decrypt a message with this sender key
    pub fn decrypt(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        // Extract iteration from first 4 bytes
        if data.len() < 4 {
            return Err(MePassaError::Crypto("Invalid ciphertext: too short".to_string()));
        }

        let iteration_bytes: [u8; 4] = data[0..4].try_into()
            .map_err(|_| MePassaError::Crypto("Failed to parse iteration".to_string()))?;
        let message_iteration = u32::from_be_bytes(iteration_bytes);
        let ciphertext = &data[4..];

        // Handle out-of-order messages by deriving the correct message key
        // NOTE: In production, should cache previous message keys for out-of-order delivery
        let message_key = if message_iteration == self.iteration {
            // Current message
            self.derive_message_key()?
        } else if message_iteration > self.iteration {
            // Future message - need to ratchet forward
            let steps = message_iteration - self.iteration;
            self.ratchet_n_times(steps)?;
            self.derive_message_key()?
        } else {
            // Past message - this is a simplified implementation
            // Production should maintain a sliding window of message keys
            return Err(MePassaError::Crypto(format!(
                "Cannot decrypt past message (iteration {} < {}). Out-of-order delivery not fully supported.",
                message_iteration, self.iteration
            )));
        };

        // Decrypt with AES-256-GCM
        let cipher = Aes256Gcm::new_from_slice(&message_key)
            .map_err(|e| MePassaError::Crypto(format!("Failed to create cipher: {}", e)))?;

        // Generate nonce from iteration
        let nonce_bytes = self.iteration_to_nonce(message_iteration);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Decrypt
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| MePassaError::Crypto(format!("Decryption failed: {}", e)))?;

        // Update iteration after successful decryption
        if message_iteration >= self.iteration {
            self.iteration = message_iteration + 1;
            self.ratchet_forward()?;
        }

        Ok(plaintext)
    }

    /// Ratchet chain key forward using HKDF-SHA256
    fn ratchet_forward(&mut self) -> Result<()> {
        // Use HKDF to derive new chain key from current chain key
        // New chain key = HKDF(chain_key, salt=iteration, info="MePassaSenderKeyChain")

        let hkdf = Hkdf::<Sha256>::new(
            Some(&self.iteration.to_be_bytes()), // salt includes iteration for uniqueness
            &self.chain_key,
        );

        let mut new_chain_key = [0u8; 32];
        hkdf.expand(HKDF_INFO_CHAIN_KEY, &mut new_chain_key)
            .map_err(|e| MePassaError::Crypto(format!("HKDF chain key derivation failed: {}", e)))?;

        self.chain_key = new_chain_key.to_vec();
        Ok(())
    }

    /// Derive message key from current chain key
    fn derive_message_key(&self) -> Result<Vec<u8>> {
        // Message key = HKDF(chain_key, salt=iteration, info="MePassaSenderKeyMessage")
        let hkdf = Hkdf::<Sha256>::new(
            Some(&self.iteration.to_be_bytes()),
            &self.chain_key,
        );

        let mut message_key = [0u8; 32];
        hkdf.expand(HKDF_INFO_MESSAGE_KEY, &mut message_key)
            .map_err(|e| MePassaError::Crypto(format!("HKDF message key derivation failed: {}", e)))?;

        Ok(message_key.to_vec())
    }

    /// Convert iteration to 12-byte nonce for AES-GCM
    fn iteration_to_nonce(&self, iteration: u32) -> [u8; 12] {
        let mut nonce = [0u8; 12];
        // Use iteration (4 bytes) + group_id hash (8 bytes)
        nonce[0..4].copy_from_slice(&iteration.to_be_bytes());

        // Hash group_id to get 8 bytes for uniqueness
        use sha2::Digest;
        let mut hasher = Sha256::new();
        hasher.update(self.group_id.as_bytes());
        let hash = hasher.finalize();
        nonce[4..12].copy_from_slice(&hash[0..8]);

        nonce
    }

    /// Ratchet forward N times (for handling out-of-order messages)
    fn ratchet_n_times(&mut self, n: u32) -> Result<()> {
        for _ in 0..n {
            self.iteration += 1;
            self.ratchet_forward()?;
        }
        Ok(())
    }

    /// Serialize sender key for transmission
    pub fn serialize(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self)
            .map_err(|e| MePassaError::Crypto(format!("Failed to serialize sender key: {}", e)))
    }

    /// Deserialize sender key from bytes
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        serde_json::from_slice(data)
            .map_err(|e| MePassaError::Crypto(format!("Failed to deserialize sender key: {}", e)))
    }
}

/// Sender Key Store
///
/// Stores sender keys for all group members.
pub struct SenderKeyStore {
    /// Keys indexed by (group_id, sender_peer_id)
    keys: std::collections::HashMap<(String, String), SenderKey>,
}

impl SenderKeyStore {
    /// Create a new sender key store
    pub fn new() -> Self {
        Self {
            keys: std::collections::HashMap::new(),
        }
    }

    /// Store a sender key
    pub fn store_key(&mut self, key: SenderKey) {
        let index = (key.group_id.clone(), key.sender_peer_id.clone());
        self.keys.insert(index, key);
    }

    /// Get a sender key
    pub fn get_key(&self, group_id: &str, sender_peer_id: &str) -> Option<&SenderKey> {
        let index = (group_id.to_string(), sender_peer_id.to_string());
        self.keys.get(&index)
    }

    /// Get a mutable sender key
    pub fn get_key_mut(&mut self, group_id: &str, sender_peer_id: &str) -> Option<&mut SenderKey> {
        let index = (group_id.to_string(), sender_peer_id.to_string());
        self.keys.get_mut(&index)
    }

    /// Remove all keys for a group
    pub fn remove_group(&mut self, group_id: &str) {
        self.keys.retain(|(gid, _), _| gid != group_id);
    }
}

impl Default for SenderKeyStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_sender_key() {
        let key = SenderKey::generate("group-1".to_string(), "peer-1".to_string()).unwrap();

        assert_eq!(key.group_id, "group-1");
        assert_eq!(key.sender_peer_id, "peer-1");
        assert_eq!(key.iteration, 0);
        assert_eq!(key.chain_key.len(), 32);
    }

    #[test]
    fn test_encrypt_decrypt() {
        let mut sender_key = SenderKey::generate("group-1".to_string(), "peer-1".to_string()).unwrap();
        let mut receiver_key = sender_key.clone();

        let plaintext = b"Hello, group!";
        let ciphertext = sender_key.encrypt(plaintext).unwrap();

        // Ciphertext should be different from plaintext (real encryption)
        assert_ne!(&ciphertext[4..], plaintext); // Skip first 4 bytes (iteration)

        let decrypted = receiver_key.decrypt(&ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_sender_key_store() {
        let mut store = SenderKeyStore::new();

        let key1 = SenderKey::generate("group-1".to_string(), "peer-1".to_string()).unwrap();
        let key2 = SenderKey::generate("group-1".to_string(), "peer-2".to_string()).unwrap();

        store.store_key(key1);
        store.store_key(key2);

        assert!(store.get_key("group-1", "peer-1").is_some());
        assert!(store.get_key("group-1", "peer-2").is_some());
        assert!(store.get_key("group-2", "peer-1").is_none());

        store.remove_group("group-1");
        assert!(store.get_key("group-1", "peer-1").is_none());
    }

    #[test]
    fn test_ratcheting_forward_secrecy() {
        let mut key = SenderKey::generate("group-1".to_string(), "peer-1".to_string()).unwrap();

        // Encrypt first message
        let plaintext1 = b"Message 1";
        let ciphertext1 = key.encrypt(plaintext1).unwrap();

        // Save chain key state after first message
        let chain_key_after_msg1 = key.chain_key.clone();

        // Encrypt second message
        let plaintext2 = b"Message 2";
        let ciphertext2 = key.encrypt(plaintext2).unwrap();

        // Chain key should have changed (forward secrecy)
        assert_ne!(key.chain_key, chain_key_after_msg1);

        // Ciphertexts should be different (different keys)
        assert_ne!(ciphertext1, ciphertext2);
    }

    #[test]
    fn test_multiple_messages_in_sequence() {
        let mut sender_key = SenderKey::generate("group-1".to_string(), "peer-1".to_string()).unwrap();
        let mut receiver_key = sender_key.clone();

        // Send 5 messages in sequence
        let messages = vec![
            b"Message 1".to_vec(),
            b"Message 2".to_vec(),
            b"Message 3".to_vec(),
            b"Message 4".to_vec(),
            b"Message 5".to_vec(),
        ];

        let mut ciphertexts = Vec::new();

        for msg in &messages {
            let ciphertext = sender_key.encrypt(msg).unwrap();
            ciphertexts.push(ciphertext);
        }

        // Decrypt all messages
        for (i, ciphertext) in ciphertexts.iter().enumerate() {
            let decrypted = receiver_key.decrypt(ciphertext).unwrap();
            assert_eq!(decrypted, messages[i]);
        }
    }

    #[test]
    fn test_different_keys_different_ciphertext() {
        let mut key1 = SenderKey::generate("group-1".to_string(), "peer-1".to_string()).unwrap();
        let mut key2 = SenderKey::generate("group-1".to_string(), "peer-2".to_string()).unwrap();

        let plaintext = b"Same plaintext";

        let ciphertext1 = key1.encrypt(plaintext).unwrap();
        let ciphertext2 = key2.encrypt(plaintext).unwrap();

        // Different sender keys should produce different ciphertexts
        assert_ne!(ciphertext1, ciphertext2);
    }

    #[test]
    fn test_serialization() {
        let key = SenderKey::generate("group-1".to_string(), "peer-1".to_string()).unwrap();

        let serialized = key.serialize().unwrap();
        let deserialized = SenderKey::deserialize(&serialized).unwrap();

        assert_eq!(key.group_id, deserialized.group_id);
        assert_eq!(key.sender_peer_id, deserialized.sender_peer_id);
        assert_eq!(key.chain_key, deserialized.chain_key);
        assert_eq!(key.iteration, deserialized.iteration);
    }
}
