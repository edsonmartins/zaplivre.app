//! PreKey management for Signal Protocol
//!
//! This module manages Signal prekeys and signed prekeys for session setup.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

use libsignal_protocol_syft::{
    kem, GenericSignedPreKey, IdentityKeyPair, KeyPair, KyberPreKeyId, KyberPreKeyRecord, PreKeyId,
    PreKeyRecord, SignedPreKeyId, SignedPreKeyRecord, Timestamp,
};
use rand::{rngs::StdRng, SeedableRng};

use crate::utils::error::{Result, ZapLivreError};

/// A single prekey record
#[derive(Clone)]
pub struct PreKey {
    /// Unique ID for this prekey
    pub id: u32,
    record: PreKeyRecord,
}

impl PreKey {
    /// Create a prekey from record
    pub fn from_record(record: PreKeyRecord) -> Result<Self> {
        let id = record
            .id()
            .map_err(|e| ZapLivreError::Identity(format!("PreKey id error: {}", e)))?;
        Ok(Self {
            id: id.into(),
            record,
        })
    }

    /// Get record
    pub fn record(&self) -> &PreKeyRecord {
        &self.record
    }

    /// Get serialized public key bytes
    pub fn public_key_bytes(&self) -> Result<Vec<u8>> {
        let public_key = self
            .record
            .public_key()
            .map_err(|e| ZapLivreError::Identity(format!("PreKey public key error: {}", e)))?;
        Ok(public_key.serialize().to_vec())
    }
}

impl std::fmt::Debug for PreKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreKey")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

/// Serializable prekey bundle for transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreKeyBundle {
    /// Identity key (Ed25519 public key as bytes)
    pub identity_key: [u8; 32],
    /// Signal identity key (serialized public key bytes)
    #[serde(default)]
    pub signal_identity_key: Option<Vec<u8>>,
    /// Signal registration id
    #[serde(default)]
    pub signal_registration_id: Option<u32>,
    /// Signal device id
    #[serde(default)]
    pub signal_device_id: Option<u32>,
    /// Signed prekey ID
    pub signed_prekey_id: u32,
    /// Signed prekey public bytes (serialized)
    pub signed_prekey: Vec<u8>,
    /// Signature over signed prekey
    pub signed_prekey_signature: Vec<u8>,
    /// Kyber prekey ID
    pub kyber_prekey_id: u32,
    /// Kyber prekey public bytes (serialized)
    pub kyber_prekey: Vec<u8>,
    /// Signature over Kyber prekey
    pub kyber_prekey_signature: Vec<u8>,
    /// One-time prekey (optional)
    pub one_time_prekey: Option<OneTimePreKey>,
}

/// One-time prekey (consumed after first use)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneTimePreKey {
    pub id: u32,
    pub public_key: Vec<u8>,
}

/// Pool of prekeys for key agreement
#[derive(Clone)]
pub struct PreKeyPool {
    /// Identity keypair (Ed25519) for app identity
    identity_keypair: crate::identity::Keypair,
    /// Signal identity keypair record (serialized)
    signal_identity_keypair_record: Vec<u8>,
    /// Signal registration id
    signal_registration_id: u32,
    /// Signal device id
    signal_device_id: u32,
    /// Signed prekey record
    signed_prekey: SignedPreKeyRecord,
    /// Kyber prekey record
    kyber_prekey: KyberPreKeyRecord,
    /// Pool of one-time prekeys
    one_time_prekeys: HashMap<u32, PreKeyRecord>,
    /// Next prekey ID to assign
    next_prekey_id: u32,
    /// Next signed prekey ID to assign
    next_signed_prekey_id: u32,
    /// Next kyber prekey ID to assign
    next_kyber_prekey_id: u32,
}

/// SEC-07: snapshot serializável do pool (records em bytes libsignal)
#[derive(Serialize, Deserialize)]
struct PreKeyPoolSnapshot {
    /// A identidade Signal TAMBÉM precisa ser estável entre restarts -
    /// sem ela o signed prekey restaurado não casa com a identity key nova
    signal_identity_keypair_record: Vec<u8>,
    signal_registration_id: u32,
    signal_device_id: u32,
    signed_prekey: Vec<u8>,
    kyber_prekey: Vec<u8>,
    one_time_prekeys: Vec<(u32, Vec<u8>)>,
    next_prekey_id: u32,
    next_signed_prekey_id: u32,
    next_kyber_prekey_id: u32,
}

impl PreKeyPool {
    /// Record serializado da identidade Signal deste pool
    pub fn signal_identity_keypair_record_bytes(&self) -> &[u8] {
        &self.signal_identity_keypair_record
    }

    /// Registration id Signal deste pool
    pub fn signal_registration_id_value(&self) -> u32 {
        self.signal_registration_id
    }

    /// SEC-07: serializa o pool para persistência (o bundle publicado deixa
    /// de mudar a cada restart)
    pub fn to_snapshot_bytes(&self) -> Result<Vec<u8>> {
        let snapshot = PreKeyPoolSnapshot {
            signal_identity_keypair_record: self.signal_identity_keypair_record.clone(),
            signal_registration_id: self.signal_registration_id,
            signal_device_id: self.signal_device_id,
            signed_prekey: self
                .signed_prekey
                .serialize()
                .map_err(|e| ZapLivreError::Crypto(format!("serialize signed prekey: {}", e)))?
                .to_vec(),
            kyber_prekey: self
                .kyber_prekey
                .serialize()
                .map_err(|e| ZapLivreError::Crypto(format!("serialize kyber prekey: {}", e)))?
                .to_vec(),
            one_time_prekeys: self
                .one_time_prekeys
                .iter()
                .map(|(id, record)| {
                    record
                        .serialize()
                        .map(|bytes| (*id, bytes.to_vec()))
                        .map_err(|e| ZapLivreError::Crypto(format!("serialize prekey: {}", e)))
                })
                .collect::<Result<Vec<_>>>()?,
            next_prekey_id: self.next_prekey_id,
            next_signed_prekey_id: self.next_signed_prekey_id,
            next_kyber_prekey_id: self.next_kyber_prekey_id,
        };
        serde_json::to_vec(&snapshot)
            .map_err(|e| ZapLivreError::Storage(format!("serialize prekey pool: {}", e)))
    }

    /// SEC-07: reconstrói o pool a partir de um snapshot persistido.
    /// O record da identidade Signal e o registration id vêm do snapshot
    /// (precisam ser os MESMOS que assinaram o signed prekey).
    pub fn from_snapshot_bytes(
        identity_keypair: crate::identity::Keypair,
        bytes: &[u8],
    ) -> Result<Self> {
        let snapshot: PreKeyPoolSnapshot = serde_json::from_slice(bytes)
            .map_err(|e| ZapLivreError::Storage(format!("deserialize prekey pool: {}", e)))?;

        let signed_prekey = SignedPreKeyRecord::deserialize(&snapshot.signed_prekey)
            .map_err(|e| ZapLivreError::Crypto(format!("deserialize signed prekey: {}", e)))?;
        let kyber_prekey = KyberPreKeyRecord::deserialize(&snapshot.kyber_prekey)
            .map_err(|e| ZapLivreError::Crypto(format!("deserialize kyber prekey: {}", e)))?;
        let one_time_prekeys = snapshot
            .one_time_prekeys
            .iter()
            .map(|(id, bytes)| {
                PreKeyRecord::deserialize(bytes)
                    .map(|r| (*id, r))
                    .map_err(|e| ZapLivreError::Crypto(format!("deserialize prekey: {}", e)))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        Ok(Self {
            identity_keypair,
            signal_identity_keypair_record: snapshot.signal_identity_keypair_record,
            signal_registration_id: snapshot.signal_registration_id,
            signal_device_id: snapshot.signal_device_id,
            signed_prekey,
            kyber_prekey,
            one_time_prekeys,
            next_prekey_id: snapshot.next_prekey_id,
            next_signed_prekey_id: snapshot.next_signed_prekey_id,
            next_kyber_prekey_id: snapshot.next_kyber_prekey_id,
        })
    }

    /// Create a new prekey pool with initial prekeys
    pub fn new(
        identity_keypair: crate::identity::Keypair,
        signal_identity_keypair_record: Vec<u8>,
        signal_registration_id: u32,
        pool_size: usize,
    ) -> Self {
        let identity_keypair_signal =
            IdentityKeyPair::try_from(signal_identity_keypair_record.as_slice())
                .expect("Failed to deserialize Signal identity keypair");
        let mut rng = StdRng::from_os_rng();

        let signed_key_pair = KeyPair::generate(&mut rng);
        let signature = identity_keypair_signal
            .private_key()
            .calculate_signature(&signed_key_pair.public_key.serialize(), &mut rng)
            .expect("Failed to sign signed prekey")
            .into_vec();
        let signed_prekey_id = 1u32;
        let signed_prekey = SignedPreKeyRecord::new(
            SignedPreKeyId::from(signed_prekey_id),
            Timestamp::from_epoch_millis(current_millis()),
            &signed_key_pair,
            &signature,
        );

        let kyber_prekey_id = 1u32;
        let kyber_prekey = KyberPreKeyRecord::generate(
            kem::KeyType::Kyber1024,
            KyberPreKeyId::from(kyber_prekey_id),
            identity_keypair_signal.private_key(),
        )
        .expect("Failed to generate Kyber prekey");

        let mut pool = Self {
            identity_keypair,
            signal_identity_keypair_record,
            signal_registration_id,
            signal_device_id: 1,
            signed_prekey,
            kyber_prekey,
            one_time_prekeys: HashMap::new(),
            next_prekey_id: 2,
            next_signed_prekey_id: 2,
            next_kyber_prekey_id: 2,
        };

        pool.replenish_prekeys(pool_size);
        pool
    }

    /// Replenish one-time prekeys to reach target count
    pub fn replenish_prekeys(&mut self, target_count: usize) {
        let current_count = self.one_time_prekeys.len();

        if current_count >= target_count {
            return;
        }

        let to_generate = target_count - current_count;
        let mut rng = StdRng::from_os_rng();

        for _ in 0..to_generate {
            let id = self.next_prekey_id;
            self.next_prekey_id += 1;
            let key_pair = KeyPair::generate(&mut rng);
            let record = PreKeyRecord::new(PreKeyId::from(id), &key_pair);
            self.one_time_prekeys.insert(id, record);
        }
    }

    /// Get a prekey bundle for key exchange
    pub fn get_bundle(&self) -> Result<PreKeyBundle> {
        let identity_keypair_signal = IdentityKeyPair::try_from(
            self.signal_identity_keypair_record.as_slice(),
        )
        .map_err(|e| {
            ZapLivreError::Identity(format!("Signal identity keypair deserialize failed: {}", e))
        })?;
        let signal_identity_key = identity_keypair_signal.identity_key().serialize().to_vec();

        let signed_prekey_public = self
            .signed_prekey
            .public_key()
            .map_err(|e| ZapLivreError::Identity(format!("Signed prekey public error: {}", e)))?
            .serialize()
            .to_vec();
        let signed_prekey_signature = self.signed_prekey.signature().map_err(|e| {
            ZapLivreError::Identity(format!("Signed prekey signature error: {}", e))
        })?;

        let kyber_prekey_public = self
            .kyber_prekey
            .public_key()
            .map_err(|e| ZapLivreError::Identity(format!("Kyber prekey public error: {}", e)))?
            .serialize()
            .to_vec();
        let kyber_prekey_signature = self
            .kyber_prekey
            .signature()
            .map_err(|e| ZapLivreError::Identity(format!("Kyber prekey signature error: {}", e)))?;

        let one_time_prekey = self
            .peek_one_time_prekey()
            .map(|record| -> Result<OneTimePreKey> {
                let prekey = PreKey::from_record(record.clone())?;
                Ok(OneTimePreKey {
                    id: prekey.id,
                    public_key: prekey.public_key_bytes()?,
                })
            })
            .transpose()?;

        Ok(PreKeyBundle {
            identity_key: self.identity_keypair.public_key_bytes(),
            signal_identity_key: Some(signal_identity_key),
            signal_registration_id: Some(self.signal_registration_id),
            signal_device_id: Some(self.signal_device_id),
            signed_prekey_id: self
                .signed_prekey
                .id()
                .map_err(|e| ZapLivreError::Identity(format!("Signed prekey id error: {}", e)))?
                .into(),
            signed_prekey: signed_prekey_public,
            signed_prekey_signature,
            kyber_prekey_id: self
                .kyber_prekey
                .id()
                .map_err(|e| ZapLivreError::Identity(format!("Kyber prekey id error: {}", e)))?
                .into(),
            kyber_prekey: kyber_prekey_public,
            kyber_prekey_signature,
            one_time_prekey,
        })
    }

    /// Peek at a one-time prekey without consuming it
    fn peek_one_time_prekey(&self) -> Option<&PreKeyRecord> {
        self.one_time_prekeys.values().next()
    }

    /// Get a specific one-time prekey by ID
    pub fn get_prekey(&self, id: u32) -> Option<&PreKeyRecord> {
        self.one_time_prekeys.get(&id)
    }

    /// Store a prekey record
    pub fn store_prekey_record(&mut self, id: u32, record: PreKeyRecord) {
        self.one_time_prekeys.insert(id, record);
        self.next_prekey_id = self.next_prekey_id.max(id + 1);
    }

    /// Remove a specific one-time prekey after use
    pub fn remove_prekey(&mut self, id: u32) -> Option<PreKeyRecord> {
        self.one_time_prekeys.remove(&id)
    }

    /// Get signed prekey record
    pub fn signed_prekey_record(&self) -> &SignedPreKeyRecord {
        &self.signed_prekey
    }

    /// Store signed prekey record
    pub fn store_signed_prekey_record(&mut self, id: u32, record: SignedPreKeyRecord) {
        self.signed_prekey = record;
        self.next_signed_prekey_id = self.next_signed_prekey_id.max(id + 1);
    }

    /// Get Kyber prekey record
    pub fn kyber_prekey_record(&self) -> &KyberPreKeyRecord {
        &self.kyber_prekey
    }

    /// Store Kyber prekey record
    pub fn store_kyber_prekey_record(&mut self, id: u32, record: KyberPreKeyRecord) {
        self.kyber_prekey = record;
        self.next_kyber_prekey_id = self.next_kyber_prekey_id.max(id + 1);
    }

    /// Rotate the signed prekey and Kyber prekey
    pub fn rotate_signed_prekey(&mut self) {
        let identity_keypair_signal =
            IdentityKeyPair::try_from(self.signal_identity_keypair_record.as_slice())
                .expect("Failed to deserialize Signal identity keypair");
        let mut rng = StdRng::from_os_rng();

        let signed_key_pair = KeyPair::generate(&mut rng);
        let signature = identity_keypair_signal
            .private_key()
            .calculate_signature(&signed_key_pair.public_key.serialize(), &mut rng)
            .expect("Failed to sign signed prekey")
            .into_vec();
        let signed_prekey_id = self.next_signed_prekey_id;
        self.next_signed_prekey_id += 1;
        self.signed_prekey = SignedPreKeyRecord::new(
            SignedPreKeyId::from(signed_prekey_id),
            Timestamp::from_epoch_millis(current_millis()),
            &signed_key_pair,
            &signature,
        );

        let kyber_prekey_id = self.next_kyber_prekey_id;
        self.next_kyber_prekey_id += 1;
        self.kyber_prekey = KyberPreKeyRecord::generate(
            kem::KeyType::Kyber1024,
            KyberPreKeyId::from(kyber_prekey_id),
            identity_keypair_signal.private_key(),
        )
        .expect("Failed to generate Kyber prekey");
    }

    /// Get count of remaining one-time prekeys
    pub fn prekey_count(&self) -> usize {
        self.one_time_prekeys.len()
    }

    /// Check if prekey pool needs replenishment
    pub fn needs_replenishment(&self) -> bool {
        self.prekey_count() < 20
    }
}

fn current_millis() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Identity;

    #[test]
    fn test_prekey_pool_creation() {
        let identity = Identity::generate(10);
        let pool = identity.prekey_pool().unwrap();
        assert!(pool.prekey_count() > 0);
    }

    #[test]
    fn test_prekey_bundle_serialization() {
        let identity = Identity::generate(5);
        let mut identity_mut = identity.clone();
        let bundle = identity_mut
            .prekey_pool_mut()
            .unwrap()
            .get_bundle()
            .unwrap();

        let json = serde_json::to_string(&bundle).unwrap();
        let deserialized: PreKeyBundle = serde_json::from_str(&json).unwrap();

        assert_eq!(bundle.signed_prekey_id, deserialized.signed_prekey_id);
        assert_eq!(bundle.identity_key, deserialized.identity_key);
    }
}
