use anyhow::Result;
use futures::stream::StreamExt;
use libp2p::{
    core::upgrade,
    identity::Keypair,
    noise, tcp, yamux,
    swarm::{Config as SwarmConfig, Swarm, SwarmEvent},
    Multiaddr, PeerId, Transport,
};
use std::time::Duration;
use tracing::{info, warn};

mod config;
mod behaviour;
mod storage;
mod health;

use config::Config;
use behaviour::BootstrapBehaviour;
use storage::DhtStorage;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Load configuration
    let config = Config::from_env()?;
    config.validate()?;

    // 2. Setup logging
    tracing_subscriber::fmt()
        .with_env_filter(&config.log_level)
        .init();

    info!("🚀 MePassa Bootstrap Node starting...");
    info!("   P2P Port: {}", config.p2p_port);
    info!("   Health Port: {}", config.health_port);
    info!("   Data Dir: {:?}", config.data_dir);

    // 3. Generate deterministic keypair
    let keypair = load_or_generate_keypair(&config)?;
    let local_peer_id = PeerId::from(keypair.public());
    let local_public_key = keypair.public();
    info!("   Peer ID: {}", local_peer_id);

    // 4. Build transport (TCP + Noise + Yamux)
    let transport = tcp::tokio::Transport::new(tcp::Config::default())
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::Config::new(&keypair)?)
        .multiplex(yamux::Config::default())
        .boxed();

    // 5. Create swarm
    let behaviour = BootstrapBehaviour::new(local_peer_id, local_public_key, &config);
    let mut swarm = Swarm::new(
        transport,
        behaviour,
        local_peer_id,
        SwarmConfig::with_tokio_executor()
            .with_idle_connection_timeout(Duration::from_secs(60)),
    );

    // 6. Initialize persistent storage
    let db_path = config.data_dir.join("dht.db");
    let storage = DhtStorage::new(db_path).await?;

    // Load previously known peers from storage
    let stored_peers = storage.load_peers().await?;
    for (peer_id, addrs) in stored_peers {
        for addr in addrs {
            swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
        }
    }

    // Cleanup stale peers (older than 7 days)
    storage.cleanup_stale(7 * 24 * 60 * 60).await?;

    // 7. Listen on configured port
    let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", config.p2p_port)
        .parse()?;
    swarm.listen_on(listen_addr.clone())?;
    info!("   Listening on: {}", listen_addr);

    // 8. Start health check server
    let peer_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let health_server = health::start_server(config.health_port, peer_count.clone());
    tokio::spawn(health_server);

    info!("✅ Bootstrap node ready!");

    // 9. Event loop
    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("📡 Listening on: {}", address);
            }

            SwarmEvent::Behaviour(event) => {
                handle_behaviour_event(event, &mut swarm, &storage, &peer_count).await;
            }

            SwarmEvent::IncomingConnection { local_addr, send_back_addr, connection_id } => {
                info!("📥 Incoming connection from {} to {} (id: {:?})", send_back_addr, local_addr, connection_id);
            }

            SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                info!("✅ Connection established with {}", peer_id);

                // Add to DHT
                let addr = endpoint.get_remote_address();
                swarm.behaviour_mut()
                    .kademlia
                    .add_address(&peer_id, addr.clone());

                // Save to persistent storage
                if let Err(e) = storage.add_peer(&peer_id, addr).await {
                    warn!("Failed to save peer to storage: {}", e);
                }

                // Update peer count
                peer_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }

            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                warn!("❌ Connection closed with {}: {:?}", peer_id, cause);
                peer_count.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            }

            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                if let Some(peer) = peer_id {
                    warn!("❌ Outgoing connection error to {}: {}", peer, error);
                } else {
                    warn!("❌ Outgoing connection error: {}", error);
                }
            }

            SwarmEvent::IncomingConnectionError { local_addr, send_back_addr, error, connection_id } => {
                warn!("❌ Incoming connection error from {} to {} (id: {:?}): {}",
                    send_back_addr, local_addr, connection_id, error);
            }

            _ => {}
        }
    }
}

async fn handle_behaviour_event(
    event: behaviour::BootstrapBehaviourEvent,
    swarm: &mut libp2p::Swarm<BootstrapBehaviour>,
    storage: &DhtStorage,
    _peer_count: &std::sync::Arc<std::sync::atomic::AtomicUsize>,
) {
    match event {
        // Kademlia events
        behaviour::BootstrapBehaviourEvent::Kademlia(kad_event) => {
            match kad_event {
                libp2p::kad::Event::RoutingUpdated { peer, .. } => {
                    info!("🔄 DHT routing updated for {}", peer);
                }
                libp2p::kad::Event::InboundRequest { request } => {
                    info!("📨 Inbound DHT request: {:?}", request);
                }
                libp2p::kad::Event::OutboundQueryProgressed { result, .. } => {
                    match result {
                        libp2p::kad::QueryResult::GetProviders(Ok(_ok)) => {
                            info!("📦 GetProviders successful");
                        }
                        libp2p::kad::QueryResult::GetProviders(Err(err)) => {
                            warn!("📦 GetProviders failed: {:?}", err);
                        }
                        libp2p::kad::QueryResult::GetRecord(Ok(_ok)) => {
                            info!("📝 GetRecord successful");
                        }
                        libp2p::kad::QueryResult::GetRecord(Err(err)) => {
                            warn!("📝 GetRecord failed: {:?}", err);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        // Identify events
        behaviour::BootstrapBehaviourEvent::Identify(identify_event) => {
            if let libp2p::identify::Event::Received { peer_id, info } = identify_event {
                info!("🆔 Identified peer {}: agent={}, protocols={:?}",
                    peer_id, info.agent_version, info.protocols);

                // Add addresses to DHT and storage
                for addr in info.listen_addrs {
                    swarm.behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, addr.clone());

                    // Save to persistent storage
                    if let Err(e) = storage.add_peer(&peer_id, &addr).await {
                        warn!("Failed to save peer address to storage: {}", e);
                    }
                }
            }
        }

        // Ping events
        behaviour::BootstrapBehaviourEvent::Ping(ping_event) => {
            match ping_event.result {
                Ok(rtt) => {
                    tracing::debug!("🏓 Ping to {} successful: {:?}", ping_event.peer, rtt);
                }
                Err(e) => {
                    warn!("🏓 Ping to {} failed: {}", ping_event.peer, e);
                }
            }
        }

        // Relay events
        behaviour::BootstrapBehaviourEvent::Relay(relay_event) => {
            match relay_event {
                libp2p::relay::Event::ReservationReqAccepted { src_peer_id, renewed } => {
                    if renewed {
                        info!("🔗 Relay reservation renewed for {}", src_peer_id);
                    } else {
                        info!("🔗 Relay reservation accepted for {}", src_peer_id);
                    }
                }
                libp2p::relay::Event::ReservationReqDenied { src_peer_id } => {
                    warn!("⛔ Relay reservation denied for {}", src_peer_id);
                }
                libp2p::relay::Event::ReservationTimedOut { src_peer_id } => {
                    info!("⏱️ Relay reservation timed out for {}", src_peer_id);
                }
                libp2p::relay::Event::CircuitReqDenied { src_peer_id, dst_peer_id } => {
                    warn!("⛔ Circuit denied: {} → {}", src_peer_id, dst_peer_id);
                }
                libp2p::relay::Event::CircuitReqAccepted { src_peer_id, dst_peer_id } => {
                    info!("🌉 Circuit created: {} ↔ {}", src_peer_id, dst_peer_id);
                }
                libp2p::relay::Event::CircuitClosed { src_peer_id, dst_peer_id, error } => {
                    if let Some(err) = error {
                        warn!("🔌 Circuit closed: {} ↔ {} (error: {})", src_peer_id, dst_peer_id, err);
                    } else {
                        info!("🔌 Circuit closed: {} ↔ {}", src_peer_id, dst_peer_id);
                    }
                }
                _ => {}
            }
        }

        // DCUtR events
        behaviour::BootstrapBehaviourEvent::Dcutr(dcutr_event) => {
            // DCUtR events are logged at debug level
            tracing::debug!("🎯 DCUtR event: {:?}", dcutr_event);
        }
    }
}

/// Load the node keypair, preferindo uma chave ALEATÓRIA persistida em disco
/// (SEC-12). A derivação por seed (SHA256 do PEER_ID_SEED) continua suportada
/// para compatibilidade, mas é insegura quando a seed é pública (qualquer um
/// derivava a chave privada do bootstrap - ex.: seed "bootstrap-1" no compose).
fn load_or_generate_keypair(config: &config::Config) -> Result<Keypair> {
    use libp2p::identity::ed25519;

    let key_path = config.data_dir.join("node_key");

    // 1) Chave persistida tem prioridade (aleatória, gerada na primeira execução)
    if key_path.exists() {
        let mut bytes = std::fs::read(&key_path)?;
        let secret_key = ed25519::SecretKey::try_from_bytes(&mut bytes)?;
        info!("🔐 Loaded persisted node key from {:?}", key_path);
        return Ok(Keypair::from(ed25519::Keypair::from(secret_key)));
    }

    // 2) Compatibilidade: PEER_ID_SEED explícita mantém a identidade de
    //    deploys existentes (os peer IDs estão hardcoded nos clients).
    //    Para migrar para chave aleatória segura, remova PEER_ID_SEED do env.
    if let Ok(seed) = std::env::var("PEER_ID_SEED") {
        if !seed.trim().is_empty() {
            tracing::warn!(
                "⚠️ INSECURE: node key derived from PEER_ID_SEED - anyone knowing \
                 the seed can impersonate this bootstrap node. Remove PEER_ID_SEED \
                 from the environment to switch to a random persisted key."
            );
            return generate_keypair_from_seed(&seed);
        }
    }

    // 3) Gerar chave aleatória e persistir
    let keypair = ed25519::Keypair::generate();
    std::fs::write(&key_path, keypair.secret().as_ref())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600));
    }
    info!("🔐 Generated new random node key, persisted to {:?}", key_path);
    Ok(Keypair::from(keypair))
}

/// Generate deterministic keypair from seed string (INSEGURO com seed pública)
fn generate_keypair_from_seed(seed: &str) -> Result<Keypair> {
    use libp2p::identity::ed25519;
    use sha2::{Sha256, Digest};

    // Hash seed to get 32 bytes
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let hash = hasher.finalize();

    // Create Ed25519 keypair from hash (convert to mutable array)
    let mut secret_bytes: [u8; 32] = hash.into();
    let secret_key = ed25519::SecretKey::try_from_bytes(&mut secret_bytes)?;
    let keypair = ed25519::Keypair::from(secret_key);

    Ok(Keypair::from(keypair))
}
