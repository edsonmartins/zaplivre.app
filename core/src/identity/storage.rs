//! Secure storage for identity and prekeys
//!
//! This module provides secure storage of cryptographic keys using
//! platform-specific secure storage (Keychain on iOS/macOS, Keystore on Android).
//!
//! **Note**: This is the Rust interface. Platform-specific implementations
//! are provided via FFI from the host application.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use hkdf::Hkdf;
use rand::{rngs::StdRng, SeedableRng};
use sha2::Sha256;

use crate::identity::{Keypair, PreKeyPool};
use crate::utils::error::{Result, ZapLivreError};
use libsignal_protocol_syft::IdentityKeyPair;

/// Identity with keypair and prekey pool
#[derive(Clone)]
pub struct Identity {
    /// Main signing keypair (Ed25519)
    keypair: Keypair,
    /// Signal identity keypair record (serialized)
    signal_identity_keypair_record: Vec<u8>,
    /// Signal registration id
    signal_registration_id: u32,
    /// Peer ID derived from keypair
    peer_id: String,
    /// PreKey pool for Signal Protocol
    prekey_pool: Option<PreKeyPool>,
}

impl Identity {
    /// Generate a new identity with prekeys
    ///
    /// # Arguments
    ///
    /// * `prekey_count` - Number of one-time prekeys to generate (default: 100)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use zaplivre_core::identity::Identity;
    ///
    /// let identity = Identity::generate(100);
    /// println!("Peer ID: {}", identity.peer_id());
    /// ```
    pub fn generate(prekey_count: usize) -> Self {
        let keypair = Keypair::generate();
        let (signal_identity_keypair_record, signal_registration_id) =
            generate_signal_identity();
        let peer_id = keypair.peer_id();
        let prekey_pool = Some(PreKeyPool::new(
            keypair.clone(),
            signal_identity_keypair_record.clone(),
            signal_registration_id,
            prekey_count,
        ));

        Self {
            keypair,
            signal_identity_keypair_record,
            signal_registration_id,
            peer_id,
            prekey_pool,
        }
    }

    /// Create identity from existing keypair
    pub fn from_keypair(keypair: Keypair) -> Self {
        let peer_id = keypair.peer_id();
        let (signal_identity_keypair_record, signal_registration_id) =
            generate_signal_identity();

        Self {
            keypair,
            signal_identity_keypair_record,
            signal_registration_id,
            peer_id,
            prekey_pool: None,
        }
    }

    /// Get peer ID
    pub fn peer_id(&self) -> &str {
        &self.peer_id
    }

    /// Get keypair reference
    pub fn keypair(&self) -> &Keypair {
        &self.keypair
    }

    /// Get Signal identity keypair record (serialized)
    pub fn signal_identity_keypair_record(&self) -> &[u8] {
        &self.signal_identity_keypair_record
    }

    /// Get Signal registration id
    pub fn signal_registration_id(&self) -> u32 {
        self.signal_registration_id
    }

    /// Get mutable prekey pool reference
    pub fn prekey_pool_mut(&mut self) -> Option<&mut PreKeyPool> {
        self.prekey_pool.as_mut()
    }

    /// Get prekey pool reference
    pub fn prekey_pool(&self) -> Option<&PreKeyPool> {
        self.prekey_pool.as_ref()
    }

    /// SEC-07: snapshot do pool para persistência
    pub fn snapshot_prekey_pool(&self) -> Option<Result<Vec<u8>>> {
        self.prekey_pool.as_ref().map(|p| p.to_snapshot_bytes())
    }

    /// SEC-07: restaura o pool persistido (mantém o bundle estável entre
    /// restarts). Retorna Err se o snapshot for inválido.
    pub fn restore_prekey_pool(&mut self, snapshot: &[u8]) -> Result<()> {
        let pool = PreKeyPool::from_snapshot_bytes(self.keypair.clone(), snapshot)?;
        // A identidade Signal acompanha o pool restaurado - o signed prekey
        // do snapshot foi assinado por ELA
        self.signal_identity_keypair_record =
            pool.signal_identity_keypair_record_bytes().to_vec();
        self.signal_registration_id = pool.signal_registration_id_value();
        self.prekey_pool = Some(pool);
        Ok(())
    }

    /// Initialize prekey pool if not already initialized
    pub fn init_prekey_pool(&mut self, prekey_count: usize) {
        if self.prekey_pool.is_none() {
            self.prekey_pool = Some(PreKeyPool::new(
                self.keypair.clone(),
                self.signal_identity_keypair_record.clone(),
                self.signal_registration_id,
                prekey_count,
            ));
        }
    }

    /// Sign a message with this identity
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.keypair.sign(message)
    }

    /// Verify a signature
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<()> {
        self.keypair.verify(message, signature)
    }

    /// Derive a stable storage encryption key from the identity keypair
    pub fn storage_key(&self) -> Result<[u8; 32]> {
        let key_bytes = self.keypair.to_bytes();
        let hkdf = Hkdf::<Sha256>::new(Some(b"zaplivre-storage-v1"), &key_bytes);
        let mut out = [0u8; 32];
        hkdf.expand(b"storage-key", &mut out)
            .map_err(|e| ZapLivreError::Crypto(format!("Storage key derivation failed: {}", e)))?;
        Ok(out)
    }
}

impl std::fmt::Debug for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Identity")
            .field("peer_id", &self.peer_id)
            .field("has_prekey_pool", &self.prekey_pool.is_some())
            .finish_non_exhaustive()
    }
}

/// Serializable identity data for persistence
#[derive(Serialize, Deserialize)]
struct IdentityData {
    keypair_bytes: Vec<u8>,
    signal_identity_keypair_record: Option<Vec<u8>>,
    signal_registration_id: Option<u32>,
    peer_id: String,
    // Prekey pool is persisted separately due to size
}

/// Storage interface for identity persistence
///
/// This trait defines the interface for storing and retrieving identities.
/// Implementations should use platform-specific secure storage:
/// - iOS/macOS: Keychain
/// - Android: Keystore + EncryptedSharedPreferences
/// - Desktop: OS keyring (keyring-rs)
pub trait IdentityStorage: Send + Sync {
    /// Save identity to secure storage
    fn save_identity(&self, identity: &Identity) -> Result<()>;

    /// Load identity from secure storage
    fn load_identity(&self) -> Result<Option<Identity>>;

    /// Delete identity from secure storage
    fn delete_identity(&self) -> Result<()>;

    /// Check if identity exists in storage
    fn has_identity(&self) -> Result<bool>;
}

/// File-based identity storage (for development/testing)
///
/// ⚠️ **WARNING**: This stores keys in plaintext. DO NOT use in production!
/// Use platform-specific secure storage instead.
pub struct FileIdentityStorage {
    data_dir: PathBuf,
}

impl FileIdentityStorage {
    /// Create a new file-based storage
    ///
    /// # Arguments
    ///
    /// * `data_dir` - Directory for storing identity file
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
        }
    }

    fn identity_path(&self) -> PathBuf {
        self.data_dir.join("identity.json")
    }
}

impl IdentityStorage for FileIdentityStorage {
    fn save_identity(&self, identity: &Identity) -> Result<()> {
        // Ensure data directory exists
        std::fs::create_dir_all(&self.data_dir).map_err(|e| {
            ZapLivreError::Storage(format!("Failed to create data directory: {}", e))
        })?;

        let data = IdentityData {
            keypair_bytes: identity.keypair.to_bytes().to_vec(),
            signal_identity_keypair_record: Some(identity.signal_identity_keypair_record.clone()),
            signal_registration_id: Some(identity.signal_registration_id),
            peer_id: identity.peer_id.clone(),
        };

        let json = serde_json::to_string_pretty(&data)
            .map_err(|e| ZapLivreError::Storage(format!("Failed to serialize identity: {}", e)))?;

        std::fs::write(self.identity_path(), json)
            .map_err(|e| ZapLivreError::Storage(format!("Failed to write identity: {}", e)))?;

        Ok(())
    }

    fn load_identity(&self) -> Result<Option<Identity>> {
        let path = self.identity_path();

        if !path.exists() {
            return Ok(None);
        }

        let json = std::fs::read_to_string(&path)
            .map_err(|e| ZapLivreError::Storage(format!("Failed to read identity: {}", e)))?;

        let data: IdentityData = serde_json::from_str(&json)
            .map_err(|e| ZapLivreError::Storage(format!("Failed to deserialize identity: {}", e)))?;

        let keypair = Keypair::from_bytes(&data.keypair_bytes)?;
        let mut identity = Identity::from_keypair(keypair);
        if let Some(record) = data.signal_identity_keypair_record {
            identity.signal_identity_keypair_record = record;
        }
        if let Some(reg_id) = data.signal_registration_id {
            identity.signal_registration_id = reg_id;
        }

        Ok(Some(identity))
    }

    fn delete_identity(&self) -> Result<()> {
        let path = self.identity_path();

        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| ZapLivreError::Storage(format!("Failed to delete identity: {}", e)))?;
        }

        Ok(())
    }

    fn has_identity(&self) -> Result<bool> {
        Ok(self.identity_path().exists())
    }
}

fn generate_signal_identity() -> (Vec<u8>, u32) {
    let mut rng = StdRng::from_os_rng();
    let identity_keypair = IdentityKeyPair::generate(&mut rng);
    let record = identity_keypair.serialize().to_vec();
    let registration_id = (rand::random::<u16>() & 0x3fff) as u32;
    (record, registration_id)
}

/// In-memory identity storage (for testing)
pub struct MemoryIdentityStorage {
    identity: std::sync::Mutex<Option<Identity>>,
}

impl MemoryIdentityStorage {
    /// Create a new in-memory storage
    pub fn new() -> Self {
        Self {
            identity: std::sync::Mutex::new(None),
        }
    }
}

impl Default for MemoryIdentityStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl IdentityStorage for MemoryIdentityStorage {
    fn save_identity(&self, identity: &Identity) -> Result<()> {
        let mut guard = self
            .identity
            .lock()
            .map_err(|e| ZapLivreError::Storage(format!("Lock poisoned: {}", e)))?;

        *guard = Some(identity.clone());
        Ok(())
    }

    fn load_identity(&self) -> Result<Option<Identity>> {
        let guard = self
            .identity
            .lock()
            .map_err(|e| ZapLivreError::Storage(format!("Lock poisoned: {}", e)))?;

        Ok(guard.clone())
    }

    fn delete_identity(&self) -> Result<()> {
        let mut guard = self
            .identity
            .lock()
            .map_err(|e| ZapLivreError::Storage(format!("Lock poisoned: {}", e)))?;

        *guard = None;
        Ok(())
    }

    fn has_identity(&self) -> Result<bool> {
        let guard = self
            .identity
            .lock()
            .map_err(|e| ZapLivreError::Storage(format!("Lock poisoned: {}", e)))?;

        Ok(guard.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_generation() {
        let identity = Identity::generate(10);
        assert!(identity.peer_id().starts_with("zaplivre_"));
        assert!(identity.prekey_pool().is_some());
    }

    #[test]
    fn test_identity_from_keypair() {
        let keypair = Keypair::generate();
        let identity = Identity::from_keypair(keypair);

        assert!(identity.peer_id().starts_with("zaplivre_"));
        assert!(identity.prekey_pool().is_none());
    }

    #[test]
    fn test_init_prekey_pool() {
        let keypair = Keypair::generate();
        let mut identity = Identity::from_keypair(keypair);

        assert!(identity.prekey_pool().is_none());

        identity.init_prekey_pool(50);

        assert!(identity.prekey_pool().is_some());
        assert_eq!(identity.prekey_pool().unwrap().prekey_count(), 50);
    }

    #[test]
    fn test_memory_storage() {
        let storage = MemoryIdentityStorage::new();

        // Initially empty
        assert!(!storage.has_identity().unwrap());
        assert!(storage.load_identity().unwrap().is_none());

        // Save identity
        let identity = Identity::generate(10);
        let peer_id = identity.peer_id().to_string();

        storage.save_identity(&identity).unwrap();

        // Verify saved
        assert!(storage.has_identity().unwrap());

        let loaded = storage.load_identity().unwrap().unwrap();
        assert_eq!(loaded.peer_id(), peer_id);

        // Delete
        storage.delete_identity().unwrap();
        assert!(!storage.has_identity().unwrap());
    }

    #[test]
    fn test_file_storage() {
        let temp_dir = std::env::temp_dir().join("zaplivre_test_identity");
        let storage = FileIdentityStorage::new(&temp_dir);

        // Clean up before test
        let _ = storage.delete_identity();

        // Initially empty
        assert!(!storage.has_identity().unwrap());

        // Save identity
        let identity = Identity::generate(10);
        let peer_id = identity.peer_id().to_string();

        storage.save_identity(&identity).unwrap();

        // Verify file exists
        assert!(temp_dir.join("identity.json").exists());

        // Load identity
        let loaded = storage.load_identity().unwrap().unwrap();
        assert_eq!(loaded.peer_id(), peer_id);

        // Clean up
        storage.delete_identity().unwrap();
        assert!(!storage.has_identity().unwrap());

        // Clean up directory
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_identity_sign_verify() {
        let identity = Identity::generate(10);
        let message = b"Hello, ZapLivre!";

        let signature = identity.sign(message);
        let result = identity.verify(message, &signature);

        assert!(result.is_ok());
    }
}
