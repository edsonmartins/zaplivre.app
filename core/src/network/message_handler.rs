//! Message Handler
//!
//! Handles incoming messages from the network:
//! 1. Validates message format
//! 2. Decrypts content (if encrypted)
//! 3. Stores message in database
//! 4. Emits events to application layer
//! 5. Sends acknowledgment back to sender

use libp2p::PeerId;
use std::{path::PathBuf, sync::Arc};

use crate::{
    crypto::{
        decrypt_for_storage, encrypt_for_storage,
        SignalEncryptedMessage, SignalSessionManager,
    },
    media::MediaEnvelope,
    reactions::ReactionEnvelope,
    protocol::{
        pb::message::Payload, AckMessage, AckStatus, EncryptedMessage as ProtoEncryptedMessage,
        MediaChunk, MediaOffer, MediaRequest, Message, MessageType, ReadReceipt, TextMessage,
        TypingIndicator,
    },
    storage::{Database, MediaType, MessageStatus, NewMedia, NewMessage, NewReaction, UpdateMessage},
    utils::error::{MePassaError, Result},
};
use tokio::sync::RwLock;
use crate::identity::Identity;
use sha2::{Digest, Sha256};

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
    event_tx: Option<tokio::sync::mpsc::UnboundedSender<MessageEvent>>,
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
        event_tx: Option<tokio::sync::mpsc::UnboundedSender<MessageEvent>>,
    ) -> Self {
        Self {
            local_peer_id,
            database,
            data_dir,
            identity,
            session_manager,
            storage_key,
            event_tx,
        }
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
        if let Err(e) = self.validate_message(&message) {
            tracing::warn!("Invalid message {}: {}", message.id, e);
            return Ok(self.create_ack(&message.id, AckStatus::Error, Some(e.to_string())));
        }

        // Process based on message type
        let result = match message.payload {
            Some(Payload::Text(ref text_msg)) => {
                self.handle_text_message(&message, text_msg).await
            }
            Some(Payload::Ack(ref ack_msg)) => {
                self.handle_ack_message(&message, ack_msg).await
            }
            Some(Payload::Typing(ref typing_msg)) => {
                self.handle_typing_indicator(&message, typing_msg).await
            }
            Some(Payload::ReadReceipt(ref read_msg)) => {
                self.handle_read_receipt(&message, read_msg).await
            }
            Some(Payload::Encrypted(ref enc_msg)) => {
                self.handle_encrypted_message(&message, enc_msg).await
            }
            Some(Payload::MediaOffer(ref offer)) => {
                self.handle_media_offer(&message, offer).await
            }
            Some(Payload::MediaChunk(ref chunk)) => {
                self.handle_media_chunk(&message, chunk).await
            }
            Some(Payload::MediaRequest(_)) => {
                // Media requests are handled in NetworkManager to enable chunk sending.
                Ok(())
            }
            None => {
                tracing::warn!("Message {} has no payload", message.id);
                Err(MePassaError::Protocol(
                    "Message has no payload".to_string(),
                ))
            }
        };

        match result {
            Ok(_) => Ok(self.create_ack(&message.id, AckStatus::Received, None)),
            Err(e) => {
                tracing::error!("Failed to process message {}: {}", message.id, e);
                Ok(self.create_ack(&message.id, AckStatus::Error, Some(e.to_string())))
            }
        }
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

    /// Validate message format
    fn validate_message(&self, message: &Message) -> Result<()> {
        // Check message ID
        if message.id.is_empty() {
            return Err(MePassaError::Protocol("Empty message ID".to_string()));
        }

        // Check sender
        if message.sender_peer_id.is_empty() {
            return Err(MePassaError::Protocol("Empty sender peer ID".to_string()));
        }

        // Check recipient (should be us)
        if message.recipient_peer_id != self.local_peer_id {
            return Err(MePassaError::Protocol(format!(
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
        tracing::debug!("📝 Received text: \"{}\"", text.content);

        if let Some(envelope) = ReactionEnvelope::decode(&text.content) {
            return self.handle_reaction_envelope(message, envelope).await;
        }

        if let Some(envelope) = MediaEnvelope::decode(&text.content) {
            return self.handle_media_envelope(message, envelope).await;
        }

        // Get or create conversation (Database has internal Mutex for thread-safety)
        let conversation_id = self.database.get_or_create_conversation(&message.sender_peer_id)?;

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
        self.database.update_conversation_last_message(&conversation_id, &message.id)?;

        tracing::info!("💾 Stored message {} in conversation {}", message.id, conversation_id);

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
            .map_err(|_| MePassaError::Protocol("Invalid UTF-8 content".to_string()))?;

        if let Some(envelope) = ReactionEnvelope::decode(&text) {
            return self.handle_reaction_envelope(message, envelope).await;
        }

        if let Some(envelope) = MediaEnvelope::decode(&text) {
            return self.handle_media_envelope(message, envelope).await;
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
        self.database.update_conversation_last_message(&conversation_id, &message.id)?;

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

    async fn handle_media_offer(&self, message: &Message, offer: &MediaOffer) -> Result<()> {
        let media_type = MediaType::from_str(&offer.media_type);
        let summary = crate::media::media_summary(
            media_type.as_str(),
            Some(&offer.file_name),
            if offer.duration_seconds > 0 {
                Some(offer.duration_seconds)
            } else {
                None
            },
        );

        let conversation_id = self.database.get_or_create_conversation(&message.sender_peer_id)?;

        let new_msg = NewMessage {
            message_id: message.id.clone(),
            conversation_id: conversation_id.clone(),
            sender_peer_id: message.sender_peer_id.clone(),
            recipient_peer_id: Some(message.recipient_peer_id.clone()),
            message_type: media_type.as_str().to_string(),
            content_encrypted: None,
            content_plaintext: Some(summary.clone()),
            status: MessageStatus::Delivered,
            parent_message_id: None,
        };

        self.database.insert_message(&new_msg)?;
        self.database.update_conversation_last_message(&conversation_id, &message.id)?;

        let new_media = NewMedia {
            media_hash: offer.media_hash.clone(),
            message_id: offer.message_id.clone(),
            media_type,
            file_name: Some(offer.file_name.clone()),
            file_size: Some(offer.file_size),
            mime_type: Some(offer.mime_type.clone()),
            local_path: None,
            thumbnail_path: None,
            width: if offer.width > 0 { Some(offer.width) } else { None },
            height: if offer.height > 0 { Some(offer.height) } else { None },
            duration_seconds: if offer.duration_seconds > 0 {
                Some(offer.duration_seconds)
            } else {
                None
            },
        };
        let _ = self.database.insert_media(&new_media);

        self.emit_event(MessageEvent::MessageReceived {
            message_id: message.id.clone(),
            from_peer_id: message.sender_peer_id.clone(),
            conversation_id,
            content: summary,
            message: message.clone(),
        });

        Ok(())
    }

    async fn handle_media_chunk(&self, _message: &Message, chunk: &MediaChunk) -> Result<()> {
        use std::io::{Seek, SeekFrom, Write};

        let tmp_dir = self.data_dir.join("media").join("tmp");
        std::fs::create_dir_all(&tmp_dir)
            .map_err(|e| MePassaError::Storage(format!("Failed to create tmp dir: {}", e)))?;
        let tmp_path = tmp_dir.join(format!("{}.part", chunk.media_hash));
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&tmp_path)
            .map_err(|e| MePassaError::Storage(format!("Failed to open temp file: {}", e)))?;
        file.seek(SeekFrom::Start(chunk.offset as u64))
            .map_err(|e| MePassaError::Storage(format!("Failed to seek temp file: {}", e)))?;
        file.write_all(&chunk.data)
            .map_err(|e| MePassaError::Storage(format!("Failed to write chunk: {}", e)))?;

        if chunk.is_last {
            let media = self
                .database
                .get_media_by_hash(&chunk.media_hash)?
                .ok_or_else(|| MePassaError::NotFound("Media record not found".to_string()))?;

            let extension = media
                .file_name
                .as_ref()
                .and_then(|name| std::path::Path::new(name).extension())
                .and_then(|ext| ext.to_str());
            let file_name = match extension {
                Some(ext) => format!("{}.{}", chunk.media_hash, ext),
                None => chunk.media_hash.clone(),
            };
            let final_path = self.data_dir.join("media").join(file_name);
            std::fs::create_dir_all(self.data_dir.join("media"))
                .map_err(|e| MePassaError::Storage(format!("Failed to create media dir: {}", e)))?;
            std::fs::rename(&tmp_path, &final_path)
                .map_err(|e| MePassaError::Storage(format!("Failed to finalize media file: {}", e)))?;

            self.database
                .update_media_local_path(media.id, &final_path.to_string_lossy())
                .map_err(|e| MePassaError::Storage(e.to_string()))?;
        }

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
            .ok_or_else(|| MePassaError::NotFound("Media not found".to_string()))?;
        let local_path = media
            .local_path
            .ok_or_else(|| MePassaError::NotFound("Media file missing".to_string()))?;
        let data = std::fs::read(&local_path)?;

        let chunk_size = if request.chunk_size > 0 {
            request.chunk_size as usize
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

    /// Emit an event to the application layer
    fn emit_event(&self, event: MessageEvent) {
        if let Some(ref tx) = self.event_tx {
            if let Err(e) = tx.send(event) {
                tracing::warn!("Failed to emit message event: {}", e);
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
            .map_err(|_| MePassaError::Protocol("Invalid UTF-8 content".to_string()))?;
        Ok(text)
    }

    async fn handle_media_envelope(&self, message: &Message, envelope: MediaEnvelope) -> Result<()> {
        let media_bytes = envelope.media_bytes()?;
        let mut hasher = Sha256::new();
        hasher.update(&media_bytes);
        let computed_hash = format!("{:x}", hasher.finalize());
        if computed_hash != envelope.media_hash {
            return Err(MePassaError::Protocol("Media hash mismatch".to_string()));
        }

        let media_type = MediaType::from_str(&envelope.media_type);
        let message_type = match media_type {
            MediaType::VoiceMessage => "voice",
            MediaType::Audio => "audio",
            MediaType::Image => "image",
            MediaType::Video => "video",
            MediaType::Document => "document",
        }
        .to_string();

        let conversation_id = self.database.get_or_create_conversation(&message.sender_peer_id)?;

        let summary = crate::media::media_summary(
            media_type.as_str(),
            envelope.file_name.as_deref(),
            envelope.duration_seconds,
        );

        let media_dir = self.data_dir.join("media");
        std::fs::create_dir_all(&media_dir)
            .map_err(|e| MePassaError::Storage(format!("Failed to create media dir: {}", e)))?;

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
            .map_err(|e| MePassaError::Storage(format!("Failed to write media file: {}", e)))?;

        let mut thumbnail_path = None;
        if let Some(thumbnail_bytes) = envelope.thumbnail_bytes()? {
            let thumb_dir = media_dir.join("thumbnails");
            std::fs::create_dir_all(&thumb_dir).map_err(|e| {
                MePassaError::Storage(format!("Failed to create thumbnail dir: {}", e))
            })?;
            let thumb_path = thumb_dir.join(format!("{}.jpg", envelope.media_hash));
            std::fs::write(&thumb_path, &thumbnail_bytes).map_err(|e| {
                MePassaError::Storage(format!("Failed to write thumbnail file: {}", e))
            })?;
            thumbnail_path = Some(thumb_path.to_string_lossy().to_string());
        }

        let new_msg = NewMessage {
            message_id: message.id.clone(),
            conversation_id: conversation_id.clone(),
            sender_peer_id: message.sender_peer_id.clone(),
            recipient_peer_id: Some(message.recipient_peer_id.clone()),
            message_type: message_type.clone(),
            content_encrypted: self
                .encrypt_for_storage(envelope.encode()?.as_bytes())
                .ok(),
            content_plaintext: Some(summary.clone()),
            status: MessageStatus::Delivered,
            parent_message_id: None,
        };

        self.database.insert_message(&new_msg)?;
        self.database.update_conversation_last_message(&conversation_id, &message.id)?;

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{contacts::NewContact, schema::init_schema};
    use libp2p::PeerId;

    #[tokio::test]
    async fn test_handle_text_message() {
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

        let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();

        let identity = Arc::new(RwLock::new(crate::identity::Identity::generate(0)));
        let storage_key = identity.read().await.storage_key().unwrap();
        let session_manager = SignalSessionManager::new(Arc::clone(&identity));
        let handler = MessageHandler::new(
            local_peer_id.clone(),
            db_arc,
            std::env::temp_dir().join("mepassa_test_media"),
            identity,
            session_manager,
            storage_key,
            Some(event_tx),
        );

        // Create test message
        let message = Message {
            id: "msg-123".to_string(),
            sender_peer_id: sender_peer_id,
            recipient_peer_id: local_peer_id,
            timestamp: chrono::Utc::now().timestamp_millis(),
            r#type: MessageType::Text as i32,
            payload: Some(Payload::Text(TextMessage {
                content: "Hello, World!".to_string(),
                reply_to_id: String::new(),
                metadata: std::collections::HashMap::new(),
            })),
        };

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
            std::env::temp_dir().join("mepassa_test_media"),
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
}
