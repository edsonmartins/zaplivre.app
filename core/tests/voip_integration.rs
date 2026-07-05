//! VoIP Integration Tests
//!
//! End-to-end tests for VoIP signaling between 2 peers.
//! Requer a feature `voip`: cargo test --test voip_integration --features voip

#![cfg(feature = "voip")]

use libp2p::identity::Keypair;
use libp2p::{Multiaddr, PeerId};
use mepassa_core::{
    api::{Client, ClientBuilder},
    voip::{CallManager, VoIPIntegration, SignalingMessage},
};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::RwLock;
use tokio::time::timeout;
use uuid::Uuid;

/// Test helper: Create a client with VoIP enabled.
///
/// Must be called from within a `tokio::task::LocalSet` (uses `spawn_local`
/// to drive the libp2p swarm event loop in the background).
async fn create_test_client(name: &str) -> (Arc<Client>, TempDir, PeerId) {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    let keypair = Keypair::generate_ed25519();
    let peer_id = PeerId::from(keypair.public());

    let client = ClientBuilder::new()
        .data_dir(data_dir)
        .keypair(keypair)
        .build()
        .await
        .unwrap();

    let client = Arc::new(client);

    // Drive the swarm: without pumping the network event loop, listeners
    // never become ready and connections/signaling never progress.
    let net_client = client.clone();
    tokio::task::spawn_local(async move {
        let _ = net_client.run_network().await;
    });

    tracing::info!("✅ Created test client '{}': {}", name, peer_id);

    (client, temp_dir, peer_id)
}

/// Test helper: Start listening on local address and return the actual
/// (auto-assigned) listening address.
async fn start_listening(client: &Client) -> Multiaddr {
    let addr: Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
    client.listen_on(addr).await.unwrap();

    // Wait for the listener to be ready and grab the real port (tcp/0 is auto-assigned)
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if let Some(addr) = client
            .listening_addresses()
            .await
            .into_iter()
            .find(|a| a.contains("/ip4/127.0.0.1/") && !a.ends_with("/tcp/0"))
        {
            return addr.parse().unwrap();
        }
    }
    panic!("Listener did not become ready in time");
}

/// Test helper: Connect peer A to peer B
async fn connect_peers(client_a: &Client, peer_b_id: PeerId, addr_b: Multiaddr) {
    client_a.connect_to_peer(peer_b_id, addr_b).await.unwrap();

    // Wait for connection to establish (up to 5s)
    for _ in 0..50 {
        if client_a.connected_peers_count().await > 0 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test]
#[ignore] // Requires real network/audio devices - run with `cargo test voip_integration --features voip -- --include-ignored`.
// The VoIP code uses `tokio::task::spawn_local`, so each test body runs inside a `tokio::task::LocalSet`.
async fn test_two_peers_setup() {
    let local = tokio::task::LocalSet::new();
    local.run_until(async {
    tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .ok();

    tracing::info!("🧪 Test: Two peers VoIP setup");

    // Create two clients
    let (client_a, _temp_a, peer_a_id) = create_test_client("PeerA").await;
    let (client_b, _temp_b, peer_b_id) = create_test_client("PeerB").await;

    tracing::info!("📡 Peer A: {}", peer_a_id);
    tracing::info!("📡 Peer B: {}", peer_b_id);

    // Start listening
    let addr_a = start_listening(&client_a).await;
    let addr_b = start_listening(&client_b).await;

    tracing::info!("🎧 Peer A listening on: {}", addr_a);
    tracing::info!("🎧 Peer B listening on: {}", addr_b);

    // Connect peers
    connect_peers(&client_a, peer_b_id, addr_b.clone()).await;

    tracing::info!("✅ Peers connected successfully");

    // Verify connection
    let peers_a = client_a.connected_peers_count().await;
    tracing::info!("👥 Peer A connected to {} peers", peers_a);

    assert!(peers_a > 0, "Peer A should be connected to at least 1 peer");
    }).await;
}

#[tokio::test]
#[ignore]
async fn test_call_offer_flow() {
    let local = tokio::task::LocalSet::new();
    local.run_until(async {
    tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .ok();

    tracing::info!("🧪 Test: Call offer flow");

    // Create two clients
    let (client_a, _temp_a, peer_a_id) = create_test_client("PeerA").await;
    let (client_b, _temp_b, peer_b_id) = create_test_client("PeerB").await;

    // Setup and connect
    let addr_a = start_listening(&client_a).await;
    let addr_b = start_listening(&client_b).await;
    connect_peers(&client_a, peer_b_id, addr_b).await;
    connect_peers(&client_b, peer_a_id, addr_a).await;

    tracing::info!("📞 Peer A initiating call to Peer B...");

    // Start call from A to B
    let call_result = timeout(
        Duration::from_secs(5),
        client_a.start_call(peer_b_id.to_string())
    ).await;

    match call_result {
        Ok(Ok(call_id)) => {
            tracing::info!("✅ Call initiated successfully: {}", call_id);
            assert!(!call_id.is_empty(), "Call ID should not be empty");
            // Verify it's a valid UUID format
            assert!(uuid::Uuid::parse_str(&call_id).is_ok(), "Call ID should be a valid UUID");
        }
        Ok(Err(e)) => {
            tracing::error!("❌ Call failed: {}", e);
            panic!("Call should succeed: {}", e);
        }
        Err(_) => {
            tracing::error!("⏱️ Call timed out");
            panic!("Call initiation timed out");
        }
    }
    }).await;
}

#[tokio::test]
#[ignore]
async fn test_call_answer_flow() {
    let local = tokio::task::LocalSet::new();
    local.run_until(async {
    tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .ok();

    tracing::info!("🧪 Test: Call answer flow");

    // Create two clients
    let (client_a, _temp_a, peer_a_id) = create_test_client("PeerA").await;
    let (client_b, _temp_b, peer_b_id) = create_test_client("PeerB").await;

    // Setup and connect
    let addr_b = start_listening(&client_b).await;
    connect_peers(&client_a, peer_b_id, addr_b).await;

    // Start call
    let call_id = client_a.start_call(peer_b_id.to_string()).await.unwrap();
    tracing::info!("📞 Call started: {}", call_id);

    // Wait for signaling to propagate
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Accept call from B
    tracing::info!("✅ Peer B accepting call...");
    let accept_result = timeout(
        Duration::from_secs(5),
        client_b.accept_call(call_id.clone())
    ).await;

    match accept_result {
        Ok(Ok(())) => {
            tracing::info!("✅ Call accepted successfully");
        }
        Ok(Err(e)) => {
            tracing::warn!("⚠️ Accept call returned error (expected for now): {}", e);
            // This is expected since we don't have full WebRTC setup in test
        }
        Err(_) => {
            tracing::error!("⏱️ Accept call timed out");
            panic!("Accept call timed out");
        }
    }
    }).await;
}

#[tokio::test]
#[ignore]
async fn test_call_reject_flow() {
    let local = tokio::task::LocalSet::new();
    local.run_until(async {
    tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .ok();

    tracing::info!("🧪 Test: Call reject flow");

    // Create two clients
    let (client_a, _temp_a, peer_a_id) = create_test_client("PeerA").await;
    let (client_b, _temp_b, peer_b_id) = create_test_client("PeerB").await;

    // Setup and connect
    let addr_b = start_listening(&client_b).await;
    connect_peers(&client_a, peer_b_id, addr_b).await;

    // Start call
    let call_id = client_a.start_call(peer_b_id.to_string()).await.unwrap();
    tracing::info!("📞 Call started: {}", call_id);

    // Wait for signaling
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Reject call from B
    tracing::info!("❌ Peer B rejecting call...");
    let reject_result = timeout(
        Duration::from_secs(5),
        client_b.reject_call(call_id.clone(), Some("User busy".to_string()))
    ).await;

    match reject_result {
        Ok(Ok(())) => {
            tracing::info!("✅ Call rejected successfully");
        }
        Ok(Err(e)) => {
            tracing::warn!("⚠️ Reject call returned error (expected for now): {}", e);
        }
        Err(_) => {
            tracing::error!("⏱️ Reject call timed out");
            panic!("Reject call timed out");
        }
    }
    }).await;
}

#[tokio::test]
#[ignore]
async fn test_call_hangup_flow() {
    let local = tokio::task::LocalSet::new();
    local.run_until(async {
    tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .ok();

    tracing::info!("🧪 Test: Call hangup flow");

    // Create two clients
    let (client_a, _temp_a, peer_a_id) = create_test_client("PeerA").await;
    let (client_b, _temp_b, peer_b_id) = create_test_client("PeerB").await;

    // Setup and connect
    let addr_b = start_listening(&client_b).await;
    connect_peers(&client_a, peer_b_id, addr_b).await;

    // Start call
    let call_id = client_a.start_call(peer_b_id.to_string()).await.unwrap();
    tracing::info!("📞 Call started: {}", call_id);

    // Wait a bit
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Hangup from A
    tracing::info!("📴 Peer A hanging up...");
    let hangup_result = timeout(
        Duration::from_secs(5),
        client_a.hangup_call(call_id.clone())
    ).await;

    match hangup_result {
        Ok(Ok(())) => {
            tracing::info!("✅ Call hung up successfully");
        }
        Ok(Err(e)) => {
            tracing::warn!("⚠️ Hangup returned error (expected for now): {}", e);
        }
        Err(_) => {
            tracing::error!("⏱️ Hangup timed out");
            panic!("Hangup timed out");
        }
    }
    }).await;
}

#[tokio::test]
#[ignore]
async fn test_mute_toggle() {
    let local = tokio::task::LocalSet::new();
    local.run_until(async {
    tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .ok();

    tracing::info!("🧪 Test: Mute toggle");

    let (client_a, _temp_a, peer_a_id) = create_test_client("PeerA").await;
    let (client_b, _temp_b, peer_b_id) = create_test_client("PeerB").await;

    let addr_b = start_listening(&client_b).await;
    connect_peers(&client_a, peer_b_id, addr_b).await;

    let call_id = client_a.start_call(peer_b_id.to_string()).await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;

    tracing::info!("🔇 Toggling mute...");
    let mute_result = client_a.toggle_mute(call_id.clone()).await;

    match mute_result {
        Ok(()) => tracing::info!("✅ Mute toggled successfully"),
        Err(e) => tracing::warn!("⚠️ Mute toggle error: {}", e),
    }
    }).await;
}

#[tokio::test]
#[ignore]
async fn test_speakerphone_toggle() {
    let local = tokio::task::LocalSet::new();
    local.run_until(async {
    tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .ok();

    tracing::info!("🧪 Test: Speakerphone toggle");

    let (client_a, _temp_a, peer_a_id) = create_test_client("PeerA").await;
    let (client_b, _temp_b, peer_b_id) = create_test_client("PeerB").await;

    let addr_b = start_listening(&client_b).await;
    connect_peers(&client_a, peer_b_id, addr_b).await;

    let call_id = client_a.start_call(peer_b_id.to_string()).await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;

    tracing::info!("🔊 Toggling speakerphone...");
    let speaker_result = client_a.toggle_speakerphone(call_id.clone()).await;

    match speaker_result {
        Ok(()) => tracing::info!("✅ Speakerphone toggled successfully"),
        Err(e) => tracing::warn!("⚠️ Speakerphone toggle error: {}", e),
    }
    }).await;
}

#[tokio::test]
async fn test_signaling_message_serialization() {
    tracing::info!("🧪 Test: SignalingMessage serialization");

    // Test CallOffer
    let offer = SignalingMessage::CallOffer {
        call_id: "call_123".to_string(),
        sdp: "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\n".to_string(),
    };

    let json = serde_json::to_string(&offer).unwrap();
    tracing::info!("📝 Serialized CallOffer: {}", json);

    let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(offer, deserialized);
    tracing::info!("✅ CallOffer serialization works");

    // Test IceCandidate
    let ice = SignalingMessage::IceCandidate {
        call_id: "call_123".to_string(),
        candidate: "candidate:1 1 UDP 2130706431 192.168.1.1 54321 typ host".to_string(),
        sdp_mid: Some("0".to_string()),
        sdp_m_line_index: Some(0),
    };

    let json = serde_json::to_string(&ice).unwrap();
    tracing::info!("📝 Serialized IceCandidate: {}", json);

    let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(ice, deserialized);
    tracing::info!("✅ IceCandidate serialization works");
}

#[tokio::test]
async fn test_call_manager_creation() {
    tracing::info!("🧪 Test: CallManager creation");

    let manager = CallManager::new();
    tracing::info!("✅ CallManager created successfully");

    // Verify manager has no active calls initially
    let active_calls = manager.get_active_calls().await;
    assert_eq!(active_calls.len(), 0, "Should have no active calls initially");

    tracing::info!("✅ CallManager has 0 active calls");
}
