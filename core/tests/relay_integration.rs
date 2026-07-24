//! Integration tests for relay fallback functionality
//!
//! Tests the connection strategy with automatic fallback:
//! Direct Connection → Hole Punching → Relay

use libp2p::{identity, Multiaddr, PeerId};
use std::time::Duration;
use zaplivre_core::network::{
    connection::{ConnectionManager, ConnectionState, ConnectionStrategy, ConnectionType},
    nat_detection::{ConnectionStrategy as NatStrategy, NatDetector, NatType},
    relay::{RelayManager, ReservationStatus},
    retry::RetryPolicy,
    NetworkManager,
};

/// Test NetworkManager creation with relay configuration
#[tokio::test]
async fn test_network_manager_with_relay() {
    let keypair = identity::Keypair::generate_ed25519();
    let relay_peer = PeerId::random();
    let relay_addr: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();

    let manager = NetworkManager::with_relay(keypair, Some(relay_peer), Some(relay_addr));

    assert!(manager.is_ok());
    let manager = manager.unwrap();
    assert!(!manager.has_relay()); // Not reserved yet
}

/// Test connection manager creation and initial state
#[test]
fn test_connection_manager_creation() {
    let manager = ConnectionManager::new(RetryPolicy::default());
    let peer_id = PeerId::random();

    // Initially, peer should have no strategy
    assert!(manager.get_strategy(&peer_id).is_none());

    // Should not recommend relay for unknown peer
    assert!(!manager.should_try_relay(&peer_id));
}

/// Test connection strategy fallback after failures
#[test]
fn test_connection_strategy_fallback() {
    let mut manager = ConnectionManager::new(RetryPolicy::default());
    let peer_id = PeerId::random();

    // Initial state should be disconnected
    let state = manager.get_state(&peer_id);
    assert!(matches!(state, ConnectionState::Disconnected));

    // Record first failure - should start attempting direct
    manager.record_failure(peer_id);
    let state = manager.get_state(&peer_id);
    assert!(matches!(state, ConnectionState::AttemptingDirect { .. }));

    // Record 2nd and 3rd failure - should still be attempting direct
    manager.record_failure(peer_id);
    manager.record_failure(peer_id);

    // Record 4th failure - should now transition to hole punch
    manager.record_failure(peer_id);

    // Should now be attempting hole punch
    let state = manager.get_state(&peer_id);
    assert!(matches!(state, ConnectionState::AttemptingHolePunch { .. }));
}

/// Test connection success recording
#[test]
fn test_connection_success() {
    let mut manager = ConnectionManager::new(RetryPolicy::default());
    let peer_id = PeerId::random();

    // Record successful direct connection
    manager.record_success(peer_id, ConnectionType::Direct);

    let state = manager.get_state(&peer_id);
    assert!(matches!(
        state,
        ConnectionState::Connected(ConnectionType::Direct)
    ));
}

/// Test retry policy exponential backoff
#[test]
fn test_retry_policy_backoff() {
    let policy = RetryPolicy::default();

    // Test exponential backoff: 1s, 2s, 4s, 8s, 16s
    assert_eq!(policy.next_delay(0), Some(Duration::from_secs(1)));
    assert_eq!(policy.next_delay(1), Some(Duration::from_secs(2)));
    assert_eq!(policy.next_delay(2), Some(Duration::from_secs(4)));
    assert_eq!(policy.next_delay(3), Some(Duration::from_secs(8)));
    assert_eq!(policy.next_delay(4), Some(Duration::from_secs(16)));

    // After max attempts, should return None
    assert_eq!(policy.next_delay(5), None);
}

/// Test retry policy max delay capping
#[test]
fn test_retry_policy_max_delay() {
    let policy = RetryPolicy {
        max_attempts: 10,
        base_delay: Duration::from_secs(1),
        max_delay: Duration::from_secs(30),
    };

    // After enough attempts, delay should cap at max_delay
    let delay = policy.next_delay(6).unwrap(); // Would be 64s, but caps at 30s
    assert_eq!(delay, Duration::from_secs(30));
}

/// Test NAT detection type inference
#[test]
fn test_nat_detection() {
    let mut detector = NatDetector::new();

    // Initially unknown
    assert_eq!(detector.guess_nat_type(), NatType::Unknown);

    // Add same observed address multiple times → FullCone
    let addr1: Multiaddr = "/ip4/203.0.113.1/tcp/12345".parse().unwrap();
    detector.add_observed_address(addr1.clone());
    detector.add_observed_address(addr1.clone());

    let nat_type = detector.guess_nat_type();
    assert_eq!(nat_type, NatType::FullCone);

    // Different addresses → Symmetric
    let mut detector2 = NatDetector::new();
    let addr2: Multiaddr = "/ip4/203.0.113.1/tcp/12345".parse().unwrap();
    let addr3: Multiaddr = "/ip4/203.0.113.2/tcp/54321".parse().unwrap();
    detector2.add_observed_address(addr2);
    detector2.add_observed_address(addr3);

    let nat_type2 = detector2.guess_nat_type();
    assert_eq!(nat_type2, NatType::Symmetric);
}

/// Test NAT-based connection strategy recommendation
#[test]
fn test_nat_connection_strategy() {
    let mut detector = NatDetector::new();

    // Unknown NAT → try direct first
    assert_eq!(
        detector.connection_recommendation(),
        NatStrategy::DirectFirst
    );

    // FullCone NAT → try direct
    let addr: Multiaddr = "/ip4/203.0.113.1/tcp/12345".parse().unwrap();
    detector.add_observed_address(addr.clone());
    detector.add_observed_address(addr);
    assert_eq!(
        detector.connection_recommendation(),
        NatStrategy::DirectFirst
    );

    // Symmetric NAT → should use relay
    let mut detector2 = NatDetector::new();
    let addr1: Multiaddr = "/ip4/203.0.113.1/tcp/12345".parse().unwrap();
    let addr2: Multiaddr = "/ip4/203.0.113.2/tcp/54321".parse().unwrap();
    detector2.add_observed_address(addr1);
    detector2.add_observed_address(addr2);

    let strategy = detector2.connection_recommendation();
    assert!(
        matches!(strategy, NatStrategy::RelayFirst)
            || matches!(strategy, NatStrategy::HolePunchFirst)
    );
}

/// Test relay manager reservation lifecycle
#[test]
fn test_relay_manager_lifecycle() {
    let relay_peer = PeerId::random();
    let relay_addr: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();

    let mut manager = RelayManager::new(Some(relay_peer), Some(relay_addr));

    // Initially not reserved
    assert!(!manager.has_reservation());
    assert_eq!(manager.reservation_status, ReservationStatus::NotReserved);

    // Mark as pending
    manager.mark_reservation_pending();
    assert!(!manager.has_reservation());
    assert!(matches!(
        manager.reservation_status,
        ReservationStatus::Pending { .. }
    ));

    // Mark as reserved
    manager.mark_reservation_reserved(3600);
    assert!(manager.has_reservation());
    assert!(matches!(
        manager.reservation_status,
        ReservationStatus::Reserved { .. }
    ));

    // Mark as failed
    manager.mark_reservation_failed("Test error".to_string());
    assert!(!manager.has_reservation());
    assert!(matches!(
        manager.reservation_status,
        ReservationStatus::Failed { .. }
    ));
}

/// Test relay circuit address construction
#[test]
fn test_relay_circuit_addr() {
    let relay_peer = PeerId::random();
    let relay_addr: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();
    let target_peer = PeerId::random();

    let manager = RelayManager::new(Some(relay_peer), Some(relay_addr));

    // Should construct circuit address
    let circuit = manager.circuit_addr(&target_peer);
    assert!(circuit.is_some());

    let circuit_str = circuit.unwrap().to_string();
    assert!(circuit_str.contains("p2p-circuit"));
    assert!(circuit_str.contains(&relay_peer.to_string()));
    assert!(circuit_str.contains(&target_peer.to_string()));
}

/// Test relay circuit address when not configured
#[test]
fn test_relay_circuit_addr_not_configured() {
    let target_peer = PeerId::random();
    let manager = RelayManager::new(None, None);

    let circuit = manager.circuit_addr(&target_peer);
    assert!(circuit.is_none());
}

/// Test reservation expiry
#[test]
fn test_reservation_expiry() {
    let mut manager = RelayManager::new(None, None);

    // Not expired when not reserved
    assert!(!manager.is_reservation_expired());

    // Reserve with 0 seconds → should expire immediately
    manager.mark_reservation_reserved(0);
    std::thread::sleep(Duration::from_millis(10));
    assert!(manager.is_reservation_expired());

    // Reserve with long TTL → should not expire
    manager.mark_reservation_reserved(3600);
    assert!(!manager.is_reservation_expired());
}

/// Test connection type equality
#[test]
fn test_connection_type_equality() {
    assert_eq!(ConnectionType::Direct, ConnectionType::Direct);
    assert_eq!(ConnectionType::HolePunch, ConnectionType::HolePunch);
    assert_eq!(ConnectionType::Relayed, ConnectionType::Relayed);

    assert_ne!(ConnectionType::Direct, ConnectionType::Relayed);
    assert_ne!(ConnectionType::HolePunch, ConnectionType::Relayed);
    assert_ne!(ConnectionType::Direct, ConnectionType::HolePunch);
}

/// Test success rate calculation
#[test]
fn test_success_rate_calculation() {
    let peer_id = PeerId::random();
    let mut strategy = ConnectionStrategy::new(peer_id, RetryPolicy::default());

    // Initially 0% success rate (no attempts)
    assert_eq!(strategy.success_rate(&ConnectionType::Direct), 0.0);

    // Record some attempts manually
    strategy
        .attempts
        .push(zaplivre_core::network::connection::ConnectionAttempt {
            started_at: std::time::Instant::now(),
            duration: Duration::from_secs(1),
            success: true,
            connection_type: ConnectionType::Direct,
        });

    strategy
        .attempts
        .push(zaplivre_core::network::connection::ConnectionAttempt {
            started_at: std::time::Instant::now(),
            duration: Duration::from_secs(1),
            success: false,
            connection_type: ConnectionType::Direct,
        });

    // 1 success out of 2 attempts = 50%
    assert_eq!(strategy.success_rate(&ConnectionType::Direct), 0.5);

    // No relay attempts = 0%
    assert_eq!(strategy.success_rate(&ConnectionType::Relayed), 0.0);
}

/// Test that relay is recommended after sufficient failures
#[test]
fn test_should_try_relay_after_failures() {
    let peer_id = PeerId::random();
    let mut strategy = ConnectionStrategy::new(peer_id, RetryPolicy::default());

    // Initially should not try relay
    assert!(!strategy.should_try_relay());

    // Set state to attempting direct with 3+ attempts
    strategy.state = ConnectionState::AttemptingDirect {
        attempt: 3,
        started: std::time::Instant::now(),
    };

    // Should now recommend relay
    assert!(strategy.should_try_relay());
}

/// Test multiple connection strategies for different peers
#[test]
fn test_multiple_peer_strategies() {
    let mut manager = ConnectionManager::new(RetryPolicy::default());
    let peer1 = PeerId::random();
    let peer2 = PeerId::random();

    // Different peers should have independent strategies
    manager.record_failure(peer1);
    manager.record_failure(peer1);
    manager.record_failure(peer1);

    manager.record_success(peer2, ConnectionType::Direct);

    // Peer1 should be in fallback mode
    let state1 = manager.get_state(&peer1);
    assert!(!matches!(state1, ConnectionState::Disconnected));

    // Peer2 should be connected
    let state2 = manager.get_state(&peer2);
    assert!(matches!(
        state2,
        ConnectionState::Connected(ConnectionType::Direct)
    ));
}
