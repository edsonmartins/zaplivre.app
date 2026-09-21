//! Message Handler
//!
//! Handles incoming messages from the network:
//! 1. Validates message format
//! 2. Decrypts content (if encrypted)
//! 3. Stores message in database
//! 4. Emits events to application layer
//! 5. Sends acknowledgment back to sender

use libp2p::PeerId;
use std::{collections::HashMap, path::PathBuf, sync::Arc};

use crate::identity::Identity;
use crate::{
    crypto::{
        decrypt_for_storage, encrypt_for_storage, SignalEncryptedMessage, SignalSessionManager,
    },
    media::{MediaEnvelope, MediaOfferEnvelope},
    protocol::{
        pb::message::Payload, AckMessage, AckStatus, EncryptedMessage as ProtoEncryptedMessage,
        MediaChunk, MediaRequest, Message, MessageType, PreKeyBundleSync, ReadReceipt, TextMessage,
        TypingIndicator,
    },
    reactions::ReactionEnvelope,
    storage::{
        Database, MediaType, MessageStatus, NewMedia, NewMessage, NewReaction, UpdateMessage,
    },
    utils::error::{Result, ZapLivreError},
};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

/// Hard ceiling for a single received media file.
const MAX_MEDIA_BYTES: u64 = 512 * 1024 * 1024;

/// Message handler
///
/// Processes incoming messages and coordinates between network, storage, and crypto layers.
pub struct MessageHandler {
    /// Local peer ID
    local_peer_id: String,

    /// Database for storing messages (thread-safe via internal Mutex)
    database: Arc<Database>,

    /// Base data directory for storing media files
    data_dir: PathBuf,

    /// Identity (prekeys for X3DH)
    #[allow(dead_code)]
    identity: Arc<RwLock<Identity>>,

    /// E2E session manager
    session_manager: SignalSessionManager,

    /// Storage encryption key
    storage_key: [u8; 32],

    /// Event callback for notifying UI
    event_tx: Option<tokio::sync::mpsc::Sender<MessageEvent>>,

    /// Whether plaintext application payloads are accepted (dev only, SEC-01)
    allow_plaintext: bool,

    /// Media downloads in flight: sealed-blob hash -> (offset -> length) of the
    /// chunks already written.
    downloads: std::sync::Mutex<HashMap<String, HashMap<u64, u64>>>,
}

impl MessageHandler {
    /// Create a new message handler
    pub fn new(
        local_peer_id: String,
        database: Arc<Database>,
        data_dir: PathBuf,
        identity: Arc<RwLock<Identity>>,
        session_manager: SignalSessionManager,
        storage_key: [u8; 32],
        event_tx: Option<tokio::sync::mpsc::Sender<MessageEvent>>,
    ) -> Self {
        Self {
            local_peer_id,
            database,
            data_dir,
            identity,
            session_manager,
            storage_key,
            event_tx,
            allow_plaintext: false,
            downloads: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// A group message delivered to us individually by its sender.
    ///
    /// It goes through the same ordered event queue as the group control
    /// envelopes: handled right here it could overtake the invite or the
    /// sender key that precede it and be dropped as "unknown group".
    async fn handle_group_fanout(
        &self,
        message: &Message,
        group_msg: crate::group::GroupMessage,
    ) -> Result<()> {
        self.emit_event(MessageEvent::GroupMessage {
            from_peer_id: message.sender_peer_id.clone(),
            message: group_msg,
        });
        Ok(())
    }

    /// Accept plaintext `Text` payloads from peers without an E2E session.
    ///
    /// Secure by default: only local development (`ZAPLIVRE_ALLOW_PLAINTEXT`)
    /// should enable this, mirroring the send-side downgrade policy.
    pub fn allow_plaintext(mut self, allow: bool) -> Self {
        self.allow_plaintext = allow;
        self
    }

    /// Handle an incoming message request
    ///
    /// Returns an acknowledgment message to send back to the sender.
    pub async fn handle_incoming_message(
        &self,
        from_peer: PeerId,
        message: Message,
    ) -> Result<AckMessage> {
        tracing::info!(
            "📨 Processing message {} from {} (type: {:?})",
            message.id,
            from_peer,
            MessageType::try_from(message.r#type).unwrap_or(MessageType::Unspecified)
        );

        // Validate message
        if let Err(e) = self.validate_message(&from_peer, &message) {
            tracing::warn!("Invalid message {}: {}", message.id, e);
            return Ok(self.create_ack(&message.id, AckStatus::Error, Some(e.to_string())));
        }

        // Delivery is at-least-once: the same message can arrive through the
        // message store and again through the P2P outbox retry. Acknowledge what
        // was already processed instead of decrypting it twice (the ratchet
        // refuses the replay, which the sender would then record as Failed and
        // the mailbox would keep forever). Only content-bearing payloads are
        // tracked; ACKs, receipts, typing, prekey sync and media chunks are
        // idempotent or ephemeral.
        let tracked = matches!(
            message.payload,
            Some(Payload::Text(_)) | Some(Payload::Encrypted(_))
        );
        if tracked
            && self
                .database
                .is_message_processed(&message.sender_peer_id, &message.id)
                .unwrap_or(false)
        {
            tracing::debug!("Duplicate message {} acknowledged", message.id);
            return Ok(self.create_ack(&message.id, AckStatus::Received, None));
        }

        // Process based on message type
        let result = match message.payload {
            Some(Payload::Text(ref text_msg)) => self.handle_text_message(&message, text_msg).await,
            Some(Payload::Ack(ref ack_msg)) => self.handle_ack_message(&message, ack_msg).await,
            Some(Payload::Typing(ref typing_msg)) => {
                self.handle_typing_indicator(&message, typing_msg).await
            }
            Some(Payload::ReadReceipt(ref read_msg)) => {
                self.handle_read_receipt(&message, read_msg).await
            }
            Some(Payload::Encrypted(ref enc_msg)) => {
                self.handle_encrypted_message(&message, enc_msg).await
            }
            // Offers travel inside the Signal session as a MediaOfferEnvelope.
            // The bare payload exposed file name, size and hash to the
            // transport and the message store, and carried no file key.
            Some(Payload::MediaOffer(_)) => Err(ZapLivreError::Protocol(
                "Plaintext media offers are no longer accepted".to_string(),
            )),
            Some(Payload::MediaChunk(ref chunk)) => self.handle_media_chunk(&message, chunk).await,
            Some(Payload::PrekeyBundleSync(ref sync)) => {
                self.handle_prekey_bundle_sync(&message, from_peer, sync)
                    .await
            }
            Some(Payload::MediaRequest(_)) => {
                // Media requests are handled in NetworkManager to enable chunk sending.
                Ok(())
            }
            None => {
                tracing::warn!("Message {} has no payload", message.id);
                Err(ZapLivreError::Protocol(
                    "Message has no payload".to_string(),
                ))
            }
        };

        match result {
            Ok(_) => {
                if tracked {
                    if let Err(e) = self
                        .database
                        .mark_message_processed(&message.sender_peer_id, &message.id)
                    {
                        tracing::warn!("Failed to record processed message: {}", e);
                    }
                }
                Ok(self.create_ack(&message.id, AckStatus::Received, None))
            }
            Err(e) => {
                tracing::error!("Failed to process message {}: {}", message.id, e);
                Ok(self.create_ack(&message.id, AckStatus::Error, Some(e.to_string())))
            }
        }
    }

    /// Build the private control message sent after a P2P connection is
    /// authenticated. Only the public bundle is serialized; private Signal
    /// material never leaves the local identity store.
    pub async fn create_prekey_bundle_sync(&self, peer_id: &PeerId) -> Result<Message> {
        let mut identity = self.identity.write().await;
        identity.init_prekey_pool(100);
        let pool = identity
            .prekey_pool_mut()
            .ok_or_else(|| ZapLivreError::Identity("Prekey pool not initialized".into()))?;
        let bundle_json = serde_json::to_string(&pool.get_bundle()?)
            .map_err(|e| ZapLivreError::Identity(format!("Failed to serialize prekeys: {}", e)))?;
        Ok(Message {
            id: uuid::Uuid::new_v4().to_string(),
            sender_peer_id: self.local_peer_id.clone(),
            recipient_peer_id: peer_id.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            r#type: MessageType::PrekeyBundleSync as i32,
            payload: Some(Payload::PrekeyBundleSync(PreKeyBundleSync { bundle_json })),
        })
    }

    /// Wake messages queued while this peer was offline. This is deliberately
    /// tied to the authenticated connection event so the user never has to
    /// send a second message to bootstrap delivery.
    pub fn wake_pending_outbound(&self, peer_id: &PeerId) {
        match self.database.wake_outbound_for_peer(&peer_id.to_string()) {
            Ok(count) if count > 0 => {
                tracing::info!("📤 Woke {} queued messages for {}", count, peer_id);
            }
            Ok(_) => {}
            Err(e) => tracing::warn!("Failed to wake queued messages for {}: {}", peer_id, e),
        }
    }

    async fn handle_prekey_bundle_sync(
        &self,
        message: &Message,
        from_peer: PeerId,
        sync: &PreKeyBundleSync,
    ) -> Result<()> {
        if message.sender_peer_id != from_peer.to_string() {
            return Err(ZapLivreError::Protocol("Prekey sender mismatch".into()));
        }
        // Validate and normalize before persisting. New peers send the native
        // core format; older clients may still send the identity-server DTO.
        // Persisting one canonical format keeps subsequent session setup
        // independent of which client initiated the connection.
        let bundle_json =
            match serde_json::from_str::<crate::identity::PreKeyBundle>(&sync.bundle_json) {
                Ok(_) => sync.bundle_json.clone(),
                Err(_) => {
                    let dto: crate::identity_client::PreKeyBundle =
                        serde_json::from_str(&sync.bundle_json).map_err(|e| {
                            ZapLivreError::Identity(format!("Invalid prekey bundle: {}", e))
                        })?;
                    let core_bundle = dto.to_core().map_err(|e| {
                        ZapLivreError::Identity(format!("Invalid prekey bundle: {}", e))
                    })?;
                    serde_json::to_string(&core_bundle).map_err(|e| {
                        ZapLivreError::Identity(format!("Failed to normalize prekey bundle: {}", e))
                    })?
                }
            };
        let peer_id = from_peer.to_string();
        // Ed25519 libp2p PeerIds embed the authenticated public key in the
        // identity multihash. Persist it so safety-number verification also
        // works for contacts discovered automatically.
        let authenticated_public_key = public_key_from_peer_id(&from_peer);
        let update = crate::storage::UpdateContact {
            prekey_bundle_json: Some(Some(bundle_json.clone())),
            public_key: authenticated_public_key
                .clone()
                .filter(|key| !key.is_empty()),
            last_seen_at: Some(chrono::Utc::now()),
            ..Default::default()
        };
        match self.database.update_contact(&peer_id, &update) {
            Ok(()) => {}
            Err(crate::storage::StorageError::NotFound(_)) => {
                self.database.insert_contact(&crate::storage::NewContact {
                    peer_id,
                    username: None,
                    display_name: None,
                    public_key: authenticated_public_key.unwrap_or_default(),
                    prekey_bundle_json: Some(bundle_json),
                })?;
            }
            Err(e) => return Err(e.into()),
        }
        tracing::info!("🔑 Automatically synchronized prekeys for {}", from_peer);
        Ok(())
    }

    /// Handle acknowledgment for an outgoing message
    pub async fn handle_outgoing_ack(&self, ack: AckMessage) -> Result<()> {
        tracing::info!(
            "✅ Received ACK for message {} - status: {:?}",
            ack.message_id,
            AckStatus::try_from(ack.status).unwrap_or(AckStatus::Unspecified)
        );

        // Update message status in database
        let status = match AckStatus::try_from(ack.status) {
            Ok(AckStatus::Received) => MessageStatus::Delivered,
            Ok(AckStatus::Error) => MessageStatus::Failed,
            _ => return Ok(()), // Ignore other statuses
        };

        {
            let update = UpdateMessage {
                status: Some(status),
                ..Default::default()
            };
            if let Err(e) = self.database.update_message(&ack.message_id, &update) {
                tracing::warn!("Failed to update message status: {}", e);
            }
        }

        // Emit event (include recipient when available)
        let to_peer_id = self
            .database
            .get_message(&ack.message_id)
            .ok()
            .and_then(|msg| msg.recipient_peer_id);

        self.emit_event(MessageEvent::MessageDelivered {
            message_id: ack.message_id.clone(),
            status,
            to_peer_id,
        });

        Ok(())
    }

    /// Re-enfileira uma mensagem cujo request outbound falhou (conexão caiu
    /// entre o send e o ACK). O worker de retry (builder) fará a reentrega;
    /// o status da mensagem regride para Pending em vez de ficar Sent.
    pub fn requeue_failed_outbound(
        &self,
        peer_id: &libp2p::PeerId,
        message: crate::protocol::Message,
    ) {
        use prost::Message as _;

        let proto_bytes = message.encode_to_vec();
        let next_attempt_at = chrono::Utc::now().timestamp() + 5;
        let message_type = crate::protocol::pb::MessageType::try_from(message.r#type)
            .map(|t| t.as_str_name().to_lowercase())
            .unwrap_or_else(|_| "unknown".to_string());

        match self.database.enqueue_outbound(
            &message.id,
            &peer_id.to_string(),
            &message_type,
            &proto_bytes,
            next_attempt_at,
        ) {
            Ok(()) => {
                let update = UpdateMessage {
                    status: Some(MessageStatus::Pending),
                    ..Default::default()
                };
                if let Err(e) = self.database.update_message(&message.id, &update) {
                    tracing::warn!("Failed to regress message status to Pending: {}", e);
                }
                tracing::info!(
                    "Outbound failure: message {} requeued for retry to {}",
                    message.id,
                    peer_id
                );
            }
            Err(e) => {
                tracing::warn!("Failed to requeue message {} for retry: {}", message.id, e);
            }
        }
    }

    /// Validate message format
    fn validate_message(&self, from_peer: &PeerId, message: &Message) -> Result<()> {
        // Check message ID
        if message.id.is_empty() {
            return Err(ZapLivreError::Protocol("Empty message ID".to_string()));
        }

        // Check sender
        if message.sender_peer_id.is_empty() {
            return Err(ZapLivreError::Protocol("Empty sender peer ID".to_string()));
        }

        // The declared sender must be the peer authenticated by the transport
        // (Noise for P2P, signed request for the message store). Without this
        // any connected peer could impersonate any contact.
        if message.sender_peer_id != from_peer.to_string() {
            return Err(ZapLivreError::Protocol(format!(
                "Sender mismatch: message claims {} but came from {}",
                message.sender_peer_id, from_peer
            )));
        }

        // SEC-01 on the receive side: plaintext Text also carries reaction,
        // media and group-control envelopes, so it is refused unless the
        // development downgrade is explicitly enabled.
        if matches!(message.payload, Some(Payload::Text(_))) && !self.allow_plaintext {
            return Err(ZapLivreError::Protocol(
                "Plaintext message refused: E2E is required".to_string(),
            ));
        }

        // Check recipient (should be us)
        if message.recipient_peer_id != self.local_peer_id {
            return Err(ZapLivreError::Protocol(format!(
                "Message not addressed to us (expected: {}, got: {})",
                self.local_peer_id, message.recipient_peer_id
            )));
        }

        // Check timestamp is not too old (> 7 days)
        let now = chrono::Utc::now().timestamp_millis();
        let age_ms = now - message.timestamp;
        if age_ms > 7 * 24 * 60 * 60 * 1000 {
            tracing::warn!(
                "Message {} is very old ({} days), but accepting anyway",
                message.id,
                age_ms / (24 * 60 * 60 * 1000)
            );
        }

        Ok(())
    }

    /// Handle text message
    async fn handle_text_message(&self, message: &Message, text: &TextMessage) -> Result<()> {
        // Never log message content: RUST_LOG can raise the level in production.
        tracing::debug!("📝 Received text ({} bytes)", text.content.len());

        if let Some(envelope) = ReactionEnvelope::decode(&text.content) {
            return self.handle_reaction_envelope(message, envelope).await;
        }

        if let Some(envelope) = MediaEnvelope::decode(&text.content) {
            return self.handle_media_envelope(message, envelope).await;
        }

        if let Some(offer) = MediaOfferEnvelope::decode(&text.content) {
            return self.handle_media_offer_envelope(message, offer).await;
        }

        if let Some(group_msg) = crate::group::decode_group_message(&text.content) {
            return self.handle_group_fanout(message, group_msg).await;
        }

        if let Some(envelope) = crate::group::GroupControlEnvelope::decode(&text.content) {
            self.emit_event(MessageEvent::GroupControl {
                from_peer_id: message.sender_peer_id.clone(),
                envelope,
            });
            return Ok(());
        }

        // Get or create conversation (Database has internal Mutex for thread-safety)
        let conversation_id = self
            .database
            .get_or_create_conversation(&message.sender_peer_id)?;

        // Store message in database
        let new_msg = NewMessage {
            message_id: message.id.clone(),
            conversation_id: conversation_id.clone(),
            sender_peer_id: message.sender_peer_id.clone(),
            recipient_peer_id: Some(message.recipient_peer_id.clone()),
            message_type: "text".to_string(),
            content_encrypted: self.encrypt_for_storage(text.content.as_bytes()).ok(),
            content_plaintext: None,
            status: MessageStatus::Delivered,
            parent_message_id: if text.reply_to_id.is_empty() {
                None
            } else {
                Some(text.reply_to_id.clone())
            },
        };

        self.database.insert_message(&new_msg)?;

        // Update conversation last message
        self.database
            .update_conversation_last_message(&conversation_id, &message.id)?;

        // UX-04: marca como não lida quando uma mensagem chega
        self.database
            .increment_conversation_unread(&conversation_id)?;

        tracing::info!(
            "💾 Stored message {} in conversation {}",
            message.id,
            conversation_id
        );

        // Emit event to UI
        self.emit_event(MessageEvent::MessageReceived {
            message_id: message.id.clone(),
            from_peer_id: message.sender_peer_id.clone(),
            conversation_id: conversation_id.clone(),
            content: text.content.clone(),
            message: message.clone(),
        });

        Ok(())
    }

    /// Handle acknowledgment message
    async fn handle_ack_message(&self, _message: &Message, ack: &AckMessage) -> Result<()> {
        // This is an ACK for one of our messages
        self.handle_outgoing_ack(ack.clone()).await
    }

    async fn handle_encrypted_message(
        &self,
        message: &Message,
        encrypted: &ProtoEncryptedMessage,
    ) -> Result<()> {
        let peer_id = message.sender_peer_id.clone();

        let device_id = if encrypted.sender_device_id != 0 {
            encrypted.sender_device_id
        } else {
            1
        };
        let crypto_msg = SignalEncryptedMessage {
            ciphertext: encrypted.ciphertext.clone(),
            ciphertext_type: encrypted.ciphertext_type,
            sender_device_id: device_id,
        };

        let plaintext = self
            .session_manager
            .decrypt_from(&peer_id, device_id, &crypto_msg)
            .await?;
        let text = String::from_utf8(plaintext)
            .map_err(|_| ZapLivreError::Protocol("Invalid UTF-8 content".to_string()))?;

        if let Some(envelope) = ReactionEnvelope::decode(&text) {
            return self.handle_reaction_envelope(message, envelope).await;
        }

        if let Some(envelope) = MediaEnvelope::decode(&text) {
            return self.handle_media_envelope(message, envelope).await;
        }

        if let Some(offer) = MediaOfferEnvelope::decode(&text) {
            return self.handle_media_offer_envelope(message, offer).await;
        }

        if let Some(group_msg) = crate::group::decode_group_message(&text) {
            return self.handle_group_fanout(message, group_msg).await;
        }

        if let Some(envelope) = crate::group::GroupControlEnvelope::decode(&text) {
            self.emit_event(MessageEvent::GroupControl {
                from_peer_id: message.sender_peer_id.clone(),
                envelope,
            });
            return Ok(());
        }

        let conversation_id = self.database.get_or_create_conversation(&peer_id)?;
        let new_msg = NewMessage {
            message_id: message.id.clone(),
            conversation_id: conversation_id.clone(),
            sender_peer_id: message.sender_peer_id.clone(),
            recipient_peer_id: Some(message.recipient_peer_id.clone()),
            message_type: "text".to_string(),
            content_encrypted: self.encrypt_for_storage(text.as_bytes()).ok(),
            content_plaintext: None,
            status: MessageStatus::Delivered,
            parent_message_id: None,
        };

        self.database.insert_message(&new_msg)?;
        self.database
            .update_conversation_last_message(&conversation_id, &message.id)?;

        // UX-04: marca como não lida quando uma mensagem chega
        self.database
            .increment_conversation_unread(&conversation_id)?;

        let mut display_message = message.clone();
        display_message.payload = Some(Payload::Text(TextMessage {
            content: text.clone(),
            reply_to_id: String::new(),
            metadata: std::collections::HashMap::new(),
        }));
        display_message.r#type = MessageType::Text as i32;

        self.emit_event(MessageEvent::MessageReceived {
            message_id: message.id.clone(),
            from_peer_id: message.sender_peer_id.clone(),
            conversation_id,
            content: text,
            message: display_message,
        });

        Ok(())
    }

    /// Handle typing indicator
    async fn handle_typing_indicator(
        &self,
        message: &Message,
        typing: &TypingIndicator,
    ) -> Result<()> {
        tracing::debug!(
            "⌨️ Typing indicator from {}: {}",
            message.sender_peer_id,
            typing.is_typing
        );

        // Emit event to UI
        self.emit_event(MessageEvent::TypingIndicator {
            from_peer_id: message.sender_peer_id.clone(),
            is_typing: typing.is_typing,
        });

        Ok(())
    }

    /// Handle read receipt
    async fn handle_read_receipt(&self, message: &Message, read: &ReadReceipt) -> Result<()> {
        tracing::debug!(
            "✓✓ Read receipt from {} for message {}",
            message.sender_peer_id,
            read.message_id
        );

        // Update message status in database
        {
            let update = UpdateMessage {
                status: Some(MessageStatus::Read),
                read_at: Some(read.read_at),
                ..Default::default()
            };
            if let Err(e) = self.database.update_message(&read.message_id, &update) {
                tracing::warn!("Failed to update message read status: {}", e);
            }
        }

        // Emit event to UI
        self.emit_event(MessageEvent::MessageRead {
            message_id: read.message_id.clone(),
            by_peer_id: message.sender_peer_id.clone(),
            read_at: read.read_at,
        });

        Ok(())
    }

    /// Record a sealed media offer. The file itself is pulled later, on demand.
    async fn handle_media_offer_envelope(
        &self,
        message: &Message,
        offer: MediaOfferEnvelope,
    ) -> Result<()> {
        // The hash names files on disk and comes from the peer.
        if !is_sha256_hex(&offer.media_hash) {
            return Err(ZapLivreError::Protocol(
                "Invalid media hash in offer".to_string(),
            ));
        }
        if offer
            .sealed_size()
            .is_none_or(|size| size > MAX_MEDIA_BYTES)
        {
            return Err(ZapLivreError::Protocol(
                "Media offer size out of range".to_string(),
            ));
        }

        let media_type = MediaType::from_db_str(&offer.media_type);
        let summary = crate::media::media_summary(
            media_type.as_str(),
            offer.file_name.as_deref(),
            offer.duration_seconds,
        );
        let conversation_id = self
            .database
            .get_or_create_conversation(&message.sender_peer_id)?;

        let mut thumbnail_path = None;
        if let Some(thumbnail_bytes) = offer.thumbnail_bytes()? {
            let thumb_dir = self.data_dir.join("media").join("thumbnails");
            std::fs::create_dir_all(&thumb_dir).map_err(|e| {
                ZapLivreError::Storage(format!("Failed to create thumbnail dir: {}", e))
            })?;
            let thumb_path = thumb_dir.join(format!("{}.jpg", offer.media_hash));
            std::fs::write(&thumb_path, &thumbnail_bytes).map_err(|e| {
                ZapLivreError::Storage(format!("Failed to write thumbnail file: {}", e))
            })?;
            thumbnail_path = Some(thumb_path.to_string_lossy().to_string());
        }

        // The offer (with the file key) is kept encrypted at rest: it is needed
        // to open the blob whenever the user decides to download it.
        let new_msg = NewMessage {
            message_id: message.id.clone(),
            conversation_id: conversation_id.clone(),
            sender_peer_id: message.sender_peer_id.clone(),
            recipient_peer_id: Some(message.recipient_peer_id.clone()),
            message_type: media_type.as_str().to_string(),
            content_encrypted: Some(self.encrypt_for_storage(offer.encode()?.as_bytes())?),
            content_plaintext: Some(summary.clone()),
            status: MessageStatus::Delivered,
            parent_message_id: None,
        };
        self.database.insert_message(&new_msg)?;
        self.database
            .update_conversation_last_message(&conversation_id, &message.id)?;

        let new_media = NewMedia {
            media_hash: offer.media_hash.clone(),
            message_id: message.id.clone(),
            media_type,
            file_name: offer.file_name.clone(),
            file_size: Some(offer.file_size),
            mime_type: offer.mime_type.clone(),
            local_path: None,
            thumbnail_path,
            width: offer.width,
            height: offer.height,
            duration_seconds: offer.duration_seconds,
        };
        self.database.insert_media(&new_media)?;

        let mut display_message = message.clone();
        display_message.payload = Some(Payload::Text(TextMessage {
            content: summary.clone(),
            reply_to_id: String::new(),
            metadata: std::collections::HashMap::new(),
        }));
        display_message.r#type = MessageType::Text as i32;

        self.emit_event(MessageEvent::MessageReceived {
            message_id: message.id.clone(),
            from_peer_id: message.sender_peer_id.clone(),
            conversation_id,
            content: summary,
            message: display_message,
        });

        Ok(())
    }

    /// The sealed-media offer stored with `message_id`, if that message has one.
    fn stored_media_offer(&self, message_id: &str) -> Result<MediaOfferEnvelope> {
        let message = self.database.get_message(message_id)?;
        let blob = message
            .content_encrypted
            .ok_or_else(|| ZapLivreError::NotFound("Message carries no media offer".to_string()))?;
        let content = crate::crypto::decrypt_for_storage(&self.storage_key, &blob)?;
        std::str::from_utf8(&content)
            .ok()
            .and_then(MediaOfferEnvelope::decode)
            .ok_or_else(|| ZapLivreError::NotFound("Message carries no media offer".to_string()))
    }

    async fn handle_media_chunk(&self, message: &Message, chunk: &MediaChunk) -> Result<()> {
        use std::io::{Seek, SeekFrom, Write};

        // Every field of the chunk is attacker-controlled. The hash becomes a
        // file name, so it must be exactly a SHA-256 hex digest (no separators).
        if !is_sha256_hex(&chunk.media_hash) {
            return Err(ZapLivreError::Protocol(
                "Invalid media hash in chunk".to_string(),
            ));
        }

        // Only accept chunks for media that was offered to us by this very
        // sender (already authenticated against the transport peer).
        let media = self
            .database
            .get_media_by_hash(&chunk.media_hash)?
            .ok_or_else(|| ZapLivreError::NotFound("Media record not found".to_string()))?;
        let offered_by_sender = self
            .database
            .get_message(&media.message_id)
            .map(|msg| msg.sender_peer_id == message.sender_peer_id)
            .unwrap_or(false);
        if !offered_by_sender {
            tracing::warn!(
                "🚫 Unsolicited media chunk for {} from {}",
                chunk.media_hash,
                message.sender_peer_id
            );
            return Err(ZapLivreError::Permission(
                "Media chunk from a peer that did not offer this media".to_string(),
            ));
        }
        if media.local_path.is_some() {
            // Already downloaded; late or repeated chunks are harmless.
            return Ok(());
        }

        // Chunks carry the sealed blob; the offer says how to open it and how
        // large it is, which bounds the write window (no sparse-file abuse).
        let offer = self.stored_media_offer(&media.message_id)?;
        let sealed_size = offer
            .sealed_size()
            .filter(|size| *size <= MAX_MEDIA_BYTES)
            .ok_or_else(|| ZapLivreError::Protocol("Media offer size out of range".to_string()))?;
        let offset = u64::try_from(chunk.offset)
            .map_err(|_| ZapLivreError::Protocol("Negative media chunk offset".to_string()))?;
        let end = offset.checked_add(chunk.data.len() as u64);
        if !matches!(end, Some(end) if end <= sealed_size) {
            return Err(ZapLivreError::Protocol(
                "Media chunk outside the offered file size".to_string(),
            ));
        }

        let tmp_dir = self.data_dir.join("media").join("tmp");
        std::fs::create_dir_all(&tmp_dir)
            .map_err(|e| ZapLivreError::Storage(format!("Failed to create tmp dir: {}", e)))?;
        let tmp_path = tmp_dir.join(format!("{}.part", chunk.media_hash));

        // Chunks are independent requests and may arrive in any order, so
        // completion is "every byte received", not "the chunk flagged last".
        let received = {
            let mut downloads = self.downloads.lock().unwrap_or_else(|e| e.into_inner());
            if !downloads.contains_key(&chunk.media_hash) {
                let _ = std::fs::remove_file(&tmp_path); // stale partial file
            }
            let parts = downloads.entry(chunk.media_hash.clone()).or_default();
            parts.insert(offset, chunk.data.len() as u64);
            parts.values().sum::<u64>()
        };

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&tmp_path)
            .map_err(|e| ZapLivreError::Storage(format!("Failed to open temp file: {}", e)))?;
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| ZapLivreError::Storage(format!("Failed to seek temp file: {}", e)))?;
        file.write_all(&chunk.data)
            .map_err(|e| ZapLivreError::Storage(format!("Failed to write chunk: {}", e)))?;
        drop(file);

        if received < sealed_size {
            return Ok(());
        }

        // Everything is here: verify the sealed blob, open it, keep only the
        // plaintext. Whatever happens, this download attempt is over.
        self.downloads
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&chunk.media_hash);
        let sealed = std::fs::read(&tmp_path);
        let _ = std::fs::remove_file(&tmp_path);
        let sealed = sealed.map_err(|e| {
            ZapLivreError::Storage(format!("Failed to read reassembled media: {}", e))
        })?;
        let plaintext = offer.open(&sealed).inspect_err(|e| {
            tracing::error!("🚫 Media {} discarded: {}", chunk.media_hash, e);
        })?;

        let extension = media
            .file_name
            .as_ref()
            .and_then(|name| std::path::Path::new(name).extension())
            .and_then(|ext| ext.to_str())
            .filter(|ext| ext.chars().all(|c| c.is_ascii_alphanumeric()));
        let file_name = match extension {
            Some(ext) => format!("{}.{}", chunk.media_hash, ext),
            None => chunk.media_hash.clone(),
        };
        let media_dir = self.data_dir.join("media");
        std::fs::create_dir_all(&media_dir)
            .map_err(|e| ZapLivreError::Storage(format!("Failed to create media dir: {}", e)))?;
        let final_path = media_dir.join(file_name);
        std::fs::write(&final_path, &plaintext)
            .map_err(|e| ZapLivreError::Storage(format!("Failed to write media file: {}", e)))?;

        self.database
            .update_media_local_path(media.id, &final_path.to_string_lossy())
            .map_err(|e| ZapLivreError::Storage(e.to_string()))?;

        Ok(())
    }

    pub async fn build_media_chunks(
        &self,
        from_peer: PeerId,
        request: &MediaRequest,
    ) -> Result<Vec<Message>> {
        let media = self
            .database
            .get_media_by_hash(&request.media_hash)?
            .ok_or_else(|| ZapLivreError::NotFound("Media not found".to_string()))?;

        // SEC-02: só servir mídia a peers que participam da conversa da mídia
        // (antes qualquer peer conectado podia baixar qualquer mídia pelo hash)
        let requester = from_peer.to_string();
        let authorized = self
            .database
            .get_message(&media.message_id)
            .map(|msg| {
                msg.sender_peer_id == requester
                    || msg.recipient_peer_id.as_deref() == Some(requester.as_str())
            })
            .unwrap_or(false);
        if !authorized {
            tracing::warn!(
                "🚫 Media request for {} from unauthorized peer {}",
                request.media_hash,
                from_peer
            );
            return Err(ZapLivreError::Permission(
                "Peer not authorized for this media".to_string(),
            ));
        }
        let local_path = media
            .local_path
            .ok_or_else(|| ZapLivreError::NotFound("Media file missing".to_string()))?;
        // Serve the sealed form only. It is re-created from the local file with
        // the key and nonce kept in the (encrypted at rest) offer; a media
        // without an offer was never meant to be pulled and is not served.
        let offer = self.stored_media_offer(&media.message_id)?;
        let data = offer.seal(&std::fs::read(&local_path)?)?;

        // The requester picks the chunk size; bound it so it can neither force
        // millions of tiny chunks nor a frame above the codec limit.
        let chunk_size = if request.chunk_size > 0 {
            (request.chunk_size as usize).clamp(4 * 1024, 1024 * 1024)
        } else {
            64 * 1024
        };

        let mut chunks = Vec::new();
        let mut offset: usize = if request.offset > 0 {
            std::cmp::min(request.offset as usize, data.len())
        } else {
            0
        };
        while offset < data.len() {
            let end = std::cmp::min(offset + chunk_size, data.len());
            let chunk_data = data[offset..end].to_vec();
            let is_last = end >= data.len();
            let chunk = MediaChunk {
                message_id: request.message_id.clone(),
                media_hash: request.media_hash.clone(),
                offset: offset as i64,
                data: chunk_data,
                is_last,
            };
            let msg = Message {
                id: uuid::Uuid::new_v4().to_string(),
                sender_peer_id: self.local_peer_id.clone(),
                recipient_peer_id: from_peer.to_string(),
                timestamp: chrono::Utc::now().timestamp_millis(),
                r#type: MessageType::MediaChunk as i32,
                payload: Some(Payload::MediaChunk(chunk)),
            };
            chunks.push(msg);
            offset = end;
        }

        Ok(chunks)
    }

    /// Create an acknowledgment message
    fn create_ack(&self, message_id: &str, status: AckStatus, error: Option<String>) -> AckMessage {
        AckMessage {
            message_id: message_id.to_string(),
            status: status as i32,
            error: error.unwrap_or_default(),
        }
    }

    /// Emit an event to the application layer.
    /// CORE-09: canal bounded - se a UI não consome (1024 pendentes), o
    /// evento é DESCARTADO com log em vez de crescer memória sem teto.
    /// A UI tem safety net de refresh periódico que cobre eventos perdidos.
    fn emit_event(&self, event: MessageEvent) {
        if let Some(ref tx) = self.event_tx {
            match tx.try_send(event) {
                Ok(()) => {}
                Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                    tracing::warn!("Event channel full - dropping message event");
                }
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                    tracing::warn!("Event channel closed - dropping message event");
                }
            }
        }
    }

    fn encrypt_for_storage(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        encrypt_for_storage(&self.storage_key, plaintext)
    }

    #[allow(dead_code)]
    fn decrypt_for_storage(&self, blob: &[u8]) -> Result<String> {
        let bytes = decrypt_for_storage(&self.storage_key, blob)?;
        let text = String::from_utf8(bytes)
            .map_err(|_| ZapLivreError::Protocol("Invalid UTF-8 content".to_string()))?;
        Ok(text)
    }

    async fn handle_media_envelope(
        &self,
        message: &Message,
        envelope: MediaEnvelope,
    ) -> Result<()> {
        let media_bytes = envelope.media_bytes()?;
        let mut hasher = Sha256::new();
        hasher.update(&media_bytes);
        let computed_hash = format!("{:x}", hasher.finalize());
        if computed_hash != envelope.media_hash {
            return Err(ZapLivreError::Protocol("Media hash mismatch".to_string()));
        }

        let media_type = MediaType::from_db_str(&envelope.media_type);
        let message_type = match media_type {
            MediaType::VoiceMessage => "voice",
            MediaType::Audio => "audio",
            MediaType::Image => "image",
            MediaType::Video => "video",
            MediaType::Document => "document",
        }
        .to_string();

        let conversation_id = self
            .database
            .get_or_create_conversation(&message.sender_peer_id)?;

        let summary = crate::media::media_summary(
            media_type.as_str(),
            envelope.file_name.as_deref(),
            envelope.duration_seconds,
        );

        let media_dir = self.data_dir.join("media");
        std::fs::create_dir_all(&media_dir)
            .map_err(|e| ZapLivreError::Storage(format!("Failed to create media dir: {}", e)))?;

        let extension = envelope
            .file_name
            .as_ref()
            .and_then(|name| std::path::Path::new(name).extension())
            .and_then(|ext| ext.to_str());
        let file_name = match extension {
            Some(ext) => format!("{}.{}", envelope.media_hash, ext),
            None => envelope.media_hash.clone(),
        };
        let media_path = media_dir.join(file_name);
        std::fs::write(&media_path, &media_bytes)
            .map_err(|e| ZapLivreError::Storage(format!("Failed to write media file: {}", e)))?;

        let mut thumbnail_path = None;
        if let Some(thumbnail_bytes) = envelope.thumbnail_bytes()? {
            let thumb_dir = media_dir.join("thumbnails");
            std::fs::create_dir_all(&thumb_dir).map_err(|e| {
                ZapLivreError::Storage(format!("Failed to create thumbnail dir: {}", e))
            })?;
            let thumb_path = thumb_dir.join(format!("{}.jpg", envelope.media_hash));
            std::fs::write(&thumb_path, &thumbnail_bytes).map_err(|e| {
                ZapLivreError::Storage(format!("Failed to write thumbnail file: {}", e))
            })?;
            thumbnail_path = Some(thumb_path.to_string_lossy().to_string());
        }

        let new_msg = NewMessage {
            message_id: message.id.clone(),
            conversation_id: conversation_id.clone(),
            sender_peer_id: message.sender_peer_id.clone(),
            recipient_peer_id: Some(message.recipient_peer_id.clone()),
            message_type: message_type.clone(),
            content_encrypted: self.encrypt_for_storage(envelope.encode()?.as_bytes()).ok(),
            content_plaintext: Some(summary.clone()),
            status: MessageStatus::Delivered,
            parent_message_id: None,
        };

        self.database.insert_message(&new_msg)?;
        self.database
            .update_conversation_last_message(&conversation_id, &message.id)?;

        let new_media = NewMedia {
            media_hash: envelope.media_hash.clone(),
            message_id: message.id.clone(),
            media_type,
            file_name: envelope.file_name.clone(),
            file_size: Some(media_bytes.len() as i64),
            mime_type: envelope.mime_type.clone(),
            local_path: Some(media_path.to_string_lossy().to_string()),
            thumbnail_path,
            width: envelope.width,
            height: envelope.height,
            duration_seconds: envelope.duration_seconds,
        };
        let _ = self.database.insert_media(&new_media);

        let mut display_message = message.clone();
        display_message.payload = Some(Payload::Text(TextMessage {
            content: summary.clone(),
            reply_to_id: String::new(),
            metadata: std::collections::HashMap::new(),
        }));
        display_message.r#type = MessageType::Text as i32;

        self.emit_event(MessageEvent::MessageReceived {
            message_id: message.id.clone(),
            from_peer_id: message.sender_peer_id.clone(),
            conversation_id,
            content: summary,
            message: display_message,
        });

        Ok(())
    }

    async fn handle_reaction_envelope(
        &self,
        message: &Message,
        envelope: ReactionEnvelope,
    ) -> Result<()> {
        let peer_id = message.sender_peer_id.clone();

        match envelope.action.as_str() {
            "add" => {
                let new_reaction = NewReaction {
                    reaction_id: uuid::Uuid::new_v4().to_string(),
                    message_id: envelope.message_id,
                    peer_id,
                    emoji: envelope.emoji,
                };
                self.database.add_reaction(&new_reaction)?;
            }
            "remove" => {
                self.database
                    .remove_reaction(&envelope.message_id, &peer_id, &envelope.emoji)?;
            }
            other => {
                tracing::warn!("Unknown reaction action received: {}", other);
            }
        }

        Ok(())
    }
}

/// Extract an Ed25519 public key from a libp2p identity-multihash PeerId.
/// Lowercase hex SHA-256: the only shape accepted for peer-supplied media
/// hashes, since they end up in file names.
fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn public_key_from_peer_id(peer_id: &PeerId) -> Option<Vec<u8>> {
    let multihash = peer_id.as_ref();
    if multihash.code() != 0x00 {
        return None;
    }
    let public_key = libp2p::identity::PublicKey::try_decode_protobuf(multihash.digest()).ok()?;
    let ed25519 = public_key.try_into_ed25519().ok()?;
    Some(ed25519.to_bytes().to_vec())
}

/// Message events emitted to application layer
#[derive(Debug, Clone)]
pub enum MessageEvent {
    /// New message received
    MessageReceived {
        message_id: String,
        from_peer_id: String,
        conversation_id: String,
        content: String,
        message: Message,
    },

    /// Message delivered (ACK received)
    MessageDelivered {
        message_id: String,
        status: MessageStatus,
        to_peer_id: Option<String>,
    },

    /// Message read by recipient
    MessageRead {
        message_id: String,
        by_peer_id: String,
        read_at: i64,
    },

    /// Typing indicator
    TypingIndicator {
        from_peer_id: String,
        is_typing: bool,
    },

    /// Group control envelope received (invite/sender_key/membership)
    /// Processado pela task de orquestração no builder (tem acesso a rede)
    GroupControl {
        from_peer_id: String,
        envelope: crate::group::GroupControlEnvelope,
    },

    /// Group message fanned out over the 1:1 channel by `from_peer_id`.
    /// Handled by the same orchestration task, in arrival order.
    GroupMessage {
        from_peer_id: String,
        message: crate::group::GroupMessage,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_authenticated_ed25519_key_from_peer_id() {
        let keypair = libp2p::identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());
        let extracted = public_key_from_peer_id(&peer_id).expect("ed25519 key must be embedded");
        let expected = keypair.public().try_into_ed25519().unwrap().to_bytes();
        assert_eq!(extracted, expected);
    }
    use crate::storage::{contacts::NewContact, schema::init_schema};
    use libp2p::PeerId;

    /// Handler wired to an in-memory DB with `sender` already a contact.
    async fn text_fixture(
        allow_plaintext: bool,
    ) -> (
        MessageHandler,
        PeerId,
        String,
        tokio::sync::mpsc::Receiver<MessageEvent>,
    ) {
        let db = Database::in_memory().unwrap();
        init_schema(&db).unwrap();

        // Generate valid PeerId for sender
        let sender_peer = PeerId::random();
        let sender_peer_id = sender_peer.to_string();
        let local_peer_id = "local-peer".to_string();

        // Insert test contact (required for foreign keys)
        let contact = NewContact {
            peer_id: sender_peer_id.clone(),
            username: None,
            display_name: Some("Sender".to_string()),
            public_key: vec![1, 2, 3],
            prekey_bundle_json: None,
        };
        db.insert_contact(&contact).unwrap();

        let db_arc = Arc::new(db);

        let (event_tx, event_rx) = tokio::sync::mpsc::channel(64);

        let identity = Arc::new(RwLock::new(crate::identity::Identity::generate(0)));
        let storage_key = identity.read().await.storage_key().unwrap();
        let session_manager = SignalSessionManager::new(Arc::clone(&identity));
        let handler = MessageHandler::new(
            local_peer_id.clone(),
            db_arc,
            std::env::temp_dir().join("zaplivre_test_media"),
            identity,
            session_manager,
            storage_key,
            Some(event_tx),
        )
        .allow_plaintext(allow_plaintext);

        (handler, sender_peer, local_peer_id, event_rx)
    }

    fn plaintext_message(id: &str, sender: &str, recipient: &str) -> Message {
        Message {
            id: id.to_string(),
            sender_peer_id: sender.to_string(),
            recipient_peer_id: recipient.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            r#type: MessageType::Text as i32,
            payload: Some(Payload::Text(TextMessage {
                content: "Hello, World!".to_string(),
                reply_to_id: String::new(),
                metadata: std::collections::HashMap::new(),
            })),
        }
    }

    #[tokio::test]
    async fn test_handle_text_message() {
        let (handler, sender_peer, local_peer_id, mut event_rx) = text_fixture(true).await;
        let message = plaintext_message("msg-123", &sender_peer.to_string(), &local_peer_id);

        // Handle message
        let ack = handler
            .handle_incoming_message(sender_peer, message)
            .await
            .unwrap();

        // Verify ACK
        assert_eq!(ack.message_id, "msg-123");
        assert_eq!(ack.status, AckStatus::Received as i32);

        // Verify event emitted
        let event = event_rx.recv().await.unwrap();
        match event {
            MessageEvent::MessageReceived {
                message_id,
                content,
                message,
                ..
            } => {
                assert_eq!(message_id, "msg-123");
                assert_eq!(content, "Hello, World!");
                assert_eq!(message.id, "msg-123");
            }
            _ => panic!("Expected MessageReceived event"),
        }
    }

    #[tokio::test]
    async fn test_plaintext_refused_by_default() {
        let (handler, sender_peer, local_peer_id, mut event_rx) = text_fixture(false).await;
        let message = plaintext_message("msg-plain", &sender_peer.to_string(), &local_peer_id);

        let ack = handler
            .handle_incoming_message(sender_peer, message)
            .await
            .unwrap();

        assert_eq!(ack.status, AckStatus::Error as i32);
        assert!(
            event_rx.try_recv().is_err(),
            "refused message must not reach the UI"
        );
    }

    #[tokio::test]
    async fn test_spoofed_sender_is_rejected() {
        let (handler, contact_peer, local_peer_id, mut event_rx) = text_fixture(true).await;
        // The attacker is authenticated as itself but claims to be the contact.
        let attacker = PeerId::random();
        let message = plaintext_message("msg-spoof", &contact_peer.to_string(), &local_peer_id);

        let ack = handler
            .handle_incoming_message(attacker, message)
            .await
            .unwrap();

        assert_eq!(ack.status, AckStatus::Error as i32);
        assert!(
            event_rx.try_recv().is_err(),
            "spoofed message must not reach the UI"
        );
    }

    #[tokio::test]
    async fn test_handle_ack() {
        let db = Database::in_memory().unwrap();
        init_schema(&db).unwrap();

        // Insert test contacts (required for foreign keys)
        let local_peer_id = "local-peer".to_string();
        let remote_peer_id = PeerId::random().to_string();

        // Insert local peer as contact
        let local_contact = NewContact {
            peer_id: local_peer_id.clone(),
            username: None,
            display_name: Some("Local".to_string()),
            public_key: vec![1, 2, 3],
            prekey_bundle_json: None,
        };
        db.insert_contact(&local_contact).unwrap();

        // Insert remote peer as contact
        let remote_contact = NewContact {
            peer_id: remote_peer_id.clone(),
            username: None,
            display_name: Some("Remote".to_string()),
            public_key: vec![4, 5, 6],
            prekey_bundle_json: None,
        };
        db.insert_contact(&remote_contact).unwrap();

        // Create conversation first
        let conversation_id = db.get_or_create_conversation(&remote_peer_id).unwrap();

        // Insert a message first
        let new_msg = NewMessage {
            message_id: "msg-456".to_string(),
            conversation_id,
            sender_peer_id: local_peer_id.clone(),
            recipient_peer_id: Some(remote_peer_id),
            message_type: "text".to_string(),
            content_encrypted: None,
            content_plaintext: Some("Test".to_string()),
            status: MessageStatus::Sent,
            parent_message_id: None,
        };
        db.insert_message(&new_msg).unwrap();

        let db_arc = Arc::new(db);

        let identity = Arc::new(RwLock::new(crate::identity::Identity::generate(0)));
        let storage_key = identity.read().await.storage_key().unwrap();
        let session_manager = SignalSessionManager::new(Arc::clone(&identity));
        let handler = MessageHandler::new(
            local_peer_id,
            Arc::clone(&db_arc),
            std::env::temp_dir().join("zaplivre_test_media"),
            identity,
            session_manager,
            storage_key,
            None,
        );

        // Create ACK message
        let ack = AckMessage {
            message_id: "msg-456".to_string(),
            status: AckStatus::Received as i32,
            error: String::new(),
        };

        // Handle ACK
        handler.handle_outgoing_ack(ack).await.unwrap();

        // Verify message status updated
        {
            let message = db_arc.get_message("msg-456").unwrap();
            assert_eq!(message.status, MessageStatus::Delivered);
        }
    }

    /// SEC-03: helper - handler + DB com contato, mensagem e registro de mídia
    async fn media_test_setup(
        data: &[u8],
        tmp: &std::path::Path,
    ) -> (MessageHandler, String, String, String, Vec<u8>) {
        let db = Database::in_memory().unwrap();
        init_schema(&db).unwrap();

        let identity = Arc::new(RwLock::new(crate::identity::Identity::generate(0)));
        let storage_key = identity.read().await.storage_key().unwrap();
        let offer = crate::media::transfer::new_offer(
            crate::media::MediaOfferMeta {
                media_type: "document",
                file_name: Some("doc.bin".to_string()),
                mime_type: Some("application/octet-stream".to_string()),
                width: None,
                height: None,
                duration_seconds: None,
                thumbnail: None,
            },
            data,
        )
        .unwrap();
        let sealed = offer.seal(data).unwrap();

        let sender_peer = PeerId::random().to_string();
        db.insert_contact(&NewContact {
            peer_id: sender_peer.clone(),
            username: None,
            display_name: None,
            public_key: vec![1, 2, 3],
            prekey_bundle_json: None,
        })
        .unwrap();

        let conversation_id = db.get_or_create_conversation(&sender_peer).unwrap();
        let message_id = "media-msg-1".to_string();
        db.insert_message(&crate::storage::messages::NewMessage {
            message_id: message_id.clone(),
            conversation_id,
            sender_peer_id: sender_peer.clone(),
            recipient_peer_id: Some("local-peer".to_string()),
            message_type: "document".to_string(),
            // The sealed offer, as the receive path stores it
            content_encrypted: Some(
                crate::crypto::encrypt_for_storage(
                    &storage_key,
                    offer.encode().unwrap().as_bytes(),
                )
                .unwrap(),
            ),
            content_plaintext: None,
            status: crate::storage::MessageStatus::Delivered,
            parent_message_id: None,
        })
        .unwrap();

        let media_hash = offer.media_hash.clone();
        db.insert_media(&crate::storage::media::NewMedia {
            media_hash: media_hash.clone(),
            message_id: message_id.clone(),
            media_type: crate::storage::media::MediaType::Document,
            file_name: Some("doc.bin".to_string()),
            file_size: Some(data.len() as i64),
            mime_type: Some("application/octet-stream".to_string()),
            local_path: None,
            thumbnail_path: None,
            width: None,
            height: None,
            duration_seconds: None,
        })
        .unwrap();

        let db_arc = Arc::new(db);
        let session_manager = SignalSessionManager::new(Arc::clone(&identity));
        let handler = MessageHandler::new(
            "local-peer".to_string(),
            Arc::clone(&db_arc),
            tmp.to_path_buf(),
            identity,
            session_manager,
            storage_key,
            None,
        );

        (handler, sender_peer, media_hash, message_id, sealed)
    }

    fn media_chunk_message(sender_peer_id: &str) -> Message {
        Message {
            id: "chunk-envelope".to_string(),
            sender_peer_id: sender_peer_id.to_string(),
            recipient_peer_id: "local-peer".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            r#type: MessageType::MediaChunk as i32,
            payload: None,
        }
    }

    /// SEC-03: arquivo remontado com hash correto é aceito e movido para media/
    #[tokio::test]
    async fn test_media_chunk_integrity_accepts_valid_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let data = b"conteudo legitimo do arquivo".to_vec();
        let (handler, sender, media_hash, message_id, sealed) =
            media_test_setup(&data, tmp.path()).await;
        let envelope = media_chunk_message(&sender);

        // Chunks são requests independentes: o "último" pode chegar primeiro.
        let split = sealed.len() / 2;
        for (offset, part, is_last) in [
            (split, &sealed[split..], true),
            (0, &sealed[..split], false),
        ] {
            let chunk = MediaChunk {
                message_id: message_id.clone(),
                media_hash: media_hash.clone(),
                offset: offset as i64,
                data: part.to_vec(),
                is_last,
            };
            handler
                .handle_media_chunk(&envelope, &chunk)
                .await
                .expect("valid media must be accepted");
        }

        // Só o plaintext fica em disco: o blob selado (.part) é removido
        let part = tmp.path().join("media").join("tmp");
        assert!(std::fs::read_dir(&part).unwrap().next().is_none());

        // Arquivo final existe em media/
        let final_path = tmp.path().join("media").join(format!("{}.bin", media_hash));
        assert!(final_path.exists(), "reassembled media missing");
        assert_eq!(std::fs::read(&final_path).unwrap(), data);
    }

    /// SEC-03: chunk adulterado (hash não bate) é rejeitado e o .part removido
    #[tokio::test]
    async fn test_media_chunk_integrity_rejects_tampered_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let data = b"conteudo legitimo do arquivo".to_vec();
        let (handler, sender, media_hash, message_id, sealed) =
            media_test_setup(&data, tmp.path()).await;

        // Peer malicioso envia bytes diferentes sob o mesmo media_hash
        let chunk = MediaChunk {
            message_id,
            media_hash: media_hash.clone(),
            offset: 0,
            // Mesmo tamanho ofertado, conteúdo diferente
            data: sealed.iter().map(|b| b ^ 0xFF).collect(),
            is_last: true,
        };
        let envelope = media_chunk_message(&sender);

        let err = handler
            .handle_media_chunk(&envelope, &chunk)
            .await
            .expect_err("tampered media must be rejected");
        assert!(
            err.to_string().to_lowercase().contains("integrity"),
            "unexpected error: {}",
            err
        );

        // Nem o .part nem o arquivo final podem sobrar
        let part = tmp
            .path()
            .join("media")
            .join("tmp")
            .join(format!("{}.part", media_hash));
        assert!(!part.exists(), ".part file must be deleted on rejection");
        let final_path = tmp.path().join("media").join(format!("{}.bin", media_hash));
        assert!(!final_path.exists());
    }

    /// C6: o hash vira nome de arquivo; separadores de caminho são recusados
    #[tokio::test]
    async fn test_media_chunk_rejects_path_traversal_hash() {
        let tmp = tempfile::TempDir::new().unwrap();
        let data = b"conteudo".to_vec();
        let (handler, sender, _hash, message_id, _sealed) =
            media_test_setup(&data, tmp.path()).await;

        let chunk = MediaChunk {
            message_id,
            media_hash: "../../../../escape".to_string(),
            offset: 0,
            data,
            is_last: false,
        };
        let err = handler
            .handle_media_chunk(&media_chunk_message(&sender), &chunk)
            .await
            .expect_err("traversal hash must be rejected");
        assert!(err.to_string().contains("Invalid media hash"), "{err}");
        assert!(!tmp.path().join("media").exists(), "nothing may be written");
    }

    /// C6: só quem ofertou a mídia pode enviar os chunks dela
    #[tokio::test]
    async fn test_media_chunk_rejects_unsolicited_sender() {
        let tmp = tempfile::TempDir::new().unwrap();
        let data = b"conteudo".to_vec();
        let (handler, _sender, media_hash, message_id, _sealed) =
            media_test_setup(&data, tmp.path()).await;

        let chunk = MediaChunk {
            message_id,
            media_hash,
            offset: 0,
            data,
            is_last: true,
        };
        let intruder = PeerId::random().to_string();
        handler
            .handle_media_chunk(&media_chunk_message(&intruder), &chunk)
            .await
            .expect_err("chunk from a peer that did not offer the media must be rejected");
        assert!(!tmp.path().join("media").exists(), "nothing may be written");
    }

    /// C6: offset fora do tamanho ofertado não pode criar arquivo esparso
    #[tokio::test]
    async fn test_media_chunk_rejects_offset_beyond_offered_size() {
        let tmp = tempfile::TempDir::new().unwrap();
        let data = b"conteudo".to_vec();
        let (handler, sender, media_hash, message_id, _sealed) =
            media_test_setup(&data, tmp.path()).await;

        for offset in [1_i64 << 40, -1] {
            let chunk = MediaChunk {
                message_id: message_id.clone(),
                media_hash: media_hash.clone(),
                offset,
                data: data.clone(),
                is_last: false,
            };
            handler
                .handle_media_chunk(&media_chunk_message(&sender), &chunk)
                .await
                .expect_err("out-of-range offset must be rejected");
        }
        assert!(!tmp.path().join("media").exists(), "nothing may be written");
    }

    /// P0-H: a segunda entrega da mesma mensagem é confirmada, não reprocessada
    #[tokio::test]
    async fn test_duplicate_message_is_acknowledged_once() {
        let (handler, sender_peer, local_peer_id, mut event_rx) = text_fixture(true).await;
        let sender = sender_peer.to_string();

        for _ in 0..2 {
            let message = plaintext_message("msg-dup", &sender, &local_peer_id);
            let ack = handler
                .handle_incoming_message(sender_peer, message)
                .await
                .unwrap();
            assert_eq!(ack.status, AckStatus::Received as i32);
        }

        assert!(event_rx.try_recv().is_ok(), "first delivery reaches the UI");
        assert!(
            event_rx.try_recv().is_err(),
            "duplicate must not reach the UI"
        );
    }
}
