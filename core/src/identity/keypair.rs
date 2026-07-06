//! Keypair management for ZapLivre
//!
//! This module handles the generation and management of Ed25519 signing keypairs
//! used for identity and authentication in the ZapLivre network.

use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use rand_core06::OsRng;
use serde::{Deserialize, Serialize};
use std::fmt;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};

use crate::utils::error::{Result, ZapLivreError};

/// A cryptographic keypair (Ed25519) for signing and verification
///
/// This keypair is used for:
/// - Peer identity (derives peer ID)
/// - Message signatures
/// - Authentication
#[derive(Clone)]
pub struct Keypair {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl Keypair {
    /// Generate a new random keypair using OS-provided randomness
    ///
    /// # Example
    ///
    /// ```no_run
    /// use zaplivre_core::identity::Keypair;
    ///
    /// let keypair = Keypair::generate();
    /// println!("Generated peer ID: {}", keypair.peer_id());
    /// ```
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        Self {
            signing_key,
            verifying_key,
        }
    }

    /// Create a keypair from raw bytes (32 bytes for Ed25519)
    ///
    /// # Arguments
    ///
    /// * `bytes` - 32-byte secret key
    ///
    /// # Errors
    ///
    /// Returns error if bytes length is not exactly 32
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 32 {
            return Err(ZapLivreError::Identity(format!(
                "Invalid key length: expected 32 bytes, got {}",
                bytes.len()
            )));
        }

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(bytes);

        let signing_key = SigningKey::from_bytes(&key_bytes);
        let verifying_key = signing_key.verifying_key();

        Ok(Self {
            signing_key,
            verifying_key,
        })
    }

    /// Create a keypair from a libp2p keypair
    ///
    /// # Arguments
    ///
    /// * `libp2p_keypair` - libp2p Ed25519 keypair
    ///
    /// # Errors
    ///
    /// Returns error if the libp2p keypair is not Ed25519
    pub fn from_libp2p_keypair(libp2p_keypair: &libp2p::identity::Keypair) -> Result<Self> {
        // Clone the keypair since try_into_ed25519 consumes it
        let kp_clone = libp2p_keypair.clone();

        // Try to convert to Ed25519 keypair
        let ed25519_kp = kp_clone
            .try_into_ed25519()
            .map_err(|_| ZapLivreError::Identity("Only Ed25519 keypairs are supported".to_string()))?;

        // Get the keypair bytes (64 bytes: 32 secret + 32 public)
        let keypair_bytes = ed25519_kp.to_bytes();

        // Extract only the secret key (first 32 bytes)
        let secret_bytes = &keypair_bytes[0..32];

        // Create our keypair from the secret bytes
        Self::from_bytes(secret_bytes)
    }

    /// Export the secret key as bytes (32 bytes)
    ///
    /// ⚠️ **WARNING**: Keep this secret! Never expose or transmit the secret key.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Get the public key (verifying key) as bytes (32 bytes)
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// Get the public key as a PublicKey type
    pub fn public_key(&self) -> PublicKey {
        PublicKey {
            key: self.verifying_key,
        }
    }

    /// Sign a message with this keypair
    ///
    /// # Arguments
    ///
    /// * `message` - The message to sign
    ///
    /// # Returns
    ///
    /// 64-byte signature
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        let signature: Signature = self.signing_key.sign(message);
        signature.to_bytes()
    }

    /// Verify a signature against this keypair's public key
    ///
    /// # Arguments
    ///
    /// * `message` - The original message
    /// * `signature` - The 64-byte signature
    ///
    /// # Returns
    ///
    /// `Ok(())` if signature is valid, `Err` otherwise
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<()> {
        self.public_key().verify(message, signature)
    }

    /// Get the peer ID derived from this keypair
    ///
    /// Format: `zaplivre_<base58(public_key)>`
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use zaplivre_core::identity::Keypair;
    /// let keypair = Keypair::generate();
    /// let peer_id = keypair.peer_id();
    /// assert!(peer_id.starts_with("zaplivre_"));
    /// ```
    pub fn peer_id(&self) -> String {
        self.public_key().peer_id()
    }
}

impl fmt::Debug for Keypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Keypair")
            .field("peer_id", &self.peer_id())
            .field("public_key", &hex::encode(self.public_key_bytes()))
            .finish_non_exhaustive()
    }
}

/// A public key for verification
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PublicKey {
    key: VerifyingKey,
}

impl PublicKey {
    /// Create a PublicKey from raw bytes (32 bytes)
    ///
    /// # Arguments
    ///
    /// * `bytes` - 32-byte public key
    ///
    /// # Errors
    ///
    /// Returns error if bytes are invalid
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 32 {
            return Err(ZapLivreError::Identity(format!(
                "Invalid public key length: expected 32 bytes, got {}",
                bytes.len()
            )));
        }

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(bytes);

        let key = VerifyingKey::from_bytes(&key_bytes)
            .map_err(|e| ZapLivreError::Identity(format!("Invalid public key: {}", e)))?;

        Ok(Self { key })
    }

    /// Export public key as bytes (32 bytes)
    pub fn to_bytes(&self) -> [u8; 32] {
        self.key.to_bytes()
    }

    /// Verify a signature against this public key
    ///
    /// # Arguments
    ///
    /// * `message` - The original message
    /// * `signature` - The 64-byte signature
    ///
    /// # Returns
    ///
    /// `Ok(())` if signature is valid, `Err` otherwise
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<()> {
        if signature.len() != 64 {
            return Err(ZapLivreError::Crypto(format!(
                "Invalid signature length: expected 64 bytes, got {}",
                signature.len()
            )));
        }

        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(signature);

        let signature = Signature::from_bytes(&sig_bytes);

        self.key
            .verify(message, &signature)
            .map_err(|e| ZapLivreError::Crypto(format!("Signature verification failed: {}", e)))
    }

    /// Get the peer ID derived from this public key
    ///
    /// Format: `zaplivre_<base58(public_key)>`
    pub fn peer_id(&self) -> String {
        let encoded = bs58::encode(self.to_bytes()).into_string();
        format!("zaplivre_{}", encoded)
    }

    /// Parse a peer ID back to a PublicKey
    ///
    /// # Arguments
    ///
    /// * `peer_id` - Peer ID in format `zaplivre_<base58>`
    ///
    /// # Errors
    ///
    /// Returns error if peer ID format is invalid
    pub fn from_peer_id(peer_id: &str) -> Result<Self> {
        if !peer_id.starts_with("zaplivre_") {
            return Err(ZapLivreError::Identity("Invalid peer ID format: must start with 'zaplivre_'".to_string()));
        }

        let encoded = &peer_id[8..]; // Skip "zaplivre_" prefix
        let bytes = bs58::decode(encoded)
            .into_vec()
            .map_err(|e| ZapLivreError::Identity(format!("Invalid base58 encoding: {}", e)))?;

        Self::from_bytes(&bytes)
    }
}

/// Signal identity keypair (X25519) for Signal Protocol
#[derive(Clone)]
pub struct SignalKeypair {
    secret: StaticSecret,
    public: X25519PublicKey,
}

impl SignalKeypair {
    /// Generate a new Signal identity keypair
    pub fn generate() -> Self {
        let secret = StaticSecret::random_from_rng(OsRng);
        let public = X25519PublicKey::from(&secret);
        Self { secret, public }
    }

    /// Create from raw secret bytes (32 bytes)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 32 {
            return Err(ZapLivreError::Identity(format!(
                "Invalid Signal key length: expected 32 bytes, got {}",
                bytes.len()
            )));
        }

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(bytes);
        let secret = StaticSecret::from(key_bytes);
        let public = X25519PublicKey::from(&secret);
        Ok(Self { secret, public })
    }

    /// Export secret bytes (32 bytes)
    pub fn to_bytes(&self) -> [u8; 32] {
        self.secret.to_bytes()
    }

    /// Export public bytes (32 bytes)
    pub fn public_bytes(&self) -> [u8; 32] {
        self.public.to_bytes()
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.peer_id())
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PublicKey")
            .field("peer_id", &self.peer_id())
            .field("bytes", &hex::encode(self.to_bytes()))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = Keypair::generate();
        let peer_id = keypair.peer_id();

        assert!(peer_id.starts_with("zaplivre_"));
        assert!(peer_id.len() > 8);
    }

    #[test]
    fn test_keypair_from_bytes() {
        let keypair1 = Keypair::generate();
        let bytes = keypair1.to_bytes();

        let keypair2 = Keypair::from_bytes(&bytes).unwrap();

        assert_eq!(keypair1.peer_id(), keypair2.peer_id());
        assert_eq!(keypair1.public_key_bytes(), keypair2.public_key_bytes());
    }

    #[test]
    fn test_keypair_from_invalid_bytes() {
        let result = Keypair::from_bytes(&[0u8; 16]);
        assert!(result.is_err());
    }

    #[test]
    fn test_sign_and_verify() {
        let keypair = Keypair::generate();
        let message = b"Hello, ZapLivre!";

        let signature = keypair.sign(message);
        assert_eq!(signature.len(), 64);

        let result = keypair.verify(message, &signature);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_invalid_signature() {
        let keypair = Keypair::generate();
        let message = b"Hello, ZapLivre!";
        let wrong_signature = [0u8; 64];

        let result = keypair.verify(message, &wrong_signature);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_with_different_message() {
        let keypair = Keypair::generate();
        let message1 = b"Hello, ZapLivre!";
        let message2 = b"Goodbye, ZapLivre!";

        let signature = keypair.sign(message1);
        let result = keypair.verify(message2, &signature);

        assert!(result.is_err());
    }

    #[test]
    fn test_public_key_from_bytes() {
        let keypair = Keypair::generate();
        let pub_bytes = keypair.public_key_bytes();

        let public_key = PublicKey::from_bytes(&pub_bytes).unwrap();

        assert_eq!(keypair.peer_id(), public_key.peer_id());
    }

    #[test]
    fn test_public_key_verify() {
        let keypair = Keypair::generate();
        let public_key = keypair.public_key();
        let message = b"Test message";

        let signature = keypair.sign(message);
        let result = public_key.verify(message, &signature);

        assert!(result.is_ok());
    }

    #[test]
    fn test_peer_id_roundtrip() {
        let keypair = Keypair::generate();
        let peer_id = keypair.peer_id();

        let public_key = PublicKey::from_peer_id(&peer_id).unwrap();

        assert_eq!(keypair.peer_id(), public_key.peer_id());
        assert_eq!(keypair.public_key_bytes(), public_key.to_bytes());
    }

    #[test]
    fn test_invalid_peer_id() {
        let result = PublicKey::from_peer_id("invalid_peer_id");
        assert!(result.is_err());

        let result = PublicKey::from_peer_id("notzaplivre_abc123");
        assert!(result.is_err());
    }

    #[test]
    fn test_keypair_debug() {
        let keypair = Keypair::generate();
        let debug_str = format!("{:?}", keypair);

        assert!(debug_str.contains("Keypair"));
        assert!(debug_str.contains("peer_id"));
    }

    #[test]
    fn test_public_key_display() {
        let keypair = Keypair::generate();
        let public_key = keypair.public_key();
        let display_str = format!("{}", public_key);

        assert!(display_str.starts_with("zaplivre_"));
    }
}
