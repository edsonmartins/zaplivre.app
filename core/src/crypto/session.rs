//! Session Management for E2E Encryption
//!
//! This module manages encryption sessions between peers using the Signal Protocol.
//! Each session represents a secure communication channel with a specific peer.
//!
//! Session lifecycle:
//! 1. Initiated via X3DH key agreement (Alice initiates to Bob)
//! 2. Shared secret derived and stored
//! 3. Messages encrypted/decrypted using session state
//! 4. Ratchet state advanced with each message (forward secrecy)

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::crypto::signal::EncryptedMessage;
use crate::crypto::ratchet::RatchetState;
use crate::utils::error::{Result, MePassaError};

/// Session identifier (peer ID)
pub type SessionId = String;

/// E2E Encryption Session
///
/// Represents a secure communication channel with a peer.
/// Uses a ratchet state derived from X3DH for forward secrecy.
#[derive(Debug, Clone)]
pub struct Session {
    /// Remote peer ID
    pub peer_id: String,

    /// Ratchet state for forward secrecy
    pub ratchet: RatchetState,

    /// Whether this side initiated the session
    pub is_initiator: bool,

    /// Send message counter (prevents replay attacks)
    pub send_counter: u64,

    /// Receive message counter (prevents replay attacks)
    pub recv_counter: u64,

    /// Session creation timestamp (Unix seconds)
    pub created_at: u64,

    /// Last used timestamp (Unix seconds)
    pub last_used_at: u64,
}

impl Session {
    /// Create a new session from X3DH shared secret
    pub fn new(peer_id: String, shared_secret: [u8; 32], is_initiator: bool) -> Result<Self> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let ratchet = RatchetState::new(shared_secret, is_initiator)?;

        Ok(Self {
            peer_id,
            ratchet,
            is_initiator,
            send_counter: 0,
            recv_counter: 0,
            created_at: now,
            last_used_at: now,
        })
    }

    /// Encrypt a message using this session
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<EncryptedMessage> {
        // Update last used timestamp
        self.last_used_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Encrypt using ratchet state
        let encrypted = self.ratchet.encrypt(plaintext)?;

        // Increment send counter
        self.send_counter += 1;

        Ok(encrypted)
    }

    /// Decrypt a message using this session
    pub fn decrypt(&mut self, encrypted: &EncryptedMessage) -> Result<Vec<u8>> {
        // Update last used timestamp
        self.last_used_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Decrypt using ratchet state
        let plaintext = self.ratchet.decrypt(encrypted)?;

        // Increment receive counter
        self.recv_counter += 1;

        Ok(plaintext)
    }

    /// Get session age in seconds
    pub fn age(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now - self.created_at
    }

    /// Check if session is stale (not used in 7 days)
    pub fn is_stale(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        (now - self.last_used_at) > (7 * 24 * 60 * 60) // 7 days
    }
}

/// Session Manager
///
/// Manages multiple sessions with different peers.
/// Thread-safe using Arc<RwLock<...>>.
#[derive(Debug, Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<SessionId, Session>>>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new session with a peer
    pub fn create_session(&self, peer_id: String, shared_secret: [u8; 32]) -> Result<()> {
        let session = Session::new(peer_id.clone(), shared_secret, true)?;

        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| MePassaError::Crypto(format!("Lock error: {}", e)))?;

        sessions.insert(peer_id, session);

        Ok(())
    }

    pub fn create_session_with_role(
        &self,
        peer_id: String,
        shared_secret: [u8; 32],
        is_initiator: bool,
    ) -> Result<()> {
        let session = Session::new(peer_id.clone(), shared_secret, is_initiator)?;
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| MePassaError::Crypto(format!("Lock error: {}", e)))?;
        sessions.insert(peer_id, session);
        Ok(())
    }

    /// Get a session by peer ID
    pub fn get_session(&self, peer_id: &str) -> Result<Session> {
        let sessions = self
            .sessions
            .read()
            .map_err(|e| MePassaError::Crypto(format!("Lock error: {}", e)))?;

        sessions
            .get(peer_id)
            .cloned()
            .ok_or_else(|| MePassaError::Crypto(format!("Session not found: {}", peer_id)))
    }

    /// Update a session (after encrypt/decrypt)
    pub fn update_session(&self, session: Session) -> Result<()> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| MePassaError::Crypto(format!("Lock error: {}", e)))?;

        sessions.insert(session.peer_id.clone(), session);

        Ok(())
    }

    /// Encrypt a message for a peer
    pub fn encrypt_for(&self, peer_id: &str, plaintext: &[u8]) -> Result<EncryptedMessage> {
        let mut session = self.get_session(peer_id)?;
        let encrypted = session.encrypt(plaintext)?;
        self.update_session(session)?;
        Ok(encrypted)
    }

    /// Decrypt a message from a peer
    pub fn decrypt_from(&self, peer_id: &str, encrypted: &EncryptedMessage) -> Result<Vec<u8>> {
        let mut session = self.get_session(peer_id)?;
        let plaintext = session.decrypt(encrypted)?;
        self.update_session(session)?;
        Ok(plaintext)
    }

    /// Check if a session exists with a peer
    pub fn has_session(&self, peer_id: &str) -> Result<bool> {
        let sessions = self
            .sessions
            .read()
            .map_err(|e| MePassaError::Crypto(format!("Lock error: {}", e)))?;

        Ok(sessions.contains_key(peer_id))
    }

    /// Remove a session
    pub fn remove_session(&self, peer_id: &str) -> Result<()> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| MePassaError::Crypto(format!("Lock error: {}", e)))?;

        sessions.remove(peer_id);

        Ok(())
    }

    /// Get all active session IDs
    pub fn list_sessions(&self) -> Result<Vec<String>> {
        let sessions = self
            .sessions
            .read()
            .map_err(|e| MePassaError::Crypto(format!("Lock error: {}", e)))?;

        Ok(sessions.keys().cloned().collect())
    }

    /// Get total number of sessions
    pub fn session_count(&self) -> Result<usize> {
        let sessions = self
            .sessions
            .read()
            .map_err(|e| MePassaError::Crypto(format!("Lock error: {}", e)))?;

        Ok(sessions.len())
    }

    /// Remove stale sessions (not used in 7 days)
    pub fn cleanup_stale_sessions(&self) -> Result<usize> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| MePassaError::Crypto(format!("Lock error: {}", e)))?;

        let stale_keys: Vec<String> = sessions
            .iter()
            .filter(|(_, session)| session.is_stale())
            .map(|(key, _)| key.clone())
            .collect();

        let count = stale_keys.len();

        for key in stale_keys {
            sessions.remove(&key);
        }

        Ok(count)
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::X3DH;
    use crate::identity::Identity;

    #[test]
    fn test_session_creation() {
        let shared_secret = [42u8; 32];
        let session = Session::new("peer_123".to_string(), shared_secret, true).unwrap();

        assert_eq!(session.peer_id, "peer_123");
        assert_eq!(session.ratchet.root_key, shared_secret);
        assert_eq!(session.send_counter, 0);
        assert_eq!(session.recv_counter, 0);
        assert!(session.age() < 2); // Less than 2 seconds old
        assert!(!session.is_stale());
    }

    #[test]
    fn test_session_encrypt_decrypt() {
        let shared_secret = [42u8; 32];
        let mut session = Session::new("peer_123".to_string(), shared_secret, true).unwrap();

        let plaintext = b"Hello, MePassa!";

        // Encrypt
        let encrypted = session.encrypt(plaintext).unwrap();
        assert_eq!(session.send_counter, 1);

        // Decrypt
        let decrypted = session.decrypt(&encrypted).unwrap();
        assert_eq!(session.recv_counter, 1);

        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_session_manager_create_and_get() {
        let manager = SessionManager::new();
        let shared_secret = [42u8; 32];

        // Create session
        manager
            .create_session("peer_123".to_string(), shared_secret)
            .unwrap();

        // Check exists
        assert!(manager.has_session("peer_123").unwrap());

        // Get session
        let session = manager.get_session("peer_123").unwrap();
        assert_eq!(session.peer_id, "peer_123");
        assert_eq!(session.ratchet.root_key, shared_secret);
    }

    #[test]
    fn test_session_manager_encrypt_decrypt() {
        let manager = SessionManager::new();
        let shared_secret = [42u8; 32];

        manager
            .create_session("peer_123".to_string(), shared_secret)
            .unwrap();

        let plaintext = b"Hello from session manager!";

        // Encrypt
        let encrypted = manager.encrypt_for("peer_123", plaintext).unwrap();

        // Decrypt
        let decrypted = manager.decrypt_from("peer_123", &encrypted).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());

        // Check counters were updated
        let session = manager.get_session("peer_123").unwrap();
        assert_eq!(session.send_counter, 1);
        assert_eq!(session.recv_counter, 1);
    }

    #[test]
    fn test_session_manager_remove() {
        let manager = SessionManager::new();
        let shared_secret = [42u8; 32];

        manager
            .create_session("peer_123".to_string(), shared_secret)
            .unwrap();

        assert!(manager.has_session("peer_123").unwrap());

        manager.remove_session("peer_123").unwrap();

        assert!(!manager.has_session("peer_123").unwrap());
    }

    #[test]
    fn test_session_manager_list_sessions() {
        let manager = SessionManager::new();

        manager
            .create_session("peer_1".to_string(), [1u8; 32])
            .unwrap();
        manager
            .create_session("peer_2".to_string(), [2u8; 32])
            .unwrap();
        manager
            .create_session("peer_3".to_string(), [3u8; 32])
            .unwrap();

        let sessions = manager.list_sessions().unwrap();
        assert_eq!(sessions.len(), 3);
        assert!(sessions.contains(&"peer_1".to_string()));
        assert!(sessions.contains(&"peer_2".to_string()));
        assert!(sessions.contains(&"peer_3".to_string()));

        assert_eq!(manager.session_count().unwrap(), 3);
    }

    #[test]
    fn test_session_not_found() {
        let manager = SessionManager::new();

        let result = manager.get_session("nonexistent");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Session not found"));
    }

    #[test]
    fn test_e2e_alice_to_bob_with_sessions() {
        // Bob generates identity WITHOUT one-time prekeys (to avoid bundle consumption issues in tests)
        let bob = Identity::generate(0);
        let mut bob_mut = bob.clone();

        // Bob saves his signed prekey secret
        let bob_signed_prekey_secret = bob_mut
            .prekey_pool()
            .unwrap()
            .signed_prekey()
            .secret_bytes();

        // Get Bob's prekey bundle (no one-time prekey)
        let bob_bundle = bob_mut.prekey_pool_mut().unwrap().get_bundle();

        // Alice initiates X3DH
        let (alice_shared_secret, alice_ephemeral_pub) = X3DH::initiate(&bob_bundle).unwrap();

        // Alice creates session with Bob
        let alice_manager = SessionManager::new();
        alice_manager
            .create_session_with_role(bob.peer_id().to_string(), alice_shared_secret, true)
            .unwrap();

        // Bob responds to X3DH (no one-time prekey)
        let bob_shared_secret =
            X3DH::respond(&bob_signed_prekey_secret, None, &alice_ephemeral_pub).unwrap();

        // Bob creates session with Alice
        let bob_manager = SessionManager::new();
        bob_manager
            .create_session_with_role("alice".to_string(), bob_shared_secret, false)
            .unwrap();

        // Alice sends encrypted message to Bob
        let alice_message = b"Secret message from Alice!";
        let encrypted = alice_manager
            .encrypt_for(&bob.peer_id().to_string(), alice_message)
            .unwrap();

        // Bob decrypts message from Alice
        let decrypted = bob_manager.decrypt_from("alice", &encrypted).unwrap();

        assert_eq!(alice_message, decrypted.as_slice());
    }

    #[test]
    fn test_multiple_messages_in_session() {
        let manager = SessionManager::new();
        let shared_secret = [42u8; 32];

        manager
            .create_session("peer_123".to_string(), shared_secret)
            .unwrap();

        // Send 10 messages
        for i in 0..10 {
            let msg = format!("Message {}", i);
            let encrypted = manager.encrypt_for("peer_123", msg.as_bytes()).unwrap();
            let decrypted = manager.decrypt_from("peer_123", &encrypted).unwrap();
            assert_eq!(msg.as_bytes(), decrypted.as_slice());
        }

        // Check counters
        let session = manager.get_session("peer_123").unwrap();
        assert_eq!(session.send_counter, 10);
        assert_eq!(session.recv_counter, 10);
    }
}
