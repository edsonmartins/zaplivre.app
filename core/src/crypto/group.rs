//! Group Messaging Cryptography (Sender Keys)
//!
//! This module implements Sender Keys for efficient end-to-end encrypted group messaging.
//! Based on Signal Protocol's Sender Key approach.
//!
//! **How Sender Keys Work:**
//! 1. Each group member has their own sender key
//! 2. When Alice sends to group, she encrypts with her sender key
//! 3. All group members have Alice's sender key and can decrypt
//! 4. Sender keys are distributed encrypted (using pairwise sessions)
//!
//! **Simplified Implementation:**
//! - Each sender key is derived from a random seed
//! - Sender keys ratchet forward with each message (like Double Ratchet)
//! - Group state is managed locally (no central coordinator)

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;

use crate::utils::error::{MePassaError, Result};
use serde::{Deserialize, Serialize};

/// AES-GCM encrypted payload for sender-key messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedMessage {
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

fn encrypt_message(plaintext: &[u8], key: &[u8; 32]) -> Result<EncryptedMessage> {
    let cipher = Aes256Gcm::new(key.into());
    let mut nonce_bytes = [0u8; 12];
    let mut rng = rand::rng();
    rng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| MePassaError::Crypto(format!("Group encryption failed: {}", e)))?;
    Ok(EncryptedMessage {
        nonce: nonce_bytes,
        ciphertext,
    })
}

fn decrypt_message(encrypted: &EncryptedMessage, key: &[u8; 32]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(key.into());
    let nonce = Nonce::from_slice(&encrypted.nonce);
    cipher
        .decrypt(nonce, encrypted.ciphertext.as_ref())
        .map_err(|e| MePassaError::Crypto(format!("Group decryption failed: {}", e)))
}

/// Group ID (unique identifier for a group)
pub type GroupId = String;

/// Sender ID (peer_id of the sender)
pub type SenderId = String;

/// Sender Key
///
/// Each group member has a sender key used to encrypt messages they send to the group.
/// The key ratchets forward with each message for forward secrecy.
#[derive(Debug, Clone)]
pub struct SenderKey {
    /// Sender ID (peer_id)
    pub sender_id: SenderId,

    /// Current chain key (32 bytes) - ratchets forward
    pub chain_key: [u8; 32],

    /// Message counter
    pub counter: u64,

    /// Timestamp of last use
    pub last_used_at: u64,
}

impl SenderKey {
    /// Create a new sender key from a random seed
    pub fn generate(sender_id: SenderId) -> Result<Self> {
        let mut seed = [0u8; 32];
        let mut rng = rand::rng();
        rng.fill_bytes(&mut seed);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(Self {
            sender_id,
            chain_key: seed,
            counter: 0,
            last_used_at: now,
        })
    }

    /// Create sender key from existing seed (for distribution)
    pub fn from_seed(sender_id: SenderId, seed: [u8; 32]) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            sender_id,
            chain_key: seed,
            counter: 0,
            last_used_at: now,
        }
    }

    /// Derive a message key from the current chain key
    fn derive_message_key(&self) -> Result<[u8; 32]> {
        let hkdf = Hkdf::<Sha256>::new(Some(b"mepassa-sender-key-v1"), &self.chain_key);

        let mut message_key = [0u8; 32];
        let info = format!("message-{}-{}", self.sender_id, self.counter);
        hkdf.expand(info.as_bytes(), &mut message_key)
            .map_err(|e| MePassaError::Crypto(format!("HKDF expand failed: {}", e)))?;

        Ok(message_key)
    }

    /// Ratchet forward the chain key
    fn ratchet_forward(&mut self) -> Result<()> {
        let hkdf = Hkdf::<Sha256>::new(Some(b"mepassa-sender-chain-v1"), &self.chain_key);

        let mut new_chain_key = [0u8; 32];
        hkdf.expand(b"next-chain", &mut new_chain_key)
            .map_err(|e| MePassaError::Crypto(format!("HKDF expand failed: {}", e)))?;

        self.chain_key = new_chain_key;
        self.counter += 1;

        self.last_used_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(())
    }

    /// Encrypt a message using this sender key
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<EncryptedMessage> {
        // Derive message key
        let message_key = self.derive_message_key()?;

        // Encrypt
        let encrypted = encrypt_message(plaintext, &message_key)?;

        // Ratchet forward
        self.ratchet_forward()?;

        Ok(encrypted)
    }

    /// Decrypt a message using this sender key
    pub fn decrypt(&mut self, encrypted: &EncryptedMessage) -> Result<Vec<u8>> {
        // Derive message key
        let message_key = self.derive_message_key()?;

        // Decrypt
        let plaintext = decrypt_message(encrypted, &message_key)?;

        // Ratchet forward
        self.ratchet_forward()?;

        Ok(plaintext)
    }

    /// Get the seed for distribution to other members
    pub fn seed(&self) -> [u8; 32] {
        self.chain_key
    }
}

/// Group Session
///
/// Represents a group with multiple members, each having their own sender key.
#[derive(Debug, Clone)]
pub struct GroupSession {
    /// Group ID
    pub group_id: GroupId,

    /// My sender key (for sending messages)
    pub my_sender_key: SenderKey,

    /// Other members' sender keys (for receiving messages)
    /// Map: sender_id -> SenderKey
    pub member_sender_keys: HashMap<SenderId, SenderKey>,

    /// Group creation timestamp
    pub created_at: u64,
}

impl GroupSession {
    /// Create a new group session
    pub fn new(group_id: GroupId, my_sender_id: SenderId) -> Result<Self> {
        let my_sender_key = SenderKey::generate(my_sender_id)?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(Self {
            group_id,
            my_sender_key,
            member_sender_keys: HashMap::new(),
            created_at: now,
        })
    }

    /// Restore a group session from an existing sender key seed
    pub fn from_seed(group_id: GroupId, my_sender_id: SenderId, seed: [u8; 32]) -> Self {
        let my_sender_key = SenderKey::from_seed(my_sender_id, seed);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            group_id,
            my_sender_key,
            member_sender_keys: HashMap::new(),
            created_at: now,
        }
    }

    /// Add a member's sender key to the group
    pub fn add_member(&mut self, sender_id: SenderId, sender_key_seed: [u8; 32]) {
        let sender_key = SenderKey::from_seed(sender_id.clone(), sender_key_seed);
        self.member_sender_keys.insert(sender_id, sender_key);
    }

    /// Remove a member from the group
    pub fn remove_member(&mut self, sender_id: &str) {
        self.member_sender_keys.remove(sender_id);
    }

    /// Get my sender key seed for distribution
    pub fn my_sender_key_seed(&self) -> [u8; 32] {
        self.my_sender_key.seed()
    }

    /// Encrypt a message to the group using my sender key
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<(SenderId, EncryptedMessage)> {
        let encrypted = self.my_sender_key.encrypt(plaintext)?;
        Ok((self.my_sender_key.sender_id.clone(), encrypted))
    }

    /// Decrypt a message from a group member
    pub fn decrypt(&mut self, sender_id: &str, encrypted: &EncryptedMessage) -> Result<Vec<u8>> {
        let sender_key = self
            .member_sender_keys
            .get_mut(sender_id)
            .ok_or_else(|| {
                MePassaError::Crypto(format!("Sender key not found for: {}", sender_id))
            })?;

        sender_key.decrypt(encrypted)
    }

    /// List all members in the group
    pub fn members(&self) -> Vec<String> {
        let mut members: Vec<String> = self.member_sender_keys.keys().cloned().collect();
        members.push(self.my_sender_key.sender_id.clone());
        members.sort();
        members
    }

    /// Get member count
    pub fn member_count(&self) -> usize {
        self.member_sender_keys.len() + 1 // +1 for myself
    }
}

/// Group Session Manager
///
/// Manages multiple group sessions.
#[derive(Debug, Clone)]
pub struct GroupSessionManager {
    /// My sender ID (peer_id)
    my_sender_id: SenderId,

    /// Group sessions
    /// Map: group_id -> GroupSession
    sessions: Arc<RwLock<HashMap<GroupId, GroupSession>>>,
}

impl GroupSessionManager {
    /// Create a new group session manager
    pub fn new(my_sender_id: SenderId) -> Self {
        Self {
            my_sender_id,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize a group session from an existing sender key seed
    pub fn init_group_with_seed(
        &self,
        group_id: GroupId,
        my_seed: [u8; 32],
        members: Vec<(SenderId, [u8; 32])>,
    ) -> Result<()> {
        let mut session = GroupSession::from_seed(
            group_id.clone(),
            self.my_sender_id.clone(),
            my_seed,
        );

        for (sender_id, seed) in members {
            if sender_id != self.my_sender_id {
                session.add_member(sender_id, seed);
            }
        }

        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| MePassaError::Crypto(format!("Lock error: {}", e)))?;

        sessions.insert(group_id, session);

        Ok(())
    }

    /// Create a new group
    pub fn create_group(&self, group_id: GroupId) -> Result<[u8; 32]> {
        let session = GroupSession::new(group_id.clone(), self.my_sender_id.clone())?;
        let my_seed = session.my_sender_key_seed();

        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| MePassaError::Crypto(format!("Lock error: {}", e)))?;

        sessions.insert(group_id, session);

        Ok(my_seed)
    }

    /// Join an existing group
    pub fn join_group(
        &self,
        group_id: GroupId,
        members: Vec<(SenderId, [u8; 32])>, // (sender_id, sender_key_seed)
    ) -> Result<[u8; 32]> {
        let mut session = GroupSession::new(group_id.clone(), self.my_sender_id.clone())?;
        let my_seed = session.my_sender_key_seed();

        // Add all existing members
        for (sender_id, seed) in members {
            if sender_id != self.my_sender_id {
                session.add_member(sender_id, seed);
            }
        }

        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| MePassaError::Crypto(format!("Lock error: {}", e)))?;

        sessions.insert(group_id, session);

        Ok(my_seed)
    }

    /// Add a member to an existing group
    pub fn add_member_to_group(
        &self,
        group_id: &str,
        sender_id: SenderId,
        sender_key_seed: [u8; 32],
    ) -> Result<()> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| MePassaError::Crypto(format!("Lock error: {}", e)))?;

        let session = sessions
            .get_mut(group_id)
            .ok_or_else(|| MePassaError::Crypto(format!("Group not found: {}", group_id)))?;

        session.add_member(sender_id, sender_key_seed);

        Ok(())
    }

    /// Remove a member from a group
    pub fn remove_member_from_group(&self, group_id: &str, sender_id: &str) -> Result<()> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| MePassaError::Crypto(format!("Lock error: {}", e)))?;

        let session = sessions
            .get_mut(group_id)
            .ok_or_else(|| MePassaError::Crypto(format!("Group not found: {}", group_id)))?;

        session.remove_member(sender_id);

        Ok(())
    }

    /// Remove a group session entirely
    pub fn remove_group(&self, group_id: &str) -> Result<()> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| MePassaError::Crypto(format!("Lock error: {}", e)))?;

        sessions.remove(group_id);

        Ok(())
    }

    /// Get my sender key seed for a group (to share with new members)
    pub fn my_sender_key_seed(&self, group_id: &str) -> Result<[u8; 32]> {
        let sessions = self
            .sessions
            .read()
            .map_err(|e| MePassaError::Crypto(format!("Lock error: {}", e)))?;

        let session = sessions
            .get(group_id)
            .ok_or_else(|| MePassaError::Crypto(format!("Group not found: {}", group_id)))?;

        Ok(session.my_sender_key_seed())
    }

    /// Encrypt a message to a group
    pub fn encrypt_to_group(
        &self,
        group_id: &str,
        plaintext: &[u8],
    ) -> Result<(SenderId, EncryptedMessage)> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| MePassaError::Crypto(format!("Lock error: {}", e)))?;

        let session = sessions
            .get_mut(group_id)
            .ok_or_else(|| MePassaError::Crypto(format!("Group not found: {}", group_id)))?;

        session.encrypt(plaintext)
    }

    /// Decrypt a message from a group
    pub fn decrypt_from_group(
        &self,
        group_id: &str,
        sender_id: &str,
        encrypted: &EncryptedMessage,
    ) -> Result<Vec<u8>> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| MePassaError::Crypto(format!("Lock error: {}", e)))?;

        let session = sessions
            .get_mut(group_id)
            .ok_or_else(|| MePassaError::Crypto(format!("Group not found: {}", group_id)))?;

        session.decrypt(sender_id, encrypted)
    }

    /// List all groups
    pub fn list_groups(&self) -> Result<Vec<String>> {
        let sessions = self
            .sessions
            .read()
            .map_err(|e| MePassaError::Crypto(format!("Lock error: {}", e)))?;

        Ok(sessions.keys().cloned().collect())
    }

    /// Get group member count
    pub fn group_member_count(&self, group_id: &str) -> Result<usize> {
        let sessions = self
            .sessions
            .read()
            .map_err(|e| MePassaError::Crypto(format!("Lock error: {}", e)))?;

        let session = sessions
            .get(group_id)
            .ok_or_else(|| MePassaError::Crypto(format!("Group not found: {}", group_id)))?;

        Ok(session.member_count())
    }

    /// List group members
    pub fn list_group_members(&self, group_id: &str) -> Result<Vec<String>> {
        let sessions = self
            .sessions
            .read()
            .map_err(|e| MePassaError::Crypto(format!("Lock error: {}", e)))?;

        let session = sessions
            .get(group_id)
            .ok_or_else(|| MePassaError::Crypto(format!("Group not found: {}", group_id)))?;

        Ok(session.members())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sender_key_generation() {
        let sender_key = SenderKey::generate("alice".to_string()).unwrap();

        assert_eq!(sender_key.sender_id, "alice");
        assert_eq!(sender_key.counter, 0);
        assert_ne!(sender_key.chain_key, [0u8; 32]); // Should be random
    }

    #[test]
    fn test_sender_key_encrypt_decrypt() {
        let mut sender_key = SenderKey::generate("alice".to_string()).unwrap();
        let seed = sender_key.seed();

        // Alice encrypts
        let plaintext = b"Group message!";
        let encrypted = sender_key.encrypt(plaintext).unwrap();
        assert_eq!(sender_key.counter, 1);

        // Bob has Alice's sender key and decrypts
        let mut bob_alice_key = SenderKey::from_seed("alice".to_string(), seed);
        let decrypted = bob_alice_key.decrypt(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
        assert_eq!(bob_alice_key.counter, 1);
    }

    #[test]
    fn test_group_session_creation() {
        let session = GroupSession::new("group_1".to_string(), "alice".to_string()).unwrap();

        assert_eq!(session.group_id, "group_1");
        assert_eq!(session.my_sender_key.sender_id, "alice");
        assert_eq!(session.member_count(), 1); // Just me
    }

    #[test]
    fn test_group_session_add_remove_members() {
        let mut session = GroupSession::new("group_1".to_string(), "alice".to_string()).unwrap();

        // Add Bob
        let bob_seed = [1u8; 32];
        session.add_member("bob".to_string(), bob_seed);
        assert_eq!(session.member_count(), 2);

        // Add Carol
        let carol_seed = [2u8; 32];
        session.add_member("carol".to_string(), carol_seed);
        assert_eq!(session.member_count(), 3);

        // Remove Bob
        session.remove_member("bob");
        assert_eq!(session.member_count(), 2);

        let members = session.members();
        assert!(members.contains(&"alice".to_string()));
        assert!(members.contains(&"carol".to_string()));
        assert!(!members.contains(&"bob".to_string()));
    }

    #[test]
    fn test_group_message_flow() {
        // Alice creates group
        let mut alice_session =
            GroupSession::new("group_1".to_string(), "alice".to_string()).unwrap();
        let alice_seed = alice_session.my_sender_key_seed();

        // Bob joins group with Alice's sender key
        let mut bob_session =
            GroupSession::new("group_1".to_string(), "bob".to_string()).unwrap();
        let bob_seed = bob_session.my_sender_key_seed();
        bob_session.add_member("alice".to_string(), alice_seed);

        // Alice adds Bob's sender key
        alice_session.add_member("bob".to_string(), bob_seed);

        // Alice sends message to group
        let alice_message = b"Hello group!";
        let (sender_id, encrypted) = alice_session.encrypt(alice_message).unwrap();
        assert_eq!(sender_id, "alice");

        // Bob receives and decrypts
        let decrypted = bob_session.decrypt(&sender_id, &encrypted).unwrap();
        assert_eq!(alice_message, decrypted.as_slice());
    }

    #[test]
    fn test_group_session_manager() {
        let alice_manager = GroupSessionManager::new("alice".to_string());
        let bob_manager = GroupSessionManager::new("bob".to_string());

        // Alice creates group
        let alice_seed = alice_manager.create_group("group_1".to_string()).unwrap();

        // Bob joins group with Alice's seed
        let bob_seed = bob_manager
            .join_group(
                "group_1".to_string(),
                vec![("alice".to_string(), alice_seed)],
            )
            .unwrap();

        // Alice adds Bob
        alice_manager
            .add_member_to_group("group_1", "bob".to_string(), bob_seed)
            .unwrap();

        // Alice sends message
        let message = b"Hello from Alice!";
        let (sender_id, encrypted) = alice_manager
            .encrypt_to_group("group_1", message)
            .unwrap();

        // Bob decrypts
        let decrypted = bob_manager
            .decrypt_from_group("group_1", &sender_id, &encrypted)
            .unwrap();

        assert_eq!(message, decrypted.as_slice());
    }

    #[test]
    fn test_group_with_three_members() {
        let alice_manager = GroupSessionManager::new("alice".to_string());
        let bob_manager = GroupSessionManager::new("bob".to_string());
        let carol_manager = GroupSessionManager::new("carol".to_string());

        // Alice creates group
        let alice_seed = alice_manager.create_group("group_1".to_string()).unwrap();

        // Bob joins
        let bob_seed = bob_manager
            .join_group(
                "group_1".to_string(),
                vec![("alice".to_string(), alice_seed)],
            )
            .unwrap();

        // Carol joins
        let carol_seed = carol_manager
            .join_group(
                "group_1".to_string(),
                vec![
                    ("alice".to_string(), alice_seed),
                    ("bob".to_string(), bob_seed),
                ],
            )
            .unwrap();

        // Alice adds Bob and Carol
        alice_manager
            .add_member_to_group("group_1", "bob".to_string(), bob_seed)
            .unwrap();
        alice_manager
            .add_member_to_group("group_1", "carol".to_string(), carol_seed)
            .unwrap();

        // Bob adds Carol
        bob_manager
            .add_member_to_group("group_1", "carol".to_string(), carol_seed)
            .unwrap();

        // Check member counts
        assert_eq!(alice_manager.group_member_count("group_1").unwrap(), 3);
        assert_eq!(bob_manager.group_member_count("group_1").unwrap(), 3);
        assert_eq!(carol_manager.group_member_count("group_1").unwrap(), 3);

        // Carol sends message
        let carol_message = b"Hi everyone!";
        let (sender_id, encrypted) = carol_manager
            .encrypt_to_group("group_1", carol_message)
            .unwrap();

        // Alice and Bob decrypt
        let alice_decrypted = alice_manager
            .decrypt_from_group("group_1", &sender_id, &encrypted)
            .unwrap();
        let bob_decrypted = bob_manager
            .decrypt_from_group("group_1", &sender_id, &encrypted)
            .unwrap();

        assert_eq!(carol_message, alice_decrypted.as_slice());
        assert_eq!(carol_message, bob_decrypted.as_slice());
    }

    #[test]
    fn test_list_groups() {
        let manager = GroupSessionManager::new("alice".to_string());

        manager.create_group("group_1".to_string()).unwrap();
        manager.create_group("group_2".to_string()).unwrap();
        manager.create_group("group_3".to_string()).unwrap();

        let groups = manager.list_groups().unwrap();
        assert_eq!(groups.len(), 3);
        assert!(groups.contains(&"group_1".to_string()));
        assert!(groups.contains(&"group_2".to_string()));
        assert!(groups.contains(&"group_3".to_string()));
    }

    #[test]
    fn test_sender_key_forward_secrecy() {
        let mut sender_key = SenderKey::generate("alice".to_string()).unwrap();

        // Save initial chain key
        let initial_chain_key = sender_key.chain_key;

        // Encrypt first message
        let msg1 = b"First message";
        let _encrypted1 = sender_key.encrypt(msg1).unwrap();

        // Chain key should have changed (forward secrecy)
        assert_ne!(sender_key.chain_key, initial_chain_key);
        assert_eq!(sender_key.counter, 1);

        // Save second chain key
        let second_chain_key = sender_key.chain_key;

        // Encrypt second message
        let msg2 = b"Second message";
        let _encrypted2 = sender_key.encrypt(msg2).unwrap();

        // Chain key should have changed again
        assert_ne!(sender_key.chain_key, second_chain_key);
        assert_eq!(sender_key.counter, 2);
    }
}
