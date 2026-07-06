//! Basic P2P Connectivity Example
//!
//! This example demonstrates how to create two peers and have them discover each other.
//! It shows the basic setup of the networking stack: transport, behaviour, and swarm.
//!
//! Note: This is a demonstration of the networking infrastructure. Full message
//! exchange requires protocol definitions (FASE 4) and will be added later.
//!
//! Run with: `cargo run --example basic_p2p`

use libp2p::identity::Keypair;
use zaplivre_core::network::NetworkManager;
use std::error::Error;
use tokio::time::{sleep, Duration};
use tracing::{info, warn, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Setup logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("🚀 Starting ZapLivre P2P Connectivity Example");

    // Create two peers with different keypairs
    let keypair1 = Keypair::generate_ed25519();
    let keypair2 = Keypair::generate_ed25519();

    let mut peer1 = NetworkManager::new(keypair1)?;
    let mut peer2 = NetworkManager::new(keypair2)?;

    let peer1_id = peer1.local_peer_id().clone();
    let peer2_id = peer2.local_peer_id().clone();

    info!("👤 Peer 1 ID: {}", peer1_id);
    info!("👤 Peer 2 ID: {}", peer2_id);

    // Start listening on different ports
    use libp2p::Multiaddr;
    let addr1: Multiaddr = "/ip4/127.0.0.1/tcp/9001".parse()?;
    let addr2: Multiaddr = "/ip4/127.0.0.1/tcp/9002".parse()?;

    peer1.listen_on(addr1.clone())?;
    peer2.listen_on(addr2.clone())?;

    info!("👂 Peer 1 listening on {}", addr1);
    info!("👂 Peer 2 listening on {}", addr2);

    // Add peer2 to peer1's DHT
    peer1.add_peer_to_dht(peer2_id.clone(), addr2.clone());
    info!("📋 Added Peer 2 to Peer 1's DHT");

    // Have peer1 dial peer2
    peer1.dial(peer2_id.clone(), addr2)?;
    info!("📞 Peer 1 dialing Peer 2...");

    // Give time for connection to establish
    sleep(Duration::from_secs(2)).await;

    info!("🔗 Peer 1 connected peers: {}", peer1.connected_peers());
    info!("🔗 Peer 2 connected peers: {}", peer2.connected_peers());

    // Bootstrap DHT
    peer1.bootstrap()?;
    peer2.bootstrap()?;
    info!("🔍 DHT bootstrap initiated");

    info!("✅ Example setup completed successfully!");
    info!("📊 Network infrastructure ready:");
    info!("   Peer 1 ID: {}", peer1_id);
    info!("   Peer 2 ID: {}", peer2_id);
    info!("   Initial connections: {} (Peer 1), {} (Peer 2)",
          peer1.connected_peers(),
          peer2.connected_peers());

    warn!("⚠️  Note: To actually run the event loop and exchange messages,");
    warn!("   you would call peer1.run() and peer2.run() in separate tasks.");
    warn!("   Full message exchange requires protocol definitions (FASE 4).");

    info!("🎯 This example demonstrates:");
    info!("   ✅ Transport layer setup (TCP + QUIC)");
    info!("   ✅ Network behaviour configuration (Kademlia, mDNS, etc)");
    info!("   ✅ Swarm management (listen, dial, DHT)");
    info!("   ⏳ Message protocol (pending FASE 4)");

    Ok(())
}
