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
            // Identidade veio do secure storage da plataforma (Keychain/
            // Keystore via env): garantir que nenhum identity.key legado
            // permaneça em plaintext no disco (SEC-06)
            let legacy_key_file = data_dir.join("identity.key");
            if legacy_key_file.exists() {
                if let Err(e) = std::fs::remove_file(&legacy_key_file) {
                    tracing::warn!("Failed to remove legacy identity.key: {}", e);
                } else {
                    tracing::info!("🔐 Removed legacy plaintext identity.key (secure storage in use)");
                }
            }
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
        let storage_key = identity.storage_key()?;

        // Open database
        let db_path = data_dir.join("mepassa.db");
        let database = Database::open(&db_path)?;

        // Run migrations if needed
        if needs_migration(&database)? {
            migrate(&database)?;
        }

        // SEC-07: pool de prekeys persistido - o bundle publicado precisa
        // ser estável entre restarts (bundle novo a cada boot invalida os
        // bundles que os contatos já buscaram)
        let restored = match database.load_prekey_pool() {
            Ok(Some(blob)) => {
                match crate::crypto::storage::decrypt_for_storage(&storage_key, &blob)
                    .and_then(|bytes| identity.restore_prekey_pool(&bytes))
                {
                    Ok(()) => {
                        tracing::info!("🔑 Restored persisted prekey pool");
                        true
                    }
                    Err(e) => {
                        tracing::warn!("Failed to restore prekey pool ({}); regenerating", e);
                        false
                    }
                }
            }
            Ok(None) => false,
            Err(e) => {
                tracing::warn!("Failed to load prekey pool ({}); regenerating", e);
                false
            }
        };
        if !restored {
            identity.init_prekey_pool(100);
            if let Some(Ok(snapshot)) = identity.snapshot_prekey_pool() {
                match crate::crypto::storage::encrypt_for_storage(&storage_key, &snapshot) {
                    Ok(encrypted) => {
                        if let Err(e) = database.save_prekey_pool(&encrypted) {
                            tracing::warn!("Failed to persist prekey pool: {}", e);
                        }
                    }
                    Err(e) => tracing::warn!("Failed to encrypt prekey pool: {}", e),
                }
            }
        }

        let identity = Arc::new(RwLock::new(identity));

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

        // CORE-09: bounded - descarte logado no produtor em vez de OOM
        let (event_tx, mut event_rx) = mpsc::channel(1024);

        // Create message handler for processing incoming messages
        // IMPORTANT: database.clone() shares the same SQLite connection (via internal Arc<Mutex>)
        // This ensures messages stored by MessageHandler are visible to Client
        let session_manager = SignalSessionManager::new(Arc::clone(&identity));

        // SEC-04: sessões Signal e identidades TOFU persistidas em SQLite
        // (sessões cifradas com a storage key); restaura o estado existente
        if let Err(e) = session_manager
            .attach_persistence(database.clone(), storage_key)
            .await
        {
            tracing::warn!("Failed to attach signal session persistence: {}", e);
        }
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
                Arc::clone(&identity),
            )
            .await,
        );
        #[cfg(any(feature = "voip", feature = "video"))]
        {
            let mut network = network_arc.write().await;
            network.set_voip_signaling_sender(voip_integration.signaling_sender());
            network.set_voip_fallback_sender(voip_integration.fallback_sender());
        }
        #[cfg(any(feature = "voip", feature = "video"))]
        voip_integration.clone().spawn().await;

        // Create Group Manager (FASE 15)
        // database.clone() shares the same SQLite connection
        let group_manager = Arc::new(
            crate::group::GroupManager::new(
                peer_id.to_string(),
                Arc::new(database.clone()),
                storage_key,
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

        // Handles para o worker de retry outbound (clonados antes do move para Client::new)
        let retry_db = database.clone();
        let retry_network = Arc::clone(&network_arc);

        // Handles para a orquestração do protocolo de grupo (CORE-16)
        let gc_group_manager = Arc::clone(&group_manager);
        let gc_database = database.clone();
        let gc_session_manager = session_manager.clone();
        let gc_identity = Arc::clone(&identity);
        let gc_network = Arc::clone(&network_arc);
        let gc_store_url = self.message_store_url.clone();
        let gc_local_peer_id = peer_id.to_string();

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

        // spawn_local: o processamento de GroupControl acessa o NetworkManager
        // (!Sync). build() já exige LocalSet (ver worker de retry abaixo).
        let gc_http = reqwest::Client::new();
        tokio::task::spawn_local(async move {
            while let Some(event) = event_rx.recv().await {
                // Protocolo in-band de grupo: orquestrado aqui, onde há acesso
                // a rede/sessões/DB (o MessageHandler apenas detecta o envelope)
                if let MessageEvent::GroupControl {
                    ref from_peer_id,
                    ref envelope,
                } = event
                {
                    handle_group_control(
                        from_peer_id.clone(),
                        envelope.clone(),
                        &gc_group_manager,
                        &gc_database,
                        &gc_session_manager,
                        &gc_identity,
                        &gc_network,
                        &gc_store_url,
                        &gc_http,
                        &gc_local_peer_id,
                    )
                    .await;
                    continue;
                }

                if let Some(client_event) = map_message_event(event) {
                    let callbacks = callbacks_for_events.read().await;
                    for callback in callbacks.iter() {
                        callback.on_event(client_event.clone());
                    }
                }
            }
        });

        // Worker da fila de retry outbound: mensagens que não puderam ser
        // entregues (peer offline sem store, ou OutboundFailure) ficam em
        // outbound_queue e são reenviadas aqui com backoff exponencial.
        let retry_worker = async move {
            use prost::Message as _;
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;

                let now = chrono::Utc::now().timestamp();
                let due = match retry_db.due_outbound(now, 20) {
                    Ok(entries) => entries,
                    Err(e) => {
                        tracing::warn!("Outbound retry: failed to read queue: {}", e);
                        continue;
                    }
                };

                for entry in due {
                    let Ok(peer) = entry.peer_id.parse::<libp2p::PeerId>() else {
                        tracing::warn!(
                            "Outbound retry: invalid peer id {}, dropping entry",
                            entry.peer_id
                        );
                        let _ = retry_db.remove_outbound(entry.id);
                        continue;
                    };

                    let connected =
                        Client::ensure_peer_connected_with(Arc::clone(&retry_network), peer).await;

                    if connected {
                        match crate::protocol::Message::decode(entry.proto_bytes.as_slice()) {
                            Ok(msg) => {
                                let send_result = {
                                    let mut network = retry_network.write().await;
                                    network.send_message(peer, msg)
                                };
                                if send_result.is_ok() {
                                    let _ = retry_db.remove_outbound(entry.id);
                                    let _ = retry_db.update_message(
                                        &entry.message_id,
                                        &crate::storage::UpdateMessage {
                                            sent_at: Some(chrono::Utc::now().timestamp()),
                                            received_at: None,
                                            read_at: None,
                                            status: Some(crate::storage::MessageStatus::Sent),
                                            is_deleted: None,
                                        },
                                    );
                                    tracing::info!(
                                        "Outbound retry: delivered queued message {} to {}",
                                        entry.message_id,
                                        peer
                                    );
                                    continue;
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Outbound retry: corrupt entry {} ({}), dropping",
                                    entry.message_id,
                                    e
                                );
                                let _ = retry_db.remove_outbound(entry.id);
                                continue;
                            }
                        }
                    }

                    // Backoff exponencial: 5s * 2^attempts, teto de 15 min
                    let delay = (5i64 << entry.attempts.min(8)).min(900);
                    let _ = retry_db.bump_outbound_attempt(entry.id, now + delay);
                }

                // Alinhado ao TTL de 14 dias do store & forward
                let cutoff = now - 14 * 24 * 3600;
                if let Ok(purged) = retry_db.purge_expired_outbound(cutoff) {
                    if purged > 0 {
                        tracing::info!("Outbound retry: purged {} expired entries", purged);
                    }
                }
            }
        };

        // NetworkManager é !Sync (transport do Swarm), então o worker exige
        // spawn_local. Consequência: `ClientBuilder::build` deve rodar dentro
        // de um LocalSet - garantido no caminho FFI (thread dedicada) e já
        // exigido pelo VoIPIntegration quando a feature voip está ativa.
        tokio::task::spawn_local(retry_worker);

        Ok(client)
    }
}

/// Orquestra o protocolo in-band de grupo (CORE-16).
///
/// - `invite`: cria o grupo local, subscreve o topic, guarda a seed do
///   convidante e responde `sender_key` (minha seed) a todos os membros
/// - `sender_key`: só aceita de membros do grupo (anti-spoofing)
/// - `member_added`/`member_removed`: só aceitos de admins
/// - `leave`: remove o remetente do grupo
#[allow(clippy::too_many_arguments)]
async fn handle_group_control(
    from_peer_id: String,
    envelope: crate::group::GroupControlEnvelope,
    group_manager: &Arc<crate::group::GroupManager>,
    database: &crate::storage::Database,
    session_manager: &crate::crypto::SignalSessionManager,
    identity: &Arc<RwLock<Identity>>,
    network: &Arc<RwLock<NetworkManager>>,
    message_store_url: &Option<String>,
    http: &reqwest::Client,
    local_peer_id: &str,
) {
    use crate::group::envelope::actions;
    use base64::{engine::general_purpose, Engine as _};

    let group_id = envelope.group_id.clone();
    tracing::info!(
        "👥 Group control '{}' for {} from {}",
        envelope.action,
        group_id,
        from_peer_id
    );

    match envelope.action.as_str() {
        actions::INVITE => {
            let name = envelope
                .group_name
                .clone()
                .unwrap_or_else(|| "Grupo".to_string());
            let creator = envelope
                .creator_peer_id
                .clone()
                .unwrap_or_else(|| from_peer_id.clone());
            let members = envelope.members.clone().unwrap_or_default();

            let topic = match group_manager
                .join_group_from_invite(
                    group_id.clone(),
                    name,
                    envelope.group_description.clone(),
                    creator,
                    members.clone(),
                )
                .await
            {
                Ok(topic) => topic,
                Err(e) => {
                    tracing::warn!("Failed to join group {} from invite: {}", group_id, e);
                    return;
                }
            };

            {
                let mut network = network.write().await;
                if let Err(e) = network.subscribe_gossipsub(&topic) {
                    tracing::warn!("Failed to subscribe to group topic: {}", e);
                }
            }

            // Seed do convidante
            if let Some(seed_b64) = &envelope.sender_key_seed {
                match general_purpose::STANDARD.decode(seed_b64) {
                    Ok(seed) => {
                        if let Err(e) =
                            group_manager.add_group_sender_key(&group_id, &from_peer_id, &seed)
                        {
                            tracing::warn!("Failed to store inviter sender key: {}", e);
                        }
                    }
                    Err(e) => tracing::warn!("Invalid sender key seed in invite: {}", e),
                }
            }

            // Responder com a MINHA seed para todos os membros do snapshot
            let my_seed = match group_manager.get_group_sender_key_seed(&group_id) {
                Ok(seed) => seed,
                Err(e) => {
                    tracing::warn!("Missing own sender key after join: {}", e);
                    return;
                }
            };
            let reply = crate::group::GroupControlEnvelope {
                version: 1,
                action: actions::SENDER_KEY.to_string(),
                group_id: group_id.clone(),
                group_name: None,
                group_description: None,
                creator_peer_id: None,
                members: None,
                member_peer_id: None,
                sender_key_seed: Some(general_purpose::STANDARD.encode(&my_seed)),
            };

            for member in members.iter().filter(|m| m.as_str() != local_peer_id) {
                let Ok(peer) = member.parse::<libp2p::PeerId>() else {
                    continue;
                };
                if let Err(e) = Client::send_group_control_with(
                    database,
                    session_manager,
                    Arc::clone(network),
                    Arc::clone(identity),
                    message_store_url.clone(),
                    http.clone(),
                    local_peer_id,
                    peer,
                    &reply,
                )
                .await
                {
                    tracing::warn!("Failed to send sender_key to {}: {}", member, e);
                }
            }
        }

        actions::SENDER_KEY => {
            // Anti-spoofing: só membros do grupo podem registrar sender key
            if !group_manager.is_group_member(&group_id, &from_peer_id).await {
                tracing::warn!(
                    "Rejected sender_key for {} from non-member {}",
                    group_id,
                    from_peer_id
                );
                return;
            }
            if let Some(seed_b64) = &envelope.sender_key_seed {
                match general_purpose::STANDARD.decode(seed_b64) {
                    Ok(seed) => {
                        if let Err(e) =
                            group_manager.add_group_sender_key(&group_id, &from_peer_id, &seed)
                        {
                            tracing::warn!("Failed to store sender key: {}", e);
                        }
                    }
                    Err(e) => tracing::warn!("Invalid sender key seed: {}", e),
                }
            }
        }

        actions::MEMBER_ADDED => {
            if let Some(new_member) = &envelope.member_peer_id {
                if let Err(e) = group_manager
                    .remote_add_member(&group_id, new_member, &from_peer_id)
                    .await
                {
                    tracing::warn!("Rejected remote member_added: {}", e);
                }
            }
        }

        actions::MEMBER_REMOVED => {
            if let Some(removed) = &envelope.member_peer_id {
                // Capturar o topic antes (se EU for o removido, o grupo some do manager)
                let topic = group_manager
                    .get_group(&group_id)
                    .await
                    .map(|g| libp2p::gossipsub::IdentTopic::new(&g.topic));

                match group_manager
                    .remote_remove_member(&group_id, removed, &from_peer_id)
                    .await
                {
                    Ok(true) => {
                        // Fui removido: sair do topic
                        if let Some(topic) = topic {
                            let mut network = network.write().await;
                            let _ = network.unsubscribe_gossipsub(&topic);
                        }
                    }
                    Ok(false) => {}
                    Err(e) => tracing::warn!("Rejected remote member_removed: {}", e),
                }
            }
        }

        actions::LEAVE => {
            if let Err(e) = group_manager
                .remote_member_left(&group_id, &from_peer_id)
                .await
            {
                tracing::warn!("Failed to process member leave: {}", e);
            }
        }

        other => {
            tracing::warn!("Unknown group control action: {}", other);
        }
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
        // Tratado antes do mapeamento (handle_group_control); nunca vira ClientEvent
        MessageEvent::GroupControl { .. } => None,
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

    // build() spawna workers com spawn_local, então precisa rodar em LocalSet
    // (igual ao caminho FFI de produção)

    #[tokio::test]
    async fn test_builder() {
        let local = tokio::task::LocalSet::new();
        local
            .run_until(async {
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
            })
            .await;
    }

    #[tokio::test]
    async fn test_builder_with_keypair() {
        let local = tokio::task::LocalSet::new();
        local
            .run_until(async {
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
            })
            .await;
    }
}
