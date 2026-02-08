//! Client Builder
//!
//! Builder pattern for creating MePassa clients.

use libp2p::{identity::Keypair, PeerId};
use base64::{engine::general_purpose, Engine as _};
use std::path::PathBuf;
use std::str::FromStr;

use super::client::Client;
use crate::{
    crypto::SignalSessionManager,
    identity::Identity,
    network::{MessageEvent, NetworkManager},
    storage::{Database, migrate, needs_migration},
    utils::error::{MePassaError, Result},
};
#[cfg(any(feature = "voip", feature = "video"))]
use crate::voip::{CallManager, VoIPIntegration};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Builder for creating a MePassa client
pub struct ClientBuilder {
    data_dir: Option<PathBuf>,
    keypair: Option<Keypair>,
    bootstrap_peers: Vec<(libp2p::PeerId, libp2p::Multiaddr)>,
    message_store_url: Option<String>,
    signaling_server_url: Option<String>,
}

impl ClientBuilder {
    /// Create a new client builder
    pub fn new() -> Self {
        Self {
            data_dir: None,
            keypair: None,
            bootstrap_peers: Vec::new(),
            message_store_url: None,
            signaling_server_url: None,
        }
    }

    /// Set data directory
    pub fn data_dir(mut self, path: PathBuf) -> Self {
        self.data_dir = Some(path);
        self
    }

    /// Set keypair (if None, will generate new one)
    pub fn keypair(mut self, keypair: Keypair) -> Self {
        self.keypair = Some(keypair);
        self
    }

    /// Add bootstrap peer
    pub fn add_bootstrap_peer(mut self, peer_id: libp2p::PeerId, addr: libp2p::Multiaddr) -> Self {
        self.bootstrap_peers.push((peer_id, addr));
        self
    }

    /// Set message store URL (store-and-forward)
    pub fn message_store_url(mut self, url: String) -> Self {
        self.message_store_url = Some(url);
        self
    }

    /// Set signaling server URL (WebRTC fallback signaling)
    pub fn signaling_server_url(mut self, url: String) -> Self {
        self.signaling_server_url = Some(url);
        self
    }

    /// Build the client
    pub async fn build(self) -> Result<Client> {
        // Get or create data directory
        let data_dir = self.data_dir.ok_or_else(|| {
            MePassaError::Other("Data directory is required".to_string())
        })?;

        // Create data directory if it doesn't exist
        std::fs::create_dir_all(&data_dir).map_err(|e| {
            MePassaError::Other(format!("Failed to create data directory: {}", e))
        })?;

        // Get or generate keypair
        let keypair = if let Some(keypair) = self.keypair {
            keypair
        } else if let Some(env_keypair) = load_keypair_from_env()? {
            env_keypair
        } else {
            // Try to load from file, or generate new one
            let keypair_path = data_dir.join("identity.key");
            if keypair_path.exists() {
                // Load keypair from file
                match load_keypair_from_file(&keypair_path) {
                    Ok(kp) => kp,
                    Err(e) => {
                        tracing::warn!("Failed to load keypair from file: {}, generating new one", e);
                        Keypair::generate_ed25519()
                    }
                }
            } else {
                // Generate new keypair and save to file
                let keypair = Keypair::generate_ed25519();
                if let Err(e) = save_keypair_to_file(&keypair, &keypair_path) {
                    tracing::warn!("Failed to save keypair to file: {}", e);
                }
                keypair
            }
        };

        // Create identity (convert from libp2p keypair)
        let our_keypair = crate::identity::Keypair::from_libp2p_keypair(&keypair)?;
        let mut identity = Identity::from_keypair(our_keypair);
        identity.init_prekey_pool(100);
        let storage_key = identity.storage_key()?;
        let identity = Arc::new(RwLock::new(identity));

        // Open database
        let db_path = data_dir.join("mepassa.db");
        let database = Database::open(&db_path)?;

        // Run migrations if needed
        if needs_migration(&database)? {
            migrate(&database)?;
        }

        // Get peer ID from libp2p keypair
        let peer_id = libp2p::PeerId::from(keypair.public());

        // Ensure local peer exists as contact (required for FOREIGN KEY constraints)
        ensure_local_contact_exists(&database, &peer_id.to_string(), &keypair)?;

        // Create network manager
        let network = NetworkManager::new(keypair)?;
        let network_arc = Arc::new(RwLock::new(network));

        let callbacks: Arc<RwLock<Vec<Box<dyn super::events::EventCallback>>>> =
            Arc::new(RwLock::new(Vec::new()));
        let callbacks_for_events = Arc::clone(&callbacks);

        let (event_tx, mut event_rx) = mpsc::unbounded_channel();

        // Create message handler for processing incoming messages
        // IMPORTANT: database.clone() shares the same SQLite connection (via internal Arc<Mutex>)
        // This ensures messages stored by MessageHandler are visible to Client
        let session_manager = SignalSessionManager::new(Arc::clone(&identity));
        let message_handler = Arc::new(crate::network::MessageHandler::new(
            peer_id.to_string(),
            Arc::new(database.clone()), // Shares the same SQLite connection!
            data_dir.clone(),
            Arc::clone(&identity),
            session_manager.clone(),
            storage_key,
            Some(event_tx),
        ));

        // Set message handler in network manager
        {
            let mut network = network_arc.write().await;
            network.set_message_handler(Arc::clone(&message_handler));
        }

        // Add bootstrap peers to DHT (if any)
        if !self.bootstrap_peers.is_empty() {
            tracing::info!("Adding {} bootstrap peers to DHT", self.bootstrap_peers.len());
            let mut network = network_arc.write().await;
            for (peer_id, addr) in self.bootstrap_peers {
                tracing::info!("  Adding bootstrap peer {} at {}", peer_id, addr);
                network.add_peer_to_dht(peer_id, addr);
            }
        }

        // Create VoIP components (only if feature is enabled)
        #[cfg(any(feature = "voip", feature = "video"))]
        let call_manager = Arc::new(CallManager::new());
        #[cfg(any(feature = "voip", feature = "video"))]
        let voip_integration = Arc::new(
            VoIPIntegration::new(
                Arc::clone(&network_arc),
                Arc::clone(&call_manager),
                self.signaling_server_url.clone(),
                peer_id,
            )
            .await,
        );
        #[cfg(any(feature = "voip", feature = "video"))]
        {
            let mut network = network_arc.write().await;
            network.set_voip_signaling_sender(voip_integration.signaling_sender());
        }
        #[cfg(any(feature = "voip", feature = "video"))]
        voip_integration.clone().spawn().await;

        // Create Group Manager (FASE 15)
        // database.clone() shares the same SQLite connection
        let group_manager = Arc::new(
            crate::group::GroupManager::new(
                peer_id.to_string(),
                Arc::new(database.clone()),
            )
            .map_err(|e| MePassaError::Other(format!("Failed to create group manager: {}", e)))?
        );

        // Initialize group manager (load existing groups)
        let group_topics = group_manager.init().await.map_err(|e| {
            MePassaError::Other(format!("Failed to initialize group manager: {}", e))
        })?;

        {
            let mut network = network_arc.write().await;
            network.set_group_manager(Arc::clone(&group_manager));
            for topic in group_topics {
                if let Err(err) = network.subscribe_gossipsub(&topic) {
                    tracing::warn!("Failed to subscribe to group topic: {}", err);
                }
            }
        }

        // Create client (keep network as Arc since it's shared with VoIPIntegration)
        // Note: database.clone() shares the same SQLite connection with MessageHandler
        let client = Client::new(
            peer_id,
            Arc::clone(&identity),
            network_arc,
            database, // Client owns the database (shares connection via internal Arc<Mutex>)
            data_dir,
            Arc::clone(&callbacks),
            session_manager.clone(),
            storage_key,
            self.message_store_url,
            #[cfg(any(feature = "voip", feature = "video"))]
            call_manager,
            #[cfg(any(feature = "voip", feature = "video"))]
            voip_integration,
            group_manager,
            message_handler,
        );

        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                if let Some(client_event) = map_message_event(event) {
                    let callbacks = callbacks_for_events.read().await;
                    for callback in callbacks.iter() {
                        callback.on_event(client_event.clone());
                    }
                }
            }
        });

        Ok(client)
    }
}

fn map_message_event(event: MessageEvent) -> Option<super::events::ClientEvent> {
    match event {
        MessageEvent::MessageReceived { message_id, message, .. } => {
            let from = PeerId::from_str(&message.sender_peer_id).ok()?;
            Some(super::events::ClientEvent::MessageReceived {
                message_id,
                from,
                message,
            })
        }
        MessageEvent::MessageDelivered {
            message_id,
            to_peer_id,
            ..
        } => {
            let to_peer_id = to_peer_id?;
            let to = PeerId::from_str(&to_peer_id).ok()?;
            Some(super::events::ClientEvent::MessageDelivered { message_id, to })
        }
        MessageEvent::MessageRead {
            message_id,
            by_peer_id,
            read_at,
        } => {
            let by = PeerId::from_str(&by_peer_id).ok()?;
            Some(super::events::ClientEvent::MessageRead {
                message_id,
                by,
                read_at,
            })
        }
        MessageEvent::TypingIndicator {
            from_peer_id,
            is_typing,
        } => {
            let peer_id = PeerId::from_str(&from_peer_id).ok()?;
            Some(if is_typing {
                super::events::ClientEvent::TypingStarted { peer_id }
            } else {
                super::events::ClientEvent::TypingStopped { peer_id }
            })
        }
    }
}

/// Load a keypair from a file
fn load_keypair_from_file(path: &std::path::Path) -> Result<Keypair> {
    let bytes = std::fs::read(path).map_err(|e| {
        MePassaError::Other(format!("Failed to read keypair file: {}", e))
    })?;

    // Try to parse as protobuf-encoded keypair
    Keypair::from_protobuf_encoding(&bytes).map_err(|e| {
        MePassaError::Other(format!("Failed to decode keypair: {}", e))
    })
}

/// Load a keypair from environment variable (base64-encoded protobuf)
fn load_keypair_from_env() -> Result<Option<Keypair>> {
    let encoded = match std::env::var("MEPASSA_IDENTITY_B64") {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };

    let encoded = encoded.trim();
    if encoded.is_empty() {
        return Ok(None);
    }

    let bytes = general_purpose::STANDARD.decode(encoded).map_err(|e| {
        MePassaError::Other(format!("Failed to decode identity from env: {}", e))
    })?;

    let keypair = Keypair::from_protobuf_encoding(&bytes).map_err(|e| {
        MePassaError::Other(format!("Failed to decode keypair from env: {}", e))
    })?;

    Ok(Some(keypair))
}

/// Save a keypair to a file
fn save_keypair_to_file(keypair: &Keypair, path: &std::path::Path) -> Result<()> {
    let bytes = keypair.to_protobuf_encoding().map_err(|e| {
        MePassaError::Other(format!("Failed to encode keypair: {}", e))
    })?;

    std::fs::write(path, bytes).map_err(|e| {
        MePassaError::Other(format!("Failed to write keypair file: {}", e))
    })
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Ensure the local peer exists as a contact in the database
/// This is required due to FOREIGN KEY constraints on messages.sender_peer_id
fn ensure_local_contact_exists(database: &Database, peer_id: &str, keypair: &Keypair) -> Result<()> {
    use crate::storage::NewContact;

    // Check if contact already exists
    if database.get_contact_by_peer_id(peer_id).is_ok() {
        return Ok(());
    }

    // Get public key bytes from keypair
    let public_key_bytes = keypair.public().encode_protobuf();

    // Create local peer as contact
    let local_contact = NewContact {
        peer_id: peer_id.to_string(),
        username: None,
        display_name: Some("Me".to_string()),
        public_key: public_key_bytes,
        prekey_bundle_json: None,
    };

    database.insert_contact(&local_contact).map_err(|e| {
        MePassaError::Other(format!("Failed to create local contact: {}", e))
    })?;

    tracing::info!("Created local contact for peer: {}", peer_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_builder() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        let client = ClientBuilder::new()
            .data_dir(data_dir.clone())
            .build()
            .await
            .unwrap();

        assert!(client.local_peer_id().to_string().len() > 0);

        // Database should be created
        assert!(data_dir.join("mepassa.db").exists());
    }

    #[tokio::test]
    async fn test_builder_with_keypair() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        let keypair = Keypair::generate_ed25519();
        let expected_peer_id = libp2p::PeerId::from(keypair.public());

        let client = ClientBuilder::new()
            .data_dir(data_dir)
            .keypair(keypair)
            .build()
            .await
            .unwrap();

        assert_eq!(client.local_peer_id(), expected_peer_id);
    }
}
