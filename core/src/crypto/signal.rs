//! Signal Protocol integration (libsignal-protocol-syft).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use async_trait::async_trait;
use rand::{rngs::StdRng, SeedableRng};

use libsignal_protocol_syft::{
    kem, message_decrypt, message_encrypt, process_prekey_bundle, CiphertextMessage,
    CiphertextMessageType, DeviceId, Direction, GenericSignedPreKey, IdentityChange, IdentityKey,
    IdentityKeyPair, IdentityKeyStore, KyberPreKeyId, KyberPreKeyRecord, KyberPreKeyStore,
    PreKeyBundle, PreKeyBundleContent, PreKeyId, PreKeyRecord, PreKeySignalMessage, PreKeyStore,
    ProtocolAddress, PublicKey, SessionRecord, SessionStore, SignalMessage, SignalProtocolError,
    SignedPreKeyId, SignedPreKeyRecord, SignedPreKeyStore,
};
use tokio::sync::RwLock;

use crate::identity::{Identity, PreKeyBundle as CorePreKeyBundle};
use crate::utils::error::{Result, ZapLivreError};

/// Encrypted message payload produced by Signal
#[derive(Debug, Clone)]
pub struct SignalEncryptedMessage {
    pub ciphertext: Vec<u8>,
    pub ciphertext_type: u32,
    pub sender_device_id: u32,
}

#[derive(Clone)]
pub struct SignalSessionManager {
    store: SignalStore,
}

impl SignalSessionManager {
    pub fn new(identity: Arc<RwLock<Identity>>) -> Self {
        Self {
            store: SignalStore::new(identity),
        }
    }

    /// Anexa persistência SQLite (SEC-04): sessões Signal e identidades TOFU
    /// sobrevivem a restart. Sessões são cifradas com a storage key; as
    /// identidades confiadas são chaves públicas (armazenadas como estão).
    /// Carrega o estado existente para a memória.
    pub async fn attach_persistence(
        &self,
        db: crate::storage::Database,
        storage_key: [u8; 32],
    ) -> Result<()> {
        // Carregar sessões persistidas
        {
            let rows: Vec<(String, Vec<u8>)> = {
                let conn = db.conn();
                let mut stmt = conn
                    .prepare("SELECT address, record FROM signal_sessions")
                    .map_err(|e| ZapLivreError::Storage(e.to_string()))?;
                let result = stmt
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                    .map_err(|e| ZapLivreError::Storage(e.to_string()))?
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|e| ZapLivreError::Storage(e.to_string()))?;
                result
            };

            let mut sessions = self.store.inner.sessions.write().await;
            for (address, blob) in rows {
                match crate::crypto::storage::decrypt_for_storage(&storage_key, &blob).and_then(
                    |bytes| {
                        SessionRecord::deserialize(&bytes).map_err(|e| {
                            ZapLivreError::Crypto(format!("Invalid session record: {}", e))
                        })
                    },
                ) {
                    Ok(record) => {
                        sessions.insert(address, record);
                    }
                    Err(e) => tracing::warn!("Failed to restore signal session: {}", e),
                }
            }
            if !sessions.is_empty() {
                tracing::info!(
                    "🔐 Restored {} Signal sessions from storage",
                    sessions.len()
                );
            }
        }

        // Carregar identidades TOFU
        {
            let rows: Vec<(String, Vec<u8>)> = {
                let conn = db.conn();
                let mut stmt = conn
                    .prepare("SELECT address, identity_key FROM signal_identities")
                    .map_err(|e| ZapLivreError::Storage(e.to_string()))?;
                let result = stmt
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                    .map_err(|e| ZapLivreError::Storage(e.to_string()))?
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|e| ZapLivreError::Storage(e.to_string()))?;
                result
            };

            let mut identities = self.store.inner.trusted_identities.write().await;
            for (address, bytes) in rows {
                match IdentityKey::decode(&bytes) {
                    Ok(key) => {
                        identities.insert(address, key);
                    }
                    Err(e) => tracing::warn!("Failed to restore trusted identity: {}", e),
                }
            }
        }

        let mut persistence = self.store.inner.persistence.write().await;
        *persistence = Some((db, storage_key));
        Ok(())
    }

    pub async fn has_session(&self, peer_id: &str, device_id: u32) -> Result<bool> {
        let address = protocol_address(peer_id, device_id)?;
        let key = address_key(&address);
        let sessions = self.store.inner.sessions.read().await;
        Ok(sessions.contains_key(&key))
    }

    pub async fn encrypt_for(
        &self,
        peer_id: &str,
        device_id: u32,
        bundle: Option<&CorePreKeyBundle>,
        plaintext: &[u8],
    ) -> Result<SignalEncryptedMessage> {
        let address = protocol_address(peer_id, device_id)?;

        if !self.has_session(peer_id, device_id).await? {
            let Some(bundle) = bundle else {
                return Err(ZapLivreError::Crypto(
                    "Missing prekey bundle for Signal session".to_string(),
                ));
            };
            let signal_bundle = to_signal_bundle(bundle)?;
            let mut session_store = self.store.handle();
            let mut identity_store = self.store.handle();
            let mut rng = StdRng::from_os_rng();
            process_prekey_bundle(
                &address,
                &mut session_store,
                &mut identity_store,
                &signal_bundle,
                SystemTime::now(),
                &mut rng,
            )
            .await
            .map_err(signal_error)?;
        }

        let mut session_store = self.store.handle();
        let mut identity_store = self.store.handle();
        let mut rng = StdRng::from_os_rng();
        let message = message_encrypt(
            plaintext,
            &address,
            &mut session_store,
            &mut identity_store,
            SystemTime::now(),
            &mut rng,
        )
        .await
        .map_err(signal_error)?;

        Ok(SignalEncryptedMessage {
            ciphertext: message.serialize().to_vec(),
            ciphertext_type: message.message_type() as u32,
            sender_device_id: device_id,
        })
    }

    pub async fn decrypt_from(
        &self,
        peer_id: &str,
        device_id: u32,
        encrypted: &SignalEncryptedMessage,
    ) -> Result<Vec<u8>> {
        let address = protocol_address(peer_id, device_id)?;
        let ciphertext_type = CiphertextMessageType::try_from(encrypted.ciphertext_type as u8)
            .map_err(|_| {
                ZapLivreError::Crypto(format!(
                    "Unsupported ciphertext type: {}",
                    encrypted.ciphertext_type
                ))
            })?;

        let ciphertext = match ciphertext_type {
            CiphertextMessageType::Whisper => {
                let msg = SignalMessage::try_from(encrypted.ciphertext.as_slice())
                    .map_err(signal_error)?;
                CiphertextMessage::SignalMessage(msg)
            }
            CiphertextMessageType::PreKey => {
                let msg = PreKeySignalMessage::try_from(encrypted.ciphertext.as_slice())
                    .map_err(signal_error)?;
                CiphertextMessage::PreKeySignalMessage(msg)
            }
            _ => {
                return Err(ZapLivreError::Crypto(format!(
                    "Unsupported ciphertext type: {:?}",
                    ciphertext_type
                )));
            }
        };

        let mut session_store = self.store.handle();
        let mut identity_store = self.store.handle();
        let mut pre_key_store = self.store.handle();
        let signed_pre_key_store = self.store.handle();
        let mut kyber_pre_key_store = self.store.handle();
        let mut rng = StdRng::from_os_rng();

        message_decrypt(
            &ciphertext,
            &address,
            &mut session_store,
            &mut identity_store,
            &mut pre_key_store,
            &signed_pre_key_store,
            &mut kyber_pre_key_store,
            &mut rng,
        )
        .await
        .map_err(signal_error)
    }
}

#[derive(Clone)]
struct SignalStore {
    inner: Arc<SignalStoreInner>,
}

impl SignalStore {
    fn new(identity: Arc<RwLock<Identity>>) -> Self {
        Self {
            inner: Arc::new(SignalStoreInner {
                identity,
                sessions: RwLock::new(HashMap::new()),
                trusted_identities: RwLock::new(HashMap::new()),
                persistence: RwLock::new(None),
            }),
        }
    }

    fn handle(&self) -> SignalStoreHandle {
        SignalStoreHandle {
            inner: Arc::clone(&self.inner),
        }
    }
}

struct SignalStoreInner {
    identity: Arc<RwLock<Identity>>,
    sessions: RwLock<HashMap<String, SessionRecord>>,
    trusted_identities: RwLock<HashMap<String, IdentityKey>>,
    /// Persistência SQLite (db + storage key) - SEC-04
    persistence: RwLock<Option<(crate::storage::Database, [u8; 32])>>,
}

impl SignalStoreInner {
    /// Persiste uma sessão (cifrada) - falha só gera warning para não
    /// bloquear o fluxo de mensagens
    async fn persist_session(&self, address: &str, record: &SessionRecord) {
        let persistence = self.persistence.read().await;
        let Some((db, storage_key)) = persistence.as_ref() else {
            return;
        };

        let result = record
            .serialize()
            .map_err(|e| ZapLivreError::Crypto(format!("Session serialize failed: {}", e)))
            .and_then(|bytes| crate::crypto::storage::encrypt_for_storage(storage_key, &bytes))
            .and_then(|blob| {
                db.conn()
                    .execute(
                        r#"
                        INSERT INTO signal_sessions (address, record, updated_at)
                        VALUES (?1, ?2, unixepoch())
                        ON CONFLICT(address) DO UPDATE SET
                            record = excluded.record, updated_at = unixepoch()
                        "#,
                        rusqlite::params![address, blob],
                    )
                    .map_err(|e| ZapLivreError::Storage(e.to_string()))
            });

        if let Err(e) = result {
            tracing::warn!("Failed to persist signal session: {}", e);
        }
    }

    /// Persiste uma identidade TOFU (chave pública)
    async fn persist_identity(&self, address: &str, identity: &IdentityKey) {
        let persistence = self.persistence.read().await;
        let Some((db, _)) = persistence.as_ref() else {
            return;
        };

        let bytes = identity.serialize();
        let result = db.conn().execute(
            r#"
            INSERT INTO signal_identities (address, identity_key)
            VALUES (?1, ?2)
            ON CONFLICT(address) DO UPDATE SET identity_key = excluded.identity_key
            "#,
            rusqlite::params![address, bytes.as_ref()],
        );
        if let Err(e) = result {
            tracing::warn!("Failed to persist trusted identity: {}", e);
        }
    }
}

#[derive(Clone)]
struct SignalStoreHandle {
    inner: Arc<SignalStoreInner>,
}

#[async_trait(?Send)]
impl IdentityKeyStore for SignalStoreHandle {
    async fn get_identity_key_pair(
        &self,
    ) -> libsignal_protocol_syft::error::Result<IdentityKeyPair> {
        let identity = self.inner.identity.read().await;
        IdentityKeyPair::try_from(identity.signal_identity_keypair_record()).map_err(|e| e)
    }

    async fn get_local_registration_id(&self) -> libsignal_protocol_syft::error::Result<u32> {
        let identity = self.inner.identity.read().await;
        Ok(identity.signal_registration_id())
    }

    async fn save_identity(
        &mut self,
        address: &ProtocolAddress,
        identity: &IdentityKey,
    ) -> libsignal_protocol_syft::error::Result<IdentityChange> {
        let key = address_key(address);
        let changed = {
            let mut map = self.inner.trusted_identities.write().await;
            map.insert(key.clone(), *identity).is_some()
        };
        // SEC-04: persistir o pinning TOFU para sobreviver a restart
        self.inner.persist_identity(&key, identity).await;
        Ok(IdentityChange::from_changed(changed))
    }

    async fn is_trusted_identity(
        &self,
        address: &ProtocolAddress,
        identity: &IdentityKey,
        _direction: Direction,
    ) -> libsignal_protocol_syft::error::Result<bool> {
        let key = address_key(address);
        let map = self.inner.trusted_identities.read().await;
        Ok(match map.get(&key) {
            Some(existing) => existing == identity,
            None => true,
        })
    }

    async fn get_identity(
        &self,
        address: &ProtocolAddress,
    ) -> libsignal_protocol_syft::error::Result<Option<IdentityKey>> {
        let key = address_key(address);
        let map = self.inner.trusted_identities.read().await;
        Ok(map.get(&key).copied())
    }
}

#[async_trait(?Send)]
impl PreKeyStore for SignalStoreHandle {
    async fn get_pre_key(
        &self,
        prekey_id: PreKeyId,
    ) -> libsignal_protocol_syft::error::Result<PreKeyRecord> {
        let identity = self.inner.identity.read().await;
        let Some(pool) = identity.prekey_pool() else {
            return Err(SignalProtocolError::InvalidArgument(
                "Missing prekey pool".to_string(),
            ));
        };
        let id: u32 = prekey_id.into();
        pool.get_prekey(id)
            .cloned()
            .ok_or_else(|| SignalProtocolError::InvalidArgument("Missing prekey".to_string()))
    }

    async fn save_pre_key(
        &mut self,
        prekey_id: PreKeyId,
        record: &PreKeyRecord,
    ) -> libsignal_protocol_syft::error::Result<()> {
        let mut identity = self.inner.identity.write().await;
        identity.init_prekey_pool(100);
        let Some(pool) = identity.prekey_pool_mut() else {
            return Err(SignalProtocolError::InvalidArgument(
                "Missing prekey pool".to_string(),
            ));
        };
        let id: u32 = prekey_id.into();
        pool.store_prekey_record(id, record.clone());
        Ok(())
    }

    async fn remove_pre_key(
        &mut self,
        prekey_id: PreKeyId,
    ) -> libsignal_protocol_syft::error::Result<()> {
        let mut identity = self.inner.identity.write().await;
        let Some(pool) = identity.prekey_pool_mut() else {
            return Ok(());
        };
        let id: u32 = prekey_id.into();
        pool.remove_prekey(id);
        Ok(())
    }
}

#[async_trait(?Send)]
impl SignedPreKeyStore for SignalStoreHandle {
    async fn get_signed_pre_key(
        &self,
        signed_prekey_id: SignedPreKeyId,
    ) -> libsignal_protocol_syft::error::Result<SignedPreKeyRecord> {
        let identity = self.inner.identity.read().await;
        let Some(pool) = identity.prekey_pool() else {
            return Err(SignalProtocolError::InvalidArgument(
                "Missing prekey pool".to_string(),
            ));
        };
        let id: u32 = signed_prekey_id.into();
        let record = pool.signed_prekey_record().clone();
        let record_id: u32 = record.id()?.into();
        if record_id != id {
            return Err(SignalProtocolError::InvalidArgument(
                "Signed prekey id mismatch".to_string(),
            ));
        }
        Ok(record)
    }

    async fn save_signed_pre_key(
        &mut self,
        signed_prekey_id: SignedPreKeyId,
        record: &SignedPreKeyRecord,
    ) -> libsignal_protocol_syft::error::Result<()> {
        let mut identity = self.inner.identity.write().await;
        identity.init_prekey_pool(100);
        let Some(pool) = identity.prekey_pool_mut() else {
            return Err(SignalProtocolError::InvalidArgument(
                "Missing prekey pool".to_string(),
            ));
        };
        let id: u32 = signed_prekey_id.into();
        pool.store_signed_prekey_record(id, record.clone());
        Ok(())
    }
}

#[async_trait(?Send)]
impl KyberPreKeyStore for SignalStoreHandle {
    async fn get_kyber_pre_key(
        &self,
        kyber_prekey_id: KyberPreKeyId,
    ) -> libsignal_protocol_syft::error::Result<KyberPreKeyRecord> {
        let identity = self.inner.identity.read().await;
        let Some(pool) = identity.prekey_pool() else {
            return Err(SignalProtocolError::InvalidArgument(
                "Missing prekey pool".to_string(),
            ));
        };
        let id: u32 = kyber_prekey_id.into();
        let record = pool.kyber_prekey_record().clone();
        let record_id: u32 = record.id()?.into();
        if record_id != id {
            return Err(SignalProtocolError::InvalidArgument(
                "Kyber prekey id mismatch".to_string(),
            ));
        }
        Ok(record)
    }

    async fn save_kyber_pre_key(
        &mut self,
        kyber_prekey_id: KyberPreKeyId,
        record: &KyberPreKeyRecord,
    ) -> libsignal_protocol_syft::error::Result<()> {
        let mut identity = self.inner.identity.write().await;
        identity.init_prekey_pool(100);
        let Some(pool) = identity.prekey_pool_mut() else {
            return Err(SignalProtocolError::InvalidArgument(
                "Missing prekey pool".to_string(),
            ));
        };
        let id: u32 = kyber_prekey_id.into();
        pool.store_kyber_prekey_record(id, record.clone());
        Ok(())
    }

    async fn mark_kyber_pre_key_used(
        &mut self,
        _kyber_prekey_id: KyberPreKeyId,
        _ec_prekey_id: SignedPreKeyId,
        _base_key: &PublicKey,
    ) -> libsignal_protocol_syft::error::Result<()> {
        Ok(())
    }
}

#[async_trait(?Send)]
impl SessionStore for SignalStoreHandle {
    async fn load_session(
        &self,
        address: &ProtocolAddress,
    ) -> libsignal_protocol_syft::error::Result<Option<SessionRecord>> {
        let key = address_key(address);
        let map = self.inner.sessions.read().await;
        Ok(map.get(&key).cloned())
    }

    async fn store_session(
        &mut self,
        address: &ProtocolAddress,
        record: &SessionRecord,
    ) -> libsignal_protocol_syft::error::Result<()> {
        let key = address_key(address);
        {
            let mut map = self.inner.sessions.write().await;
            map.insert(key.clone(), record.clone());
        }
        // SEC-04: persistir (cifrado) para sobreviver a restart
        self.inner.persist_session(&key, record).await;
        Ok(())
    }
}

fn protocol_address(peer_id: &str, device_id: u32) -> Result<ProtocolAddress> {
    let device = to_device_id(device_id)?;
    Ok(ProtocolAddress::new(peer_id.to_string(), device))
}

fn address_key(address: &ProtocolAddress) -> String {
    format!("{}:{}", address.name(), address.device_id())
}

fn to_signal_bundle(bundle: &CorePreKeyBundle) -> Result<PreKeyBundle> {
    let signal_identity_key = bundle
        .signal_identity_key
        .as_ref()
        .ok_or_else(|| ZapLivreError::Crypto("Missing Signal identity key".to_string()))?;
    let identity_key =
        IdentityKey::try_from(signal_identity_key.as_slice()).map_err(signal_error)?;

    let signed_prekey_public =
        PublicKey::deserialize(&bundle.signed_prekey).map_err(signal_error)?;
    let kyber_prekey_public =
        kem::PublicKey::deserialize(&bundle.kyber_prekey).map_err(signal_error)?;

    let pre_key = if let Some(opk) = &bundle.one_time_prekey {
        let public = PublicKey::deserialize(&opk.public_key).map_err(signal_error)?;
        Some((PreKeyId::from(opk.id), public))
    } else {
        None
    };

    let device_id = match bundle.signal_device_id {
        Some(id) => Some(to_device_id(id)?),
        None => None,
    };

    let content = PreKeyBundleContent {
        registration_id: bundle.signal_registration_id,
        device_id,
        pre_key_id: pre_key.as_ref().map(|(id, _)| *id),
        pre_key_public: pre_key.map(|(_, public)| public),
        signed_pre_key_id: Some(SignedPreKeyId::from(bundle.signed_prekey_id)),
        signed_pre_key_public: Some(signed_prekey_public),
        signed_pre_key_signature: Some(bundle.signed_prekey_signature.clone()),
        identity_key: Some(identity_key),
        kyber_pre_key_id: Some(KyberPreKeyId::from(bundle.kyber_prekey_id)),
        kyber_pre_key_public: Some(kyber_prekey_public),
        kyber_pre_key_signature: Some(bundle.kyber_prekey_signature.clone()),
    };

    PreKeyBundle::try_from(content).map_err(signal_error)
}

fn to_device_id(device_id: u32) -> Result<DeviceId> {
    let id: u8 = device_id
        .try_into()
        .map_err(|_| ZapLivreError::Crypto("Invalid device id".to_string()))?;
    DeviceId::new(id).map_err(|_| ZapLivreError::Crypto("Invalid device id".to_string()))
}

fn signal_error<E: std::fmt::Display>(err: E) -> ZapLivreError {
    ZapLivreError::Crypto(format!("Signal error: {}", err))
}
