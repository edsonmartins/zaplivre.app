//! Client API
//!
//! Public API for MePassa client.

use libp2p::{Multiaddr, PeerId};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};

use base64::{engine::general_purpose, Engine as _};
use super::events::{ClientEvent, EventCallback};
use crate::{
    crypto::{decrypt_for_storage, encrypt_for_storage, SignalSessionManager},
    identity::Identity,
    media::MediaEnvelope,
    network::NetworkManager,
    protocol::{pb::message::Payload, EncryptedMessage as ProtoEncryptedMessage, MediaOffer, MediaRequest, Message, MessageType, TextMessage},
    reactions::ReactionEnvelope,
    storage::{contacts::{NewContact, UpdateContact}, Database, MediaType, MessageStatus, NewMessage, StorageError},
    utils::error::{MePassaError, Result},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(any(feature = "voip", feature = "video"))]
use crate::voip::{CallManager, VoIPIntegration};

const MAX_INLINE_MEDIA_BYTES: usize = 512 * 1024;

/// MePassa Client
///
/// Main entry point for using the MePassa P2P messaging platform.
pub struct Client {
    /// Local peer ID (libp2p)
    peer_id: PeerId,
    /// Local identity (keypair + prekeys)
    identity: Arc<RwLock<Identity>>,
    /// Network manager (P2P networking)
    network: Arc<RwLock<NetworkManager>>,
    /// Local storage (SQLite) - shares connection with MessageHandler via Database::clone()
    database: Database,
    /// Event callbacks
    callbacks: Arc<RwLock<Vec<Box<dyn EventCallback>>>>,
    /// Data directory
    data_dir: PathBuf,
    /// Call manager (VoIP)
    #[cfg(any(feature = "voip", feature = "video"))]
    call_manager: Arc<CallManager>,
    /// VoIP integration (network ↔ calls)
    #[cfg(any(feature = "voip", feature = "video"))]
    voip_integration: Arc<VoIPIntegration>,
    /// Group manager (FASE 15)
    group_manager: Arc<crate::group::GroupManager>,
    /// E2E session manager
    session_manager: SignalSessionManager,
    /// Storage encryption key
    storage_key: [u8; 32],
    /// Optional message store URL for offline delivery
    message_store_url: Option<String>,
    /// HTTP client for message store
    message_store_http: reqwest::Client,
    /// Message handler (for processing offline messages)
    message_handler: Arc<crate::network::MessageHandler>,
}

impl Client {
    /// Create a new client (use ClientBuilder instead)
    pub(crate) fn new(
        peer_id: PeerId,
        identity: Arc<RwLock<Identity>>,
        network: Arc<RwLock<NetworkManager>>,
        database: Database,
        data_dir: PathBuf,
        callbacks: Arc<RwLock<Vec<Box<dyn EventCallback>>>>,
        session_manager: SignalSessionManager,
        storage_key: [u8; 32],
        message_store_url: Option<String>,
        #[cfg(any(feature = "voip", feature = "video"))]
        call_manager: Arc<CallManager>,
        #[cfg(any(feature = "voip", feature = "video"))]
        voip_integration: Arc<VoIPIntegration>,
        group_manager: Arc<crate::group::GroupManager>,
        message_handler: Arc<crate::network::MessageHandler>,
    ) -> Self {
        Self {
            peer_id,
            identity,
            network,
            database,
            callbacks,
            data_dir,
            session_manager,
            storage_key,
            message_store_url,
            message_store_http: reqwest::Client::new(),
            #[cfg(any(feature = "voip", feature = "video"))]
            call_manager,
            #[cfg(any(feature = "voip", feature = "video"))]
            voip_integration,
            group_manager,
            message_handler,
        }
    }

    /// Get local peer ID
    pub fn local_peer_id(&self) -> PeerId {
        self.peer_id
    }

    /// Get local identity
    pub fn identity(&self) -> Arc<RwLock<Identity>> {
        Arc::clone(&self.identity)
    }

    /// Export current prekey bundle as JSON (for sharing)
    pub async fn get_prekey_bundle_json(&self) -> Result<String> {
        let mut identity = self.identity.write().await;
        identity.init_prekey_pool(100);
        let pool = identity
            .prekey_pool_mut()
            .ok_or_else(|| MePassaError::Identity("Prekey pool not initialized".to_string()))?;
        let bundle = pool.get_bundle()?;
        serde_json::to_string(&bundle)
            .map_err(|e| MePassaError::Identity(format!("Failed to serialize prekey bundle: {}", e)))
    }

    /// Store a contact's prekey bundle (JSON) for E2E encryption
    pub fn set_contact_prekey_bundle(&self, peer_id: String, bundle_json: String) -> Result<()> {
        let _bundle: crate::identity::PreKeyBundle = serde_json::from_str(&bundle_json)
            .map_err(|e| MePassaError::Identity(format!("Invalid prekey bundle JSON: {}", e)))?;

        let update = UpdateContact {
            prekey_bundle_json: Some(Some(bundle_json.clone())),
            ..Default::default()
        };

        match self.database.update_contact(&peer_id, &update) {
            Ok(_) => Ok(()),
            Err(StorageError::NotFound(_)) => {
                let contact = NewContact {
                    peer_id,
                    username: None,
                    display_name: None,
                    public_key: Vec::new(),
                    prekey_bundle_json: Some(bundle_json),
                };
                self.database.insert_contact(&contact)?;
                Ok(())
            }
            Err(e) => Err(MePassaError::Storage(format!("Failed to update contact: {}", e))),
        }
    }

    /// Get database
    pub fn database(&self) -> &Database {
        &self.database
    }

    /// Get data directory
    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    /// Register an event callback
    pub async fn register_callback<C>(&self, callback: C)
    where
        C: EventCallback + 'static,
    {
        let mut callbacks = self.callbacks.write().await;
        callbacks.push(Box::new(callback));
    }

    /// Emit an event to all callbacks
    async fn emit_event(&self, event: ClientEvent) {
        let callbacks = self.callbacks.read().await;
        for callback in callbacks.iter() {
            callback.on_event(event.clone());
        }
    }

    fn media_dir(&self) -> PathBuf {
        self.data_dir.join("media")
    }

    fn write_media_file(&self, media_hash: &str, file_name: Option<&str>, data: &[u8]) -> Result<String> {
        let media_dir = self.media_dir();
        std::fs::create_dir_all(&media_dir)
            .map_err(|e| MePassaError::Storage(format!("Failed to create media dir: {}", e)))?;

        let extension = file_name
            .and_then(|name| Path::new(name).extension())
            .and_then(|ext| ext.to_str());
        let file_name = match extension {
            Some(ext) => format!("{}.{}", media_hash, ext),
            None => media_hash.to_string(),
        };
        let path = media_dir.join(file_name);
        std::fs::write(&path, data)
            .map_err(|e| MePassaError::Storage(format!("Failed to write media file: {}", e)))?;
        Ok(path.to_string_lossy().to_string())
    }

    fn write_thumbnail_file(&self, media_hash: &str, data: &[u8]) -> Result<String> {
        let thumb_dir = self.media_dir().join("thumbnails");
        std::fs::create_dir_all(&thumb_dir)
            .map_err(|e| MePassaError::Storage(format!("Failed to create thumbnail dir: {}", e)))?;
        let path = thumb_dir.join(format!("{}.jpg", media_hash));
        std::fs::write(&path, data)
            .map_err(|e| MePassaError::Storage(format!("Failed to write thumbnail file: {}", e)))?;
        Ok(path.to_string_lossy().to_string())
    }

    fn compute_media_hash(data: &[u8], salt: Option<&str>) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        if let Some(salt) = salt {
            hasher.update(salt.as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }

    fn should_inline_media(size_bytes: usize) -> bool {
        size_bytes <= MAX_INLINE_MEDIA_BYTES
    }

    fn build_media_envelope(
        media_type: MediaType,
        media_hash: String,
        file_name: Option<String>,
        mime_type: Option<String>,
        width: Option<i32>,
        height: Option<i32>,
        duration_seconds: Option<i32>,
        bytes: &[u8],
        thumbnail_bytes: Option<&[u8]>,
    ) -> Result<String> {
        let bytes_b64 = general_purpose::STANDARD.encode(bytes);
        let thumbnail_b64 = thumbnail_bytes.map(|data| general_purpose::STANDARD.encode(data));
        let envelope = MediaEnvelope {
            version: 1,
            media_type: media_type.as_str().to_string(),
            media_hash,
            file_name,
            mime_type,
            width,
            height,
            duration_seconds,
            bytes_b64,
            thumbnail_b64,
        };
        envelope.encode()
    }

    async fn deliver_media_content(
        &self,
        to: PeerId,
        message_id: &str,
        content: String,
        label: &str,
    ) -> Result<DeliveryOutcome> {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let (message_type, payload) = self
            .prepare_outgoing_payload(&to, &content, String::new())
            .await?;

        let proto_message = Message {
            id: message_id.to_string(),
            sender_peer_id: self.local_peer_id().to_string(),
            recipient_peer_id: to.to_string(),
            timestamp,
            r#type: message_type as i32,
            payload: Some(payload),
        };

        self.deliver_message(to, &proto_message, label).await
    }

    /// Start listening on a multiaddr
    pub async fn listen_on(&self, addr: Multiaddr) -> Result<()> {
        let mut network = self.network.write().await;
        network.listen_on(addr)
    }

    /// Connect to a peer
    pub async fn connect_to_peer(&self, peer_id: PeerId, addr: Multiaddr) -> Result<()> {
        let mut network = self.network.write().await;
        network.add_peer_to_dht(peer_id, addr.clone());
        network.dial(peer_id, addr)?;

        self.emit_event(ClientEvent::PeerConnected { peer_id }).await;
        Ok(())
    }

    /// Send a text message to a peer
    pub async fn send_text_message(&self, to: PeerId, content: String) -> Result<String> {
        // Generate message ID
        let message_id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().timestamp_millis();

        let (message_type, payload) = self
            .prepare_outgoing_payload(&to, &content, String::new())
            .await?;

        // Create protocol message
        let proto_message = Message {
            id: message_id.clone(),
            sender_peer_id: self.local_peer_id().to_string(),
            recipient_peer_id: to.to_string(),
            timestamp,
            r#type: message_type as i32,
            payload: Some(payload),
        };

        let outcome = self.deliver_message(to, &proto_message, "text").await;

        // Store in database
        let conversation_id = self.database.get_or_create_conversation(&to.to_string())?;
        let status = match outcome {
            Ok(DeliveryOutcome { sent: true, .. }) => MessageStatus::Sent,
            Ok(DeliveryOutcome { stored: true, .. })
            | Ok(DeliveryOutcome { queued: true, .. }) => MessageStatus::Pending,
            _ => MessageStatus::Failed,
        };
        let new_msg = NewMessage {
            message_id: message_id.clone(),
            conversation_id: conversation_id.clone(),
            sender_peer_id: self.local_peer_id().to_string(),
            recipient_peer_id: Some(to.to_string()),
            message_type: "text".to_string(),
            content_encrypted: self.encrypt_for_storage(content.as_bytes()).ok(),
            content_plaintext: None,
            status,
            parent_message_id: None,
        };
        self.database.insert_message(&new_msg)?;
        self.database
            .update_conversation_last_message(&conversation_id, &message_id)?;

        // Emit event
        self.emit_event(ClientEvent::MessageSent {
            message_id: message_id.clone(),
            to,
        })
        .await;

        outcome?;

        Ok(message_id)
    }

    async fn encrypt_message_for_peer(
        &self,
        to: &PeerId,
        plaintext: &[u8],
    ) -> Result<Option<ProtoEncryptedMessage>> {
        Self::encrypt_for_peer_with(&self.database, &self.session_manager, to, plaintext).await
    }

    /// Versão associada do encrypt E2E (usada também pela task de orquestração
    /// de grupo no builder, que não tem `&self` do Client)
    pub(crate) async fn encrypt_for_peer_with(
        database: &Database,
        session_manager: &SignalSessionManager,
        to: &PeerId,
        plaintext: &[u8],
    ) -> Result<Option<ProtoEncryptedMessage>> {
        let contact = match database.get_contact_by_peer_id(&to.to_string()) {
            Ok(contact) => contact,
            Err(_) => return Ok(None),
        };

        let bundle_json = match contact.prekey_bundle_json {
            Some(value) => value,
            None => return Ok(None),
        };

        let bundle: crate::identity::PreKeyBundle = serde_json::from_str(&bundle_json)
            .map_err(|e| MePassaError::Crypto(format!("Invalid prekey bundle: {}", e)))?;

        let peer_id = to.to_string();
        let device_id = bundle.signal_device_id.unwrap_or(1);
        let has_session = session_manager.has_session(&peer_id, device_id).await?;
        let bundle_ref = if has_session { None } else { Some(&bundle) };
        let encrypted = session_manager
            .encrypt_for(&peer_id, device_id, bundle_ref, plaintext)
            .await?;

        Ok(Some(ProtoEncryptedMessage {
            ciphertext: encrypted.ciphertext,
            nonce: Vec::new(),
            ephemeral_public: Vec::new(),
            signed_prekey_id: 0,
            one_time_prekey_id: 0,
            ciphertext_type: encrypted.ciphertext_type,
            sender_device_id: encrypted.sender_device_id,
            recipient_device_id: device_id,
        }))
    }

    /// Prepara o payload de saída de uma mensagem (SEC-01):
    /// - E2E disponível → payload cifrado
    /// - Falha na criptografia (`Err`) → **erro, mensagem NÃO é enviada**
    ///   (antes havia downgrade silencioso para plaintext)
    /// - Sem sessão/bundle E2E (`Ok(None)`) → plaintext com warning, a menos
    ///   que `MEPASSA_REQUIRE_E2E=true` (aí o envio falha)
    async fn prepare_outgoing_payload(
        &self,
        to: &PeerId,
        content: &str,
        reply_to_id: String,
    ) -> Result<(MessageType, Payload)> {
        Self::prepare_payload_with(&self.database, &self.session_manager, to, content, reply_to_id)
            .await
    }

    pub(crate) async fn prepare_payload_with(
        database: &Database,
        session_manager: &SignalSessionManager,
        to: &PeerId,
        content: &str,
        reply_to_id: String,
    ) -> Result<(MessageType, Payload)> {
        match Self::encrypt_for_peer_with(database, session_manager, to, content.as_bytes()).await
        {
            Ok(Some(encrypted_payload)) => {
                Ok((MessageType::Encrypted, Payload::Encrypted(encrypted_payload)))
            }
            Ok(None) => {
                if e2e_required() {
                    return Err(MePassaError::Crypto(format!(
                        "No E2E session with {} and plaintext fallback is disabled \
                         (MEPASSA_REQUIRE_E2E)",
                        to
                    )));
                }
                tracing::warn!(
                    "⚠️ No E2E session with {} - sending PLAINTEXT \
                     (set MEPASSA_REQUIRE_E2E=true to forbid)",
                    to
                );
                Ok((
                    MessageType::Text,
                    Payload::Text(TextMessage {
                        content: content.to_string(),
                        reply_to_id,
                        metadata: std::collections::HashMap::new(),
                    }),
                ))
            }
            Err(e) => Err(MePassaError::Crypto(format!(
                "E2E encryption failed for {}: {} - message NOT sent",
                to, e
            ))),
        }
    }

    fn encrypt_for_storage(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        encrypt_for_storage(&self.storage_key, plaintext)
    }

    fn decrypt_for_storage(&self, blob: &[u8]) -> Result<String> {
        let bytes = decrypt_for_storage(&self.storage_key, blob)?;
        let text = String::from_utf8(bytes)
            .map_err(|_| MePassaError::Protocol("Invalid UTF-8 content".to_string()))?;
        Ok(text)
    }

    async fn ensure_peer_connected(&self, peer_id: PeerId) -> bool {
        Self::ensure_peer_connected_with(Arc::clone(&self.network), peer_id).await
    }

    pub(crate) async fn ensure_peer_connected_with(
        network: Arc<RwLock<NetworkManager>>,
        peer_id: PeerId,
    ) -> bool {
        let rx = {
            let mut network = network.write().await;
            if network.is_connected(&peer_id) {
                return true;
            } else {
                Some(network.resolve_peer_address(peer_id))
            }
        };

        let Some(rx) = rx else { return false };

        let resolved = match timeout(Duration::from_secs(5), rx).await {
            Ok(Ok(Some(addr))) => Some(addr),
            _ => None,
        };

        let Some(addr) = resolved else { return false };

        {
            let mut network = network.write().await;
            if network.is_connected(&peer_id) {
                return true;
            }
            network.add_peer_to_dht(peer_id, addr.clone());
            let _ = network.dial(peer_id, addr);
        }

        // Dial não é instantâneo: o ConnectionEstablished chega pelo event loop
        // do swarm, que roda em paralelo. Aguardar com deadline em vez de checar
        // imediatamente - senão a primeira mensagem para um peer alcançável cai
        // no caminho "offline".
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        loop {
            {
                let network = network.read().await;
                if network.is_connected(&peer_id) {
                    return true;
                }
            }
            if tokio::time::Instant::now() >= deadline {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    async fn deliver_message(
        &self,
        to: PeerId,
        proto_message: &Message,
        message_type: &str,
    ) -> Result<DeliveryOutcome> {
        let connected = self.ensure_peer_connected(to).await;
        if connected {
            let mut network = self.network.write().await;
            network.send_message(to, proto_message.clone())?;
            return Ok(DeliveryOutcome { sent: true, stored: false, queued: false });
        }

        if self.message_store_url.is_some()
            && self
                .store_offline_message(proto_message, message_type)
                .await
                .is_ok()
        {
            return Ok(DeliveryOutcome { sent: false, stored: true, queued: false });
        }

        // Peer offline e sem store (ou store falhou): persistir na fila local de
        // retry em vez de descartar a mensagem. O worker (builder) drena com backoff.
        let proto_bytes = {
            use prost::Message as _;
            proto_message.encode_to_vec()
        };
        let next_attempt_at = chrono::Utc::now().timestamp() + 5;
        match self.database.enqueue_outbound(
            &proto_message.id,
            &to.to_string(),
            message_type,
            &proto_bytes,
            next_attempt_at,
        ) {
            Ok(()) => Ok(DeliveryOutcome { sent: false, stored: false, queued: true }),
            Err(e) => Err(MePassaError::Network(format!(
                "Peer offline and failed to queue message for retry: {}",
                e
            ))),
        }
    }

    async fn deliver_message_with(
        network: Arc<RwLock<NetworkManager>>,
        identity: Arc<RwLock<Identity>>,
        to: PeerId,
        proto_message: Message,
        message_type: &str,
        message_store_url: Option<String>,
        message_store_http: reqwest::Client,
    ) -> Result<()> {
        let connected = Self::ensure_peer_connected_with(Arc::clone(&network), to).await;
        if connected {
            let mut network = network.write().await;
            network.send_message(to, proto_message)?;
            return Ok(());
        }

        let Some(base_url) = message_store_url
            .as_ref()
            .map(|url| url.trim_end_matches('/').to_string())
        else {
            return Err(MePassaError::Network(
                "Peer offline and message store not configured".to_string(),
            ));
        };

        let payload = crate::protocol::codec::encode(&proto_message)?;
        let request = StoreMessageRequest {
            recipient_peer_id: proto_message.recipient_peer_id.clone(),
            sender_peer_id: proto_message.sender_peer_id.clone(),
            encrypted_payload: general_purpose::STANDARD.encode(payload),
            message_type: Some(message_type.to_string()),
            message_id: proto_message.id.clone(),
        };

        let url = format!("{}/api/store", base_url);
        let (peer, ts, sig) = Self::store_auth_headers_with(
            &identity,
            &proto_message.sender_peer_id,
            "POST",
        )
        .await;
        let resp = message_store_http
            .post(url)
            .header("x-mepassa-peer", peer)
            .header("x-mepassa-ts", ts)
            .header("x-mepassa-sig", sig)
            .json(&request)
            .send()
            .await
            .map_err(|e| MePassaError::Network(format!("Message store error: {}", e)))?;

        if !resp.status().is_success() {
            return Err(MePassaError::Network(format!(
                "Message store returned {}",
                resp.status()
            )));
        }

        Ok(())
    }

    fn message_store_base_url(&self) -> Option<String> {
        self.message_store_url
            .as_ref()
            .map(|url| url.trim_end_matches('/').to_string())
    }

    /// Headers de autenticação do message store (SEC-09):
    /// assinatura Ed25519 sobre "{METHOD}:/api/store:{timestamp}"
    async fn store_auth_headers_with(
        identity: &Arc<RwLock<Identity>>,
        peer_id: &str,
        method: &str,
    ) -> (String, String, String) {
        let ts = chrono::Utc::now().timestamp();
        let message = format!("{}:/api/store:{}", method, ts);
        let signature = {
            let identity = identity.read().await;
            identity.keypair().sign(message.as_bytes())
        };
        (
            peer_id.to_string(),
            ts.to_string(),
            general_purpose::STANDARD.encode(signature),
        )
    }

    async fn store_offline_message(
        &self,
        proto_message: &Message,
        message_type: &str,
    ) -> Result<()> {
        let Some(base_url) = self.message_store_base_url() else { return Ok(()) };
        let payload = crate::protocol::codec::encode(proto_message)?;
        let request = StoreMessageRequest {
            recipient_peer_id: proto_message.recipient_peer_id.clone(),
            sender_peer_id: proto_message.sender_peer_id.clone(),
            encrypted_payload: general_purpose::STANDARD.encode(payload),
            message_type: Some(message_type.to_string()),
            message_id: proto_message.id.clone(),
        };

        let url = format!("{}/api/store", base_url);
        let local_peer = self.local_peer_id().to_string();
        let (peer, ts, sig) =
            Self::store_auth_headers_with(&self.identity, &local_peer, "POST").await;
        let resp = self
            .message_store_http
            .post(url)
            .header("x-mepassa-peer", peer)
            .header("x-mepassa-ts", ts)
            .header("x-mepassa-sig", sig)
            .json(&request)
            .send()
            .await
            .map_err(|e| MePassaError::Network(format!("Message store error: {}", e)))?;

        if !resp.status().is_success() {
            return Err(MePassaError::Network(format!(
                "Message store returned {}",
                resp.status()
            )));
        }

        Ok(())
    }

    async fn fetch_offline_messages(&self) -> Result<()> {
        let Some(base_url) = self.message_store_base_url() else { return Ok(()) };
        let url = format!(
            "{}/api/store?peer_id={}&limit=100",
            base_url,
            self.local_peer_id()
        );

        let local_peer = self.local_peer_id().to_string();
        let (peer, ts, sig) =
            Self::store_auth_headers_with(&self.identity, &local_peer, "GET").await;
        let resp = self
            .message_store_http
            .get(url)
            .header("x-mepassa-peer", peer)
            .header("x-mepassa-ts", ts)
            .header("x-mepassa-sig", sig)
            .send()
            .await
            .map_err(|e| MePassaError::Network(format!("Message store error: {}", e)))?;

        if !resp.status().is_success() {
            return Ok(());
        }

        let body: RetrieveMessagesResponse = resp
            .json()
            .await
            .map_err(|e| MePassaError::Network(format!("Invalid store response: {}", e)))?;

        if body.messages.is_empty() {
            return Ok(());
        }

        let mut processed_ids = Vec::new();
        for msg in body.messages {
            let payload = match general_purpose::STANDARD.decode(&msg.encrypted_payload) {
                Ok(bytes) => bytes,
                Err(e) => {
                    tracing::warn!("Invalid offline payload base64: {}", e);
                    continue;
                }
            };

            let decoded = match crate::protocol::codec::decode(&payload) {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!("Failed to decode offline message: {}", e);
                    continue;
                }
            };

            let sender = match PeerId::from_str(&msg.sender_peer_id) {
                Ok(id) => id,
                Err(_) => {
                    tracing::warn!("Invalid sender peer ID in offline message");
                    continue;
                }
            };

            if let Err(e) = self
                .message_handler
                .handle_incoming_message(sender, decoded)
                .await
            {
                tracing::warn!("Failed to process offline message: {}", e);
                continue;
            }

            processed_ids.push(msg.message_id);
        }

        if processed_ids.is_empty() {
            return Ok(());
        }

        let delete_url = format!("{}/api/store", base_url);
        let (peer, ts, sig) =
            Self::store_auth_headers_with(&self.identity, &local_peer, "DELETE").await;
        let _ = self
            .message_store_http
            .delete(delete_url)
            .header("x-mepassa-peer", peer)
            .header("x-mepassa-ts", ts)
            .header("x-mepassa-sig", sig)
            .json(&DeleteMessagesRequest {
                message_ids: processed_ids,
            })
            .send()
            .await;

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Media Methods (FASE 16 - Mídia & Polimento)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Send an image message with compression
    pub async fn send_image_message(
        &self,
        to: PeerId,
        image_data: &[u8],
        file_name: String,
        quality: u8,
    ) -> Result<String> {
        use crate::media::image::compress_image;

        // Compress image
        let compressed_data = compress_image(image_data, quality)
            .map_err(|e| MePassaError::Other(format!("Image compression failed: {}", e)))?;

        // Generate message ID
        let message_id = uuid::Uuid::new_v4().to_string();

        // Calculate media hash (salt with message_id to avoid collisions)
        let mut media_hash = Self::compute_media_hash(&compressed_data, None);
        if let Ok(Some(_existing)) = self.database.get_media_by_hash(&media_hash) {
            media_hash = Self::compute_media_hash(&compressed_data, Some(&message_id));
        }

        let local_path = self.write_media_file(&media_hash, Some(&file_name), &compressed_data)?;
        let media_type = MediaType::Image;
        let summary = crate::media::media_summary(media_type.as_str(), Some(&file_name), None);
        let inline = Self::should_inline_media(compressed_data.len());
        let outcome = if inline {
            let content = Self::build_media_envelope(
                media_type.clone(),
                media_hash.clone(),
                Some(file_name.clone()),
                Some("image/jpeg".to_string()),
                None,
                None,
                None,
                &compressed_data,
                None,
            )?;
            self.deliver_media_content(to, &message_id, content, "image")
                .await
        } else {
            let timestamp = chrono::Utc::now().timestamp_millis();
            let offer = MediaOffer {
                message_id: message_id.clone(),
                media_hash: media_hash.clone(),
                media_type: media_type.as_str().to_string(),
                file_name: file_name.clone(),
                mime_type: "image/jpeg".to_string(),
                file_size: compressed_data.len() as i64,
                width: 0,
                height: 0,
                duration_seconds: 0,
            };
            let message_type = MessageType::MediaOffer;
            let payload = Payload::MediaOffer(offer);

            let proto_message = Message {
                id: message_id.clone(),
                sender_peer_id: self.local_peer_id().to_string(),
                recipient_peer_id: to.to_string(),
                timestamp,
                r#type: message_type as i32,
                payload: Some(payload),
            };

            self.deliver_message(to, &proto_message, "image").await
        };

        // Store in database
        let conversation_id = self.database.get_or_create_conversation(&to.to_string())?;
        let status = match outcome {
            Ok(DeliveryOutcome { sent: true, .. }) => MessageStatus::Sent,
            Ok(DeliveryOutcome { stored: true, .. })
            | Ok(DeliveryOutcome { queued: true, .. }) => MessageStatus::Pending,
            _ => MessageStatus::Failed,
        };
        let new_msg = crate::storage::NewMessage {
            message_id: message_id.clone(),
            conversation_id: conversation_id.clone(),
            sender_peer_id: self.local_peer_id().to_string(),
            recipient_peer_id: Some(to.to_string()),
            message_type: "image".to_string(),
            content_encrypted: if inline {
                let content = Self::build_media_envelope(
                    media_type.clone(),
                    media_hash.clone(),
                    Some(file_name.clone()),
                    Some("image/jpeg".to_string()),
                    None,
                    None,
                    None,
                    &compressed_data,
                    None,
                )?;
                self.encrypt_for_storage(content.as_bytes()).ok()
            } else {
                None
            },
            content_plaintext: Some(summary.clone()),
            status,
            parent_message_id: None,
        };
        self.database.insert_message(&new_msg)?;
        self.database
            .update_conversation_last_message(&conversation_id, &message_id)?;

        // Store media record
        let new_media = crate::storage::NewMedia {
            media_hash: media_hash.clone(),
            message_id: message_id.clone(),
            media_type,
            file_name: Some(file_name),
            file_size: Some(compressed_data.len() as i64),
            mime_type: Some("image/jpeg".to_string()),
            local_path: Some(local_path),
            thumbnail_path: None,
            width: None,
            height: None,
            duration_seconds: None,
        };
        if let Err(e) = self.database.insert_media(&new_media) {
            tracing::warn!("Failed to insert media record: {}", e);
        }

        self.emit_event(ClientEvent::MessageSent {
            message_id: message_id.clone(),
            to,
        })
        .await;

        outcome?;

        Ok(message_id)
    }

    /// Send a voice message
    pub async fn send_voice_message(
        &self,
        to: PeerId,
        audio_data: &[u8],
        file_name: String,
        duration_seconds: i32,
    ) -> Result<String> {
        // Generate message ID
        let message_id = uuid::Uuid::new_v4().to_string();

        let mut media_hash = Self::compute_media_hash(audio_data, None);
        if let Ok(Some(_existing)) = self.database.get_media_by_hash(&media_hash) {
            media_hash = Self::compute_media_hash(audio_data, Some(&message_id));
        }

        let local_path = self.write_media_file(&media_hash, Some(&file_name), audio_data)?;
        let media_type = MediaType::VoiceMessage;
        let summary = crate::media::media_summary(
            media_type.as_str(),
            Some(&file_name),
            Some(duration_seconds),
        );
        let inline = Self::should_inline_media(audio_data.len());
        let outcome = if inline {
            let content = Self::build_media_envelope(
                media_type.clone(),
                media_hash.clone(),
                Some(file_name.clone()),
                Some("audio/aac".to_string()),
                None,
                None,
                Some(duration_seconds),
                audio_data,
                None,
            )?;
            self.deliver_media_content(to, &message_id, content, "voice")
                .await
        } else {
            let timestamp = chrono::Utc::now().timestamp_millis();
            let offer = MediaOffer {
                message_id: message_id.clone(),
                media_hash: media_hash.clone(),
                media_type: media_type.as_str().to_string(),
                file_name: file_name.clone(),
                mime_type: "audio/aac".to_string(),
                file_size: audio_data.len() as i64,
                width: 0,
                height: 0,
                duration_seconds,
            };
            let message_type = MessageType::MediaOffer;
            let payload = Payload::MediaOffer(offer);

            let proto_message = Message {
                id: message_id.clone(),
                sender_peer_id: self.local_peer_id().to_string(),
                recipient_peer_id: to.to_string(),
                timestamp,
                r#type: message_type as i32,
                payload: Some(payload),
            };

            self.deliver_message(to, &proto_message, "voice").await
        };

        // Store in database
        let conversation_id = self.database.get_or_create_conversation(&to.to_string())?;
        let status = match outcome {
            Ok(DeliveryOutcome { sent: true, .. }) => MessageStatus::Sent,
            Ok(DeliveryOutcome { stored: true, .. })
            | Ok(DeliveryOutcome { queued: true, .. }) => MessageStatus::Pending,
            _ => MessageStatus::Failed,
        };
        let new_msg = crate::storage::NewMessage {
            message_id: message_id.clone(),
            conversation_id: conversation_id.clone(),
            sender_peer_id: self.local_peer_id().to_string(),
            recipient_peer_id: Some(to.to_string()),
            message_type: "voice".to_string(),
            content_encrypted: if inline {
                let content = Self::build_media_envelope(
                    media_type.clone(),
                    media_hash.clone(),
                    Some(file_name.clone()),
                    Some("audio/aac".to_string()),
                    None,
                    None,
                    Some(duration_seconds),
                    audio_data,
                    None,
                )?;
                self.encrypt_for_storage(content.as_bytes()).ok()
            } else {
                None
            },
            content_plaintext: Some(summary.clone()),
            status,
            parent_message_id: None,
        };
        self.database.insert_message(&new_msg)?;
        self.database
            .update_conversation_last_message(&conversation_id, &message_id)?;

        // Store media record
        let new_media = crate::storage::NewMedia {
            media_hash: media_hash.clone(),
            message_id: message_id.clone(),
            media_type,
            file_name: Some(file_name),
            file_size: Some(audio_data.len() as i64),
            mime_type: Some("audio/aac".to_string()),
            local_path: Some(local_path),
            thumbnail_path: None,
            width: None,
            height: None,
            duration_seconds: Some(duration_seconds),
        };
        if let Err(e) = self.database.insert_media(&new_media) {
            tracing::warn!("Failed to insert media record: {}", e);
        }

        self.emit_event(ClientEvent::MessageSent {
            message_id: message_id.clone(),
            to,
        })
        .await;

        outcome?;

        Ok(message_id)
    }

    /// Send a document/file
    pub async fn send_document_message(
        &self,
        to: PeerId,
        file_data: &[u8],
        file_name: String,
        mime_type: String,
    ) -> Result<String> {
        // Generate message ID
        let message_id = uuid::Uuid::new_v4().to_string();

        let mut media_hash = Self::compute_media_hash(file_data, None);
        if let Ok(Some(_existing)) = self.database.get_media_by_hash(&media_hash) {
            media_hash = Self::compute_media_hash(file_data, Some(&message_id));
        }

        let local_path = self.write_media_file(&media_hash, Some(&file_name), file_data)?;
        let media_type = MediaType::Document;
        let summary = crate::media::media_summary(media_type.as_str(), Some(&file_name), None);
        let inline = Self::should_inline_media(file_data.len());
        let outcome = if inline {
            let content = Self::build_media_envelope(
                media_type.clone(),
                media_hash.clone(),
                Some(file_name.clone()),
                Some(mime_type.clone()),
                None,
                None,
                None,
                file_data,
                None,
            )?;
            self.deliver_media_content(to, &message_id, content, "document")
                .await
        } else {
            let timestamp = chrono::Utc::now().timestamp_millis();
            let offer = MediaOffer {
                message_id: message_id.clone(),
                media_hash: media_hash.clone(),
                media_type: media_type.as_str().to_string(),
                file_name: file_name.clone(),
                mime_type: mime_type.clone(),
                file_size: file_data.len() as i64,
                width: 0,
                height: 0,
                duration_seconds: 0,
            };
            let message_type = MessageType::MediaOffer;
            let payload = Payload::MediaOffer(offer);

            let proto_message = Message {
                id: message_id.clone(),
                sender_peer_id: self.local_peer_id().to_string(),
                recipient_peer_id: to.to_string(),
                timestamp,
                r#type: message_type as i32,
                payload: Some(payload),
            };

            self.deliver_message(to, &proto_message, "document").await
        };

        // Store in database
        let conversation_id = self.database.get_or_create_conversation(&to.to_string())?;
        let status = match outcome {
            Ok(DeliveryOutcome { sent: true, .. }) => MessageStatus::Sent,
            Ok(DeliveryOutcome { stored: true, .. })
            | Ok(DeliveryOutcome { queued: true, .. }) => MessageStatus::Pending,
            _ => MessageStatus::Failed,
        };
        let new_msg = crate::storage::NewMessage {
            message_id: message_id.clone(),
            conversation_id: conversation_id.clone(),
            sender_peer_id: self.local_peer_id().to_string(),
            recipient_peer_id: Some(to.to_string()),
            message_type: "document".to_string(),
            content_encrypted: if inline {
                let content = Self::build_media_envelope(
                    media_type.clone(),
                    media_hash.clone(),
                    Some(file_name.clone()),
                    Some(mime_type.clone()),
                    None,
                    None,
                    None,
                    file_data,
                    None,
                )?;
                self.encrypt_for_storage(content.as_bytes()).ok()
            } else {
                None
            },
            content_plaintext: Some(summary.clone()),
            status,
            parent_message_id: None,
        };
        self.database.insert_message(&new_msg)?;
        self.database
            .update_conversation_last_message(&conversation_id, &message_id)?;

        // Store media record
        let new_media = crate::storage::NewMedia {
            media_hash: media_hash.clone(),
            message_id: message_id.clone(),
            media_type,
            file_name: Some(file_name),
            file_size: Some(file_data.len() as i64),
            mime_type: Some(mime_type),
            local_path: Some(local_path),
            thumbnail_path: None,
            width: None,
            height: None,
            duration_seconds: None,
        };
        if let Err(e) = self.database.insert_media(&new_media) {
            tracing::warn!("Failed to insert media record: {}", e);
        }

        self.emit_event(ClientEvent::MessageSent {
            message_id: message_id.clone(),
            to,
        })
        .await;

        outcome?;

        Ok(message_id)
    }

    /// Send a video message
    pub async fn send_video_message(
        &self,
        to: PeerId,
        video_data: &[u8],
        file_name: String,
        width: Option<i32>,
        height: Option<i32>,
        duration_seconds: i32,
        thumbnail_data: Option<&[u8]>,
    ) -> Result<String> {
        self.ensure_peer_connected(to).await;

        // Generate message ID
        let message_id = uuid::Uuid::new_v4().to_string();

        let mut media_hash = Self::compute_media_hash(video_data, None);
        if let Ok(Some(_existing)) = self.database.get_media_by_hash(&media_hash) {
            media_hash = Self::compute_media_hash(video_data, Some(&message_id));
        }

        let local_path = self.write_media_file(&media_hash, Some(&file_name), video_data)?;
        let media_type = MediaType::Video;
        let summary = crate::media::media_summary(
            media_type.as_str(),
            Some(&file_name),
            Some(duration_seconds),
        );

        let mut thumbnail_path = None;
        if let Some(thumb_data) = thumbnail_data {
            thumbnail_path = Some(self.write_thumbnail_file(&media_hash, thumb_data)?);
        }

        let inline = Self::should_inline_media(video_data.len());
        let outcome = if inline {
            let content = Self::build_media_envelope(
                media_type.clone(),
                media_hash.clone(),
                Some(file_name.clone()),
                Some("video/mp4".to_string()),
                width,
                height,
                Some(duration_seconds),
                video_data,
                thumbnail_data,
            )?;
            self.deliver_media_content(to, &message_id, content, "video")
                .await
        } else {
            let timestamp = chrono::Utc::now().timestamp_millis();
            let offer = MediaOffer {
                message_id: message_id.clone(),
                media_hash: media_hash.clone(),
                media_type: media_type.as_str().to_string(),
                file_name: file_name.clone(),
                mime_type: "video/mp4".to_string(),
                file_size: video_data.len() as i64,
                width: width.unwrap_or(0),
                height: height.unwrap_or(0),
                duration_seconds,
            };
            let message_type = MessageType::MediaOffer;
            let payload = Payload::MediaOffer(offer);

            let proto_message = Message {
                id: message_id.clone(),
                sender_peer_id: self.local_peer_id().to_string(),
                recipient_peer_id: to.to_string(),
                timestamp,
                r#type: message_type as i32,
                payload: Some(payload),
            };

            self.deliver_message(to, &proto_message, "video").await
        };

        // Store in database
        let conversation_id = self.database.get_or_create_conversation(&to.to_string())?;
        let status = match outcome {
            Ok(DeliveryOutcome { sent: true, .. }) => MessageStatus::Sent,
            Ok(DeliveryOutcome { stored: true, .. })
            | Ok(DeliveryOutcome { queued: true, .. }) => MessageStatus::Pending,
            _ => MessageStatus::Failed,
        };
        let new_msg = crate::storage::NewMessage {
            message_id: message_id.clone(),
            conversation_id: conversation_id.clone(),
            sender_peer_id: self.local_peer_id().to_string(),
            recipient_peer_id: Some(to.to_string()),
            message_type: "video".to_string(),
            content_encrypted: if inline {
                let content = Self::build_media_envelope(
                    media_type.clone(),
                    media_hash.clone(),
                    Some(file_name.clone()),
                    Some("video/mp4".to_string()),
                    width,
                    height,
                    Some(duration_seconds),
                    video_data,
                    thumbnail_data,
                )?;
                self.encrypt_for_storage(content.as_bytes()).ok()
            } else {
                None
            },
            content_plaintext: Some(summary.clone()),
            status,
            parent_message_id: None,
        };
        self.database.insert_message(&new_msg)?;
        self.database
            .update_conversation_last_message(&conversation_id, &message_id)?;

        // Store media record
        let new_media = crate::storage::NewMedia {
            media_hash: media_hash.clone(),
            message_id: message_id.clone(),
            media_type,
            file_name: Some(file_name),
            file_size: Some(video_data.len() as i64),
            mime_type: Some("video/mp4".to_string()),
            local_path: Some(local_path),
            thumbnail_path,
            width,
            height,
            duration_seconds: Some(duration_seconds),
        };
        if let Err(e) = self.database.insert_media(&new_media) {
            tracing::warn!("Failed to insert media record: {}", e);
        }

        self.emit_event(ClientEvent::MessageSent {
            message_id: message_id.clone(),
            to,
        })
        .await;

        outcome?;

        Ok(message_id)
    }

    /// Download media by hash
    pub async fn download_media(&self, media_hash: &str) -> Result<Vec<u8>> {
        // Read from local storage if available
        if let Ok(Some(media)) = self.database.get_media_by_hash(media_hash) {
            if let Some(local_path) = media.local_path {
                if Path::new(&local_path).exists() {
                    let data = std::fs::read(&local_path)?;
                    return Ok(data);
                }
                tracing::warn!("Media path missing on disk: {}", local_path);
                let _ = self.database.delete_media(media.id);
            }

            // Request from peer if we know the message/peer
            let message = self
                .database
                .get_message(&media.message_id)
                .map_err(|e| MePassaError::Storage(e.to_string()))?;
            let peer_id = if message.sender_peer_id == self.local_peer_id().to_string() {
                message
                    .recipient_peer_id
                    .ok_or_else(|| MePassaError::Network("Missing recipient peer".to_string()))?
            } else {
                message.sender_peer_id
            };
            let peer_id: PeerId = peer_id
                .parse()
                .map_err(|_| MePassaError::Network("Invalid peer ID".to_string()))?;

            let mut last_error: Option<MePassaError> = None;

            for _ in 0..3 {
                self.ensure_peer_connected(peer_id).await;

                let request = MediaRequest {
                    message_id: media.message_id.clone(),
                    media_hash: media.media_hash.clone(),
                    offset: 0,
                    chunk_size: 64 * 1024,
                };

                let request_message = Message {
                    id: uuid::Uuid::new_v4().to_string(),
                    sender_peer_id: self.local_peer_id().to_string(),
                    recipient_peer_id: peer_id.to_string(),
                    timestamp: chrono::Utc::now().timestamp_millis(),
                    r#type: MessageType::MediaRequest as i32,
                    payload: Some(Payload::MediaRequest(request)),
                };

                {
                    let mut network = self.network.write().await;
                    network.send_message(peer_id, request_message)?;
                }

                // Poll for file to appear
                let wait_result = timeout(Duration::from_secs(10), async {
                    loop {
                        if let Ok(Some(updated)) = self.database.get_media_by_hash(media_hash) {
                            if let Some(path) = updated.local_path {
                                if Path::new(&path).exists() {
                                    return Ok::<String, MePassaError>(path);
                                }
                            }
                        }
                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }
                })
                .await;

                match wait_result {
                    Ok(Ok(path)) => {
                        let data = std::fs::read(&path)?;
                        return Ok(data);
                    }
                    Ok(Err(e)) => {
                        last_error = Some(e);
                    }
                    Err(_) => {
                        last_error = Some(MePassaError::Network(
                            "Timed out waiting for media".to_string(),
                        ));
                    }
                }

                tokio::time::sleep(Duration::from_millis(300)).await;
            }

            return Err(last_error.unwrap_or_else(|| {
                MePassaError::Network("Timed out waiting for media".to_string())
            }));
        }

        Err(MePassaError::NotFound(format!(
            "Media not found: {}",
            media_hash
        )))
    }

    /// Get media for a conversation
    pub fn get_conversation_media(
        &self,
        conversation_id: &str,
        media_type: Option<crate::storage::MediaType>,
        limit: Option<usize>,
    ) -> Result<Vec<crate::storage::Media>> {
        self.database
            .get_conversation_media(conversation_id, media_type, limit)
            .map_err(|e| MePassaError::Storage(e.to_string()))
    }

    /// Get messages for a conversation
    pub fn get_conversation_messages(
        &self,
        peer_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<crate::storage::Message>> {
        let conversation_id = format!("1:1:{}", peer_id);
        let mut messages = self.database
            .get_conversation_messages(&conversation_id, limit, offset)
            .map_err(|e| MePassaError::Storage(e.to_string()))?;

        for message in &mut messages {
            if message.content_plaintext.is_none() {
                if let Some(ref encrypted) = message.content_encrypted {
                    if let Ok(plaintext) = self.decrypt_for_storage(encrypted) {
                        message.content_plaintext = Some(plaintext);
                    }
                }
            }
        }

        Ok(messages)
    }

    // ═════════════════════════════════════════════════════════════════════
    // Message Actions (Delete & Forward)
    // ═════════════════════════════════════════════════════════════════════

    /// Delete message (soft delete - marks as deleted locally)
    pub fn delete_message(&self, message_id: &str) -> Result<()> {
        self.database
            .delete_message(message_id)
            .map_err(|e| MePassaError::Storage(e.to_string()))
    }

    /// Forward message to another peer/group
    pub async fn forward_message(
        &self,
        message_id: &str,
        to_peer_id: PeerId,
    ) -> Result<String> {
        // Get original message
        let original_msg = self
            .database
            .get_message(message_id)
            .map_err(|e| MePassaError::Storage(e.to_string()))?;

        // Create new message with forwarded content
        let new_message_id = uuid::Uuid::new_v4().to_string();
        let conversation_id = self.database.get_or_create_conversation(&to_peer_id.to_string())?;

        let forwarded_content = format!(
            "Forwarded: {}",
            original_msg
                .content_plaintext
                .or_else(|| {
                    original_msg
                        .content_encrypted
                        .as_ref()
                        .and_then(|blob| self.decrypt_for_storage(blob).ok())
                })
                .unwrap_or_default()
        );

        let (message_type, payload) = self
            .prepare_outgoing_payload(
                &to_peer_id,
                &forwarded_content,
                original_msg.message_id.clone(),
            )
            .await?;

        let proto_message = Message {
            id: new_message_id.clone(),
            sender_peer_id: self.local_peer_id().to_string(),
            recipient_peer_id: to_peer_id.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            r#type: message_type as i32,
            payload: Some(payload),
        };

        let outcome = self
            .deliver_message(to_peer_id, &proto_message, "forward")
            .await;
        let status = match outcome {
            Ok(DeliveryOutcome { sent: true, .. }) => MessageStatus::Sent,
            Ok(DeliveryOutcome { stored: true, .. })
            | Ok(DeliveryOutcome { queued: true, .. }) => MessageStatus::Pending,
            _ => MessageStatus::Failed,
        };

        let new_msg = crate::storage::NewMessage {
            message_id: new_message_id.clone(),
            conversation_id: conversation_id.clone(),
            sender_peer_id: self.local_peer_id().to_string(),
            recipient_peer_id: Some(to_peer_id.to_string()),
            message_type: "text".to_string(),
            content_encrypted: self.encrypt_for_storage(forwarded_content.as_bytes()).ok(),
            content_plaintext: None,
            status,
            parent_message_id: Some(original_msg.message_id.clone()),
        };

        self.database
            .insert_message(&new_msg)
            .map_err(|e| MePassaError::Storage(e.to_string()))?;

        self.database
            .update_conversation_last_message(&conversation_id, &new_message_id)
            .map_err(|e| MePassaError::Storage(e.to_string()))?;

        outcome?;

        Ok(new_message_id)
    }

    // ═════════════════════════════════════════════════════════════════════
    // Message Reactions (FASE 16 - TRACK 8)
    // ═════════════════════════════════════════════════════════════════════

    /// Add a reaction to a message
    pub async fn add_reaction(&self, message_id: &str, emoji: &str) -> Result<()> {
        let reaction_id = uuid::Uuid::new_v4().to_string();
        let peer_id = self.local_peer_id().to_string();

        let new_reaction = crate::storage::NewReaction {
            reaction_id,
            message_id: message_id.to_string(),
            peer_id,
            emoji: emoji.to_string(),
        };

        self.database
            .add_reaction(&new_reaction)
            .map_err(|e| MePassaError::Storage(e.to_string()))?;

        self.broadcast_reaction(message_id, emoji, "add").await?;

        Ok(())
    }

    /// Remove a reaction from a message
    pub async fn remove_reaction(&self, message_id: &str, emoji: &str) -> Result<()> {
        let peer_id = self.local_peer_id().to_string();

        self.database
            .remove_reaction(message_id, &peer_id, emoji)
            .map_err(|e| MePassaError::Storage(e.to_string()))?;

        self.broadcast_reaction(message_id, emoji, "remove").await?;

        Ok(())
    }

    fn reaction_target_peer_id(&self, message_id: &str) -> Result<PeerId> {
        let message = self
            .database
            .get_message(message_id)
            .map_err(|e| MePassaError::Storage(e.to_string()))?;

        let local_peer_id = self.local_peer_id().to_string();
        let remote_peer_id = if message.sender_peer_id == local_peer_id {
            message
                .recipient_peer_id
                .ok_or_else(|| MePassaError::Protocol("Missing recipient peer id".to_string()))?
        } else {
            message.sender_peer_id
        };

        PeerId::from_str(&remote_peer_id)
            .map_err(|_| MePassaError::Network("Invalid peer ID".to_string()))
    }

    async fn broadcast_reaction(
        &self,
        message_id: &str,
        emoji: &str,
        action: &str,
    ) -> Result<()> {
        let to_peer_id = self.reaction_target_peer_id(message_id)?;

        let envelope = ReactionEnvelope {
            version: 1,
            action: action.to_string(),
            message_id: message_id.to_string(),
            emoji: emoji.to_string(),
        };
        let content = envelope.encode()?;

        let (message_type, payload) = self
            .prepare_outgoing_payload(&to_peer_id, &content, String::new())
            .await?;

        let proto_message = Message {
            id: uuid::Uuid::new_v4().to_string(),
            sender_peer_id: self.local_peer_id().to_string(),
            recipient_peer_id: to_peer_id.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            r#type: message_type as i32,
            payload: Some(payload),
        };

        if let Err(e) = Client::deliver_message_with(
            Arc::clone(&self.network),
            Arc::clone(&self.identity),
            to_peer_id,
            proto_message,
            "reaction",
            self.message_store_url.clone(),
            self.message_store_http.clone(),
        )
        .await
        {
            tracing::warn!("Failed to broadcast reaction: {}", e);
        }

        Ok(())
    }

    /// Get all reactions for a message
    pub fn get_message_reactions(&self, message_id: &str) -> Result<Vec<crate::storage::Reaction>> {
        self.database
            .get_message_reactions(message_id)
            .map_err(|e| MePassaError::Storage(e.to_string()))
    }

    /// Get aggregated reaction counts for a message
    pub fn get_message_reaction_counts(&self, message_id: &str) -> Result<Vec<(String, u32)>> {
        self.database
            .get_message_reaction_counts(message_id)
            .map_err(|e| MePassaError::Storage(e.to_string()))
    }

    /// List all conversations
    pub fn list_conversations(&self) -> Result<Vec<crate::storage::Conversation>> {
        self.database
            .list_conversations()
            .map_err(|e| MePassaError::Storage(e.to_string()))
    }

    /// Search messages
    pub fn search_messages(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<crate::storage::Message>> {
        self.database
            .search_messages(query, limit)
            .map_err(|e| MePassaError::Storage(e.to_string()))
    }

    /// Mark conversation as read
    pub fn mark_conversation_read(&self, peer_id: &str) -> Result<()> {
        let conversation_id = format!("1:1:{}", peer_id);
        self.database
            .mark_conversation_read(&conversation_id)
            .map_err(|e| MePassaError::Storage(e.to_string()))
    }

    /// Get connected peers count
    pub async fn connected_peers_count(&self) -> usize {
        let network = self.network.read().await;
        network.connected_peers()
    }

    /// Get current listening addresses
    pub async fn listening_addresses(&self) -> Vec<String> {
        let network = self.network.read().await;
        network.listening_addresses()
            .into_iter()
            .map(|addr| addr.to_string())
            .collect()
    }

    /// Bootstrap DHT
    pub async fn bootstrap(&self) -> Result<()> {
        tracing::info!("🌐 Client bootstrap requested");
        let mut network = self.network.write().await;
        network.bootstrap()?;

        if self.message_store_url.is_some() {
            if let Err(e) = self.fetch_offline_messages().await {
                tracing::warn!("Failed to fetch offline messages: {}", e);
            }
        }

        Ok(())
    }

    // === VoIP Methods ===
    #[cfg(feature = "voip")]
    /// Start a voice call to a peer
    pub async fn start_call(&self, to_peer_id: String) -> Result<String> {
        self.voip_integration
            .start_call(to_peer_id)
            .await
            .map_err(|e| MePassaError::Other(format!("VoIP error: {}", e)))
    }

    #[cfg(feature = "voip")]
    /// Accept an incoming call
    pub async fn accept_call(&self, call_id: String) -> Result<()> {
        self.voip_integration
            .accept_call(call_id)
            .await
            .map_err(|e| MePassaError::Other(format!("VoIP error: {}", e)))
    }

    #[cfg(feature = "voip")]
    /// Reject an incoming call
    pub async fn reject_call(&self, call_id: String, reason: Option<String>) -> Result<()> {
        self.voip_integration
            .reject_call(call_id, reason)
            .await
            .map_err(|e| MePassaError::Other(format!("VoIP error: {}", e)))
    }

    #[cfg(feature = "voip")]
    /// Hang up an active call
    pub async fn hangup_call(&self, call_id: String) -> Result<()> {
        self.voip_integration
            .hangup_call(call_id)
            .await
            .map_err(|e| MePassaError::Other(format!("VoIP error: {}", e)))
    }

    #[cfg(feature = "voip")]
    /// Toggle audio mute for a call
    pub async fn toggle_mute(&self, call_id: String) -> Result<()> {
        self.call_manager
            .toggle_mute(call_id)
            .await
            .map_err(|e| MePassaError::Other(format!("VoIP error: {}", e)))
    }

    #[cfg(feature = "voip")]
    /// Toggle speakerphone for a call
    pub async fn toggle_speakerphone(&self, call_id: String) -> Result<()> {
        self.call_manager
            .toggle_speakerphone(call_id)
            .await
            .map_err(|e| MePassaError::Other(format!("VoIP error: {}", e)))
    }

    #[cfg(feature = "voip")]
    /// Send raw PCM audio frame to remote peer (Opus encoded in core)
    pub async fn send_audio_frame(
        &self,
        call_id: String,
        audio_data: &[u8],
        sample_rate: u32,
        channels: u32,
    ) -> Result<()> {
        self.call_manager
            .send_audio_frame(&call_id, audio_data, sample_rate, channels)
            .await
            .map_err(|e| MePassaError::Other(format!("Failed to send audio frame: {}", e)))
    }

    // ========== Video Methods (FASE 14) ==========

    #[cfg(any(feature = "voip", feature = "video"))]
    /// Enable video for an active call
    pub async fn enable_video(
        &self,
        call_id: String,
        codec: crate::voip::VideoCodec,
    ) -> Result<()> {
        self.call_manager
            .enable_video(&call_id, codec)
            .await
            .map_err(|e| MePassaError::Other(format!("Failed to enable video: {}", e)))
    }

    #[cfg(any(feature = "voip", feature = "video"))]
    /// Disable video for an active call
    pub async fn disable_video(&self, call_id: String) -> Result<()> {
        self.call_manager
            .disable_video(&call_id)
            .await
            .map_err(|e| MePassaError::Other(format!("Failed to disable video: {}", e)))
    }

    #[cfg(any(feature = "voip", feature = "video"))]
    /// Send video frame to remote peer
    ///
    /// Frame data should be pre-encoded (H.264 NALUs or VP8/VP9 frames)
    pub async fn send_video_frame(
        &self,
        call_id: String,
        frame_data: &[u8],
        _width: u32,
        _height: u32,
    ) -> Result<()> {
        self.call_manager
            .send_video_frame(&call_id, frame_data)
            .await
            .map_err(|e| MePassaError::Other(format!("Failed to send video frame: {}", e)))
    }

    #[cfg(any(feature = "voip", feature = "video"))]
    /// Switch camera (front/back) during video call
    ///
    /// Only applicable on mobile devices. Platform-specific camera manager
    /// handles the actual camera switching.
    pub async fn switch_camera(&self, call_id: String) -> Result<()> {
        self.call_manager
            .switch_camera(&call_id)
            .await
            .map_err(|e| MePassaError::Other(format!("Failed to switch camera: {}", e)))
    }

    #[cfg(any(feature = "voip", feature = "video"))]
    /// Register callback for receiving remote video frames (FASE 14)
    ///
    /// The callback will be invoked on a background thread whenever a remote
    /// video frame is received during an active video call.
    ///
    /// # Parameters
    /// - `callback`: Implementation of FfiVideoFrameCallback trait
    pub async fn register_video_frame_callback(
        &self,
        callback: Box<dyn crate::FfiVideoFrameCallback>,
    ) {
        self.voip_integration.register_video_frame_callback(callback).await;
    }

    #[cfg(feature = "voip")]
    /// Register callback for receiving remote audio frames (decoded PCM)
    ///
    /// The callback will be invoked on a background thread whenever remote
    /// audio is received during an active call.
    pub async fn register_audio_frame_callback(
        &self,
        callback: Box<dyn crate::FfiAudioFrameCallback>,
    ) {
        self.voip_integration.register_audio_frame_callback(callback).await;
    }

    #[cfg(any(feature = "voip", feature = "video"))]
    /// Register callback for VoIP control events (mute/speaker/camera)
    pub async fn register_voip_event_callback(
        &self,
        callback: Box<dyn crate::FfiVoipEventCallback>,
    ) {
        self.voip_integration
            .register_voip_event_callback(callback)
            .await;
    }

    /// Register callback for VoIP call lifecycle events (incoming/state/ended)
    #[cfg(any(feature = "voip", feature = "video"))]
    pub async fn register_call_event_callback(
        &self,
        callback: Box<dyn crate::FfiCallEventCallback>,
    ) {
        self.voip_integration
            .register_call_event_callback(callback)
            .await;
    }

    // ========== Group Methods (FASE 15) ==========

    /// Create a new group
    pub async fn create_group(
        &self,
        name: String,
        description: Option<String>,
    ) -> Result<crate::ffi::FfiGroup> {
        use crate::ffi::FfiGroup;

        let (group, topic) = self
            .group_manager
            .create_group(name, description)
            .await
            .map_err(|e| MePassaError::Other(format!("Failed to create group: {}", e)))?;

        {
            let mut network = self.network.write().await;
            if let Err(e) = network.subscribe_gossipsub(&topic) {
                tracing::warn!("Failed to subscribe to group topic: {}", e);
            }
        }

        Ok(FfiGroup::from_group(&group, &self.local_peer_id().to_string()))
    }

    /// Envia um envelope de controle de grupo para um peer (E2E quando há sessão)
    async fn send_group_control(
        &self,
        to: PeerId,
        envelope: &crate::group::GroupControlEnvelope,
    ) -> Result<()> {
        Self::send_group_control_with(
            &self.database,
            &self.session_manager,
            Arc::clone(&self.network),
            Arc::clone(&self.identity),
            self.message_store_url.clone(),
            self.message_store_http.clone(),
            &self.local_peer_id().to_string(),
            to,
            envelope,
        )
        .await
    }

    /// Versão associada do envio de controle de grupo (usada também pela task
    /// de orquestração no builder)
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn send_group_control_with(
        database: &Database,
        session_manager: &SignalSessionManager,
        network: Arc<RwLock<NetworkManager>>,
        identity: Arc<RwLock<Identity>>,
        message_store_url: Option<String>,
        message_store_http: reqwest::Client,
        local_peer_id: &str,
        to: PeerId,
        envelope: &crate::group::GroupControlEnvelope,
    ) -> Result<()> {
        let content = envelope.encode()?;

        // Mesma política SEC-01 das mensagens: falha de criptografia aborta o
        // envio; sem sessão E2E cai em plaintext apenas se permitido (com
        // warning extra quando o envelope carrega uma sender key seed).
        let (message_type, payload) =
            Self::prepare_payload_with(database, session_manager, &to, &content, String::new())
                .await?;

        if message_type == MessageType::Text && envelope.sender_key_seed.is_some() {
            tracing::warn!(
                "⚠️ Group control with sender key seed to {} sent WITHOUT E2E (no session)",
                to
            );
        }

        let proto_message = Message {
            id: uuid::Uuid::new_v4().to_string(),
            sender_peer_id: local_peer_id.to_string(),
            recipient_peer_id: to.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            r#type: message_type as i32,
            payload: Some(payload),
        };

        Client::deliver_message_with(
            network,
            identity,
            to,
            proto_message,
            "group_control",
            message_store_url,
            message_store_http,
        )
        .await
    }

    /// Join an existing group
    pub async fn join_group(&self, group_id: String, group_name: String) -> Result<()> {
        let topic = self.group_manager
            .join_group(group_id, group_name)
            .await
            .map_err(|e| MePassaError::Other(format!("Failed to join group: {}", e)))?;
        let mut network = self.network.write().await;
        if let Err(e) = network.subscribe_gossipsub(&topic) {
            tracing::warn!("Failed to subscribe to group topic: {}", e);
        }
        Ok(())
    }

    /// Leave a group
    pub async fn leave_group(&self, group_id: String) -> Result<()> {
        // Avisar os demais membros antes de sair (protocolo in-band)
        if let Some(group) = self.group_manager.get_group(&group_id).await {
            let envelope = crate::group::GroupControlEnvelope {
                version: 1,
                action: crate::group::envelope::actions::LEAVE.to_string(),
                group_id: group_id.clone(),
                group_name: None,
                group_description: None,
                creator_peer_id: None,
                members: None,
                member_peer_id: None,
                sender_key_seed: None,
            };
            let my_id = self.local_peer_id().to_string();
            for member in &group.members {
                if *member == my_id {
                    continue;
                }
                if let Ok(peer) = PeerId::from_str(member) {
                    if let Err(e) = self.send_group_control(peer, &envelope).await {
                        tracing::warn!("Failed to notify {} about leave: {}", member, e);
                    }
                }
            }

            let topic = libp2p::gossipsub::IdentTopic::new(&group.topic);
            let mut network = self.network.write().await;
            if let Err(e) = network.unsubscribe_gossipsub(&topic) {
                tracing::warn!("Failed to unsubscribe from group topic: {}", e);
            }
        }
        self.group_manager
            .leave_group(&group_id)
            .await
            .map_err(|e| MePassaError::Other(format!("Failed to leave group: {}", e)))
    }

    /// Add a member to a group (admin only)
    ///
    /// Protocolo in-band (CORE-16): além do estado local, envia
    /// - `invite` para o novo membro (metadados + membership + minha seed)
    /// - `member_added` para os demais membros
    pub async fn add_group_member(&self, group_id: String, peer_id: String) -> Result<()> {
        self.group_manager
            .add_member(&group_id, &peer_id)
            .await
            .map_err(|e| MePassaError::Other(format!("Failed to add member: {}", e)))?;

        let group = self
            .group_manager
            .get_group(&group_id)
            .await
            .ok_or_else(|| MePassaError::NotFound("Group not found after add".to_string()))?;

        let my_id = self.local_peer_id().to_string();
        let my_seed = self
            .group_manager
            .get_group_sender_key_seed(&group_id)
            .ok()
            .map(|seed| general_purpose::STANDARD.encode(&seed));

        let members: Vec<String> = group.members.iter().cloned().collect();

        // 1) Invite para o novo membro
        if let Ok(new_peer) = PeerId::from_str(&peer_id) {
            let invite = crate::group::GroupControlEnvelope {
                version: 1,
                action: crate::group::envelope::actions::INVITE.to_string(),
                group_id: group_id.clone(),
                group_name: Some(group.name.clone()),
                group_description: group.description.clone(),
                creator_peer_id: Some(group.creator_peer_id.clone()),
                members: Some(members.clone()),
                member_peer_id: None,
                sender_key_seed: my_seed.clone(),
            };
            if let Err(e) = self.send_group_control(new_peer, &invite).await {
                tracing::warn!("Failed to send group invite to {}: {}", peer_id, e);
            }
        }

        // 2) member_added para os demais membros
        let member_added = crate::group::GroupControlEnvelope {
            version: 1,
            action: crate::group::envelope::actions::MEMBER_ADDED.to_string(),
            group_id: group_id.clone(),
            group_name: None,
            group_description: None,
            creator_peer_id: None,
            members: None,
            member_peer_id: Some(peer_id.clone()),
            sender_key_seed: None,
        };
        for member in &members {
            if *member == my_id || *member == peer_id {
                continue;
            }
            if let Ok(peer) = PeerId::from_str(member) {
                if let Err(e) = self.send_group_control(peer, &member_added).await {
                    tracing::warn!("Failed to notify {} about member_added: {}", member, e);
                }
            }
        }

        Ok(())
    }

    /// Remove a member from a group (admin only)
    pub async fn remove_group_member(&self, group_id: String, peer_id: String) -> Result<()> {
        // Snapshot dos membros ANTES da remoção (para notificar o removido também)
        let members: Vec<String> = self
            .group_manager
            .get_group(&group_id)
            .await
            .map(|g| g.members.iter().cloned().collect())
            .unwrap_or_default();

        self.group_manager
            .remove_member(&group_id, &peer_id)
            .await
            .map_err(|e| MePassaError::Other(format!("Failed to remove member: {}", e)))?;

        let my_id = self.local_peer_id().to_string();
        let envelope = crate::group::GroupControlEnvelope {
            version: 1,
            action: crate::group::envelope::actions::MEMBER_REMOVED.to_string(),
            group_id: group_id.clone(),
            group_name: None,
            group_description: None,
            creator_peer_id: None,
            members: None,
            member_peer_id: Some(peer_id.clone()),
            sender_key_seed: None,
        };
        for member in &members {
            if *member == my_id {
                continue;
            }
            if let Ok(peer) = PeerId::from_str(member) {
                if let Err(e) = self.send_group_control(peer, &envelope).await {
                    tracing::warn!("Failed to notify {} about member_removed: {}", member, e);
                }
            }
        }

        Ok(())
    }

    /// Get the member peer IDs of a group
    pub async fn get_group_members(&self, group_id: String) -> Result<Vec<String>> {
        let group = self
            .group_manager
            .get_group(&group_id)
            .await
            .ok_or_else(|| MePassaError::NotFound("Group not found".to_string()))?;

        let mut members: Vec<String> = group.members.iter().cloned().collect();
        members.sort();
        Ok(members)
    }

    /// Update group metadata (admin only)
    pub async fn update_group(
        &self,
        group_id: String,
        name: Option<String>,
        description: Option<String>,
    ) -> Result<()> {
        self.group_manager
            .update_group(&group_id, name, description, None)
            .await
            .map_err(|e| MePassaError::Other(format!("Failed to update group: {}", e)))
    }

    /// Get all groups
    pub async fn get_groups(&self) -> Result<Vec<crate::ffi::FfiGroup>> {
        use crate::ffi::FfiGroup;

        let groups = self.group_manager.get_all_groups().await;
        let local_peer_id = self.local_peer_id().to_string();

        Ok(groups
            .iter()
            .map(|g| FfiGroup::from_group(g, &local_peer_id))
            .collect())
    }

    /// Get messages for a group
    pub async fn get_group_messages(
        &self,
        group_id: String,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<crate::ffi::FfiMessage>> {
        let conversation_id = format!("group:{}", group_id);
        let messages = self
            .database
            .get_conversation_messages(&conversation_id, limit, offset)
            .map_err(|e| MePassaError::Storage(e.to_string()))?;
        Ok(messages.into_iter().map(|m| m.into()).collect())
    }

    /// Send a text message to a group
    pub async fn send_group_message(&self, group_id: String, content: String) -> Result<String> {
        let group = self
            .group_manager
            .get_group(&group_id)
            .await
            .ok_or_else(|| MePassaError::NotFound("Group not found".to_string()))?;

        let message_id = uuid::Uuid::new_v4().to_string();
        let conversation_id = format!("group:{}", group_id);

        let encrypted_payload = self
            .group_manager
            .encrypt_group_message(&group_id, content.as_bytes())?;

        let mut group_message = crate::group::GroupMessage {
            message_id: message_id.clone(),
            group_id: group_id.clone(),
            sender_peer_id: self.local_peer_id().to_string(),
            message_type: crate::group::types::GroupMessageType::Text,
            content: encrypted_payload.clone(),
            timestamp: chrono::Utc::now().timestamp(),
            signature: Vec::new(),
        };
        {
            let identity = self.identity.read().await;
            group_message.sign(identity.keypair())?;
        }

        let payload = serde_json::to_vec(&group_message)
            .map_err(|e| MePassaError::Protocol(format!("Invalid group message: {}", e)))?;

        {
            let mut network = self.network.write().await;
            let topic = libp2p::gossipsub::IdentTopic::new(&group.topic);
            network.publish_gossipsub(&topic, payload)?;
        }

        let new_msg = crate::storage::NewMessage {
            message_id: message_id.clone(),
            conversation_id: conversation_id.clone(),
            sender_peer_id: self.local_peer_id().to_string(),
            recipient_peer_id: None,
            message_type: "group_text".to_string(),
            content_encrypted: Some(encrypted_payload),
            content_plaintext: Some(content),
            status: MessageStatus::Sent,
            parent_message_id: None,
        };
        self.database
            .insert_message(&new_msg)
            .map_err(|e| MePassaError::Storage(e.to_string()))?;
        self.database
            .update_conversation_last_message(&conversation_id, &message_id)
            .map_err(|e| MePassaError::Storage(e.to_string()))?;

        Ok(message_id)
    }

    /// Get my sender-key seed for a group
    pub async fn get_group_sender_key_seed(&self, group_id: String) -> Result<Vec<u8>> {
        self.group_manager
            .get_group_sender_key_seed(&group_id)
            .map_err(|e| MePassaError::Crypto(format!("Failed to read sender key: {}", e)))
    }

    /// Store a sender-key seed for a group member
    pub async fn add_group_sender_key(
        &self,
        group_id: String,
        sender_peer_id: String,
        sender_key_seed: Vec<u8>,
    ) -> Result<()> {
        self.group_manager
            .add_group_sender_key(&group_id, &sender_peer_id, &sender_key_seed)
            .map_err(|e| MePassaError::Crypto(format!("Failed to store sender key: {}", e)))
    }

    /// Run network event loop (blocking)
    ///
    /// This should be spawned as a separate task to process incoming P2P messages.
    /// Implementado como loop de poll para NÃO segurar o write-lock do
    /// NetworkManager para sempre (a versão anterior deadlockava qualquer
    /// outra operação de rede se fosse chamada).
    pub async fn run_network(&self) -> Result<()> {
        loop {
            let processed = self.poll_network_once().await?;
            if !processed {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }

    /// Poll network for one event (non-blocking)
    ///
    /// Returns true if an event was processed, false if no events pending.
    /// This method acquires and releases the lock quickly, allowing other
    /// operations to proceed.
    pub async fn poll_network_once(&self) -> Result<bool> {
        // CORE-04: o poll segura o write-lock só pelo tempo do evento de
        // swarm; o trabalho pesado (decrypt Signal, SQLite, fs) roda depois,
        // com o lock solto, para não bloquear sends/polls concorrentes
        let (progressed, inbound, gossip, handler) = {
            let mut network = self.network.write().await;
            let progressed = network.poll_once().await?;
            let (inbound, gossip) = network.take_pending_inbound();
            let handler = network.message_handler_arc();
            (progressed, inbound, gossip, handler)
        };

        for item in inbound {
            self.process_inbound_request(item, handler.as_deref()).await;
        }

        for message in gossip {
            let topic = message.topic.clone();
            if let Err(e) = self
                .group_manager
                .handle_gossipsub_message(&topic, message)
                .await
            {
                tracing::warn!("Failed to handle group message: {}", e);
            }
        }

        Ok(progressed)
    }

    /// CORE-04: processa um request inbound fora do lock e reencaixa o ACK
    /// (e chunks de mídia) com uma reaquisição curta
    async fn process_inbound_request(
        &self,
        item: crate::network::swarm::InboundRequest,
        handler: Option<&crate::network::MessageHandler>,
    ) {
        use crate::protocol::pb::message::Payload;
        use crate::protocol::MessageType;

        let Some(handler) = handler else {
            tracing::warn!("⚠️ No message handler configured, message will be dropped");
            return;
        };

        let crate::network::swarm::InboundRequest { peer, request, channel } = item;

        // MediaRequest dispara respostas em chunks
        let message_type =
            MessageType::try_from(request.r#type).unwrap_or(MessageType::Unspecified);
        let mut pending_chunks = Vec::new();
        if message_type == MessageType::MediaRequest {
            if let Some(Payload::MediaRequest(ref media_request)) = request.payload {
                match handler.build_media_chunks(peer, media_request).await {
                    Ok(chunks) => pending_chunks = chunks,
                    Err(e) => tracing::error!("❌ Failed to build media chunks: {}", e),
                }
            }
        }

        match handler.handle_incoming_message(peer, request).await {
            Ok(ack) => {
                tracing::info!("✅ Processed message {}, sending ACK", ack.message_id);
                let response = crate::protocol::Message {
                    id: uuid::Uuid::new_v4().to_string(),
                    sender_peer_id: self.local_peer_id().to_string(),
                    recipient_peer_id: peer.to_string(),
                    timestamp: chrono::Utc::now().timestamp_millis(),
                    r#type: MessageType::Ack as i32,
                    payload: Some(Payload::Ack(ack)),
                };

                let mut network = self.network.write().await;
                if let Err(e) = network.send_ack(channel, response) {
                    tracing::error!("❌ Failed to send ACK: {}", e);
                }
                for chunk in pending_chunks {
                    let _ = network.send_message(peer, chunk);
                }
            }
            Err(e) => {
                tracing::error!("❌ Failed to process message: {}", e);
            }
        }
    }

    /// Get a clone of the network Arc for spawning the event loop
    pub fn network_arc(&self) -> Arc<RwLock<NetworkManager>> {
        Arc::clone(&self.network)
    }
}

/// Política SEC-01: quando true, mensagens sem sessão E2E estabelecida NÃO
/// caem em plaintext - o envio falha. Default false para o alfa (a troca de
/// prekeys ainda não é automática nos apps).
fn e2e_required() -> bool {
    std::env::var("MEPASSA_REQUIRE_E2E")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

#[derive(Debug, Clone)]
struct DeliveryOutcome {
    sent: bool,
    stored: bool,
    /// Message persisted to the local outbound retry queue (peer offline)
    queued: bool,
}

#[derive(Debug, Serialize)]
struct StoreMessageRequest {
    recipient_peer_id: String,
    sender_peer_id: String,
    encrypted_payload: String,
    message_type: Option<String>,
    message_id: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RetrieveMessagesResponse {
    messages: Vec<OfflineMessageDto>,
    total: i64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct OfflineMessageDto {
    sender_peer_id: String,
    encrypted_payload: String,
    message_type: String,
    message_id: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct DeleteMessagesRequest {
    message_ids: Vec<String>,
}

#[cfg(test)]
mod tests {
    use crate::api::ClientBuilder;
    use tempfile::TempDir;

    // build() spawna workers com spawn_local, então precisa rodar em LocalSet
    // (igual ao caminho FFI de produção)

    #[tokio::test]
    async fn test_create_client() {
        let local = tokio::task::LocalSet::new();
        local
            .run_until(async {
                let temp_dir = TempDir::new().unwrap();
                let data_dir = temp_dir.path().to_path_buf();

                let client = ClientBuilder::new()
                    .data_dir(data_dir)
                    .build()
                    .await
                    .unwrap();

                assert!(client.local_peer_id().to_string().len() > 0);
            })
            .await;
    }

    #[tokio::test]
    async fn test_list_conversations() {
        let local = tokio::task::LocalSet::new();
        local
            .run_until(async {
                let temp_dir = TempDir::new().unwrap();
                let data_dir = temp_dir.path().to_path_buf();

                let client = ClientBuilder::new()
                    .data_dir(data_dir)
                    .build()
                    .await
                    .unwrap();

                let conversations = client.list_conversations().unwrap();
                assert_eq!(conversations.len(), 0);
            })
            .await;
    }
}
