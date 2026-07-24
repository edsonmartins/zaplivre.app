//! Relay client utilities
//!
//! Provides helpers for managing relay reservations and relay-based connections.

use libp2p::{Multiaddr, PeerId};
use std::time::Instant;

/// Relay manager handles relay reservations and relay-based connections
#[derive(Debug, Clone)]
pub struct RelayManager {
    /// Bootstrap relay peer ID
    pub bootstrap_relay_peer: Option<PeerId>,
    /// Bootstrap relay address
    pub relay_addr: Option<Multiaddr>,
    /// Reservation status
    pub reservation_status: ReservationStatus,
}

/// Reservation status
#[derive(Debug, Clone, PartialEq)]
pub enum ReservationStatus {
    /// No reservation
    NotReserved,
    /// Reservation request pending
    Pending { requested_at: Instant },
    /// Reservation active
    Reserved { expires_at: Instant },
    /// Reservation failed
    Failed { error: String },
}

impl RelayManager {
    /// Create a new relay manager
    pub fn new(bootstrap_relay_peer: Option<PeerId>, relay_addr: Option<Multiaddr>) -> Self {
        Self {
            bootstrap_relay_peer,
            relay_addr,
            reservation_status: ReservationStatus::NotReserved,
        }
    }

    /// Check if reservation is active
    pub fn has_reservation(&self) -> bool {
        matches!(self.reservation_status, ReservationStatus::Reserved { .. })
    }

    /// Mark reservation as pending
    pub fn mark_reservation_pending(&mut self) {
        self.reservation_status = ReservationStatus::Pending {
            requested_at: Instant::now(),
        };
    }

    /// Mark reservation as reserved
    pub fn mark_reservation_reserved(&mut self, ttl_seconds: u64) {
        let expires_at = Instant::now() + std::time::Duration::from_secs(ttl_seconds);
        self.reservation_status = ReservationStatus::Reserved { expires_at };
    }

    /// Mark reservation as failed
    pub fn mark_reservation_failed(&mut self, error: String) {
        self.reservation_status = ReservationStatus::Failed { error };
    }

    /// Get relay address for listening
    pub fn listen_addr(&self) -> Option<Multiaddr> {
        if self.has_reservation() {
            self.relay_addr.clone()
        } else {
            None
        }
    }

    /// Build relay circuit address for connecting to a peer
    ///
    /// Format: /ip4/relay-ip/tcp/relay-port/p2p/relay-peer-id/p2p-circuit/p2p/target-peer-id
    pub fn circuit_addr(&self, target_peer_id: &PeerId) -> Option<Multiaddr> {
        if let (Some(relay_addr), Some(relay_peer)) = (&self.relay_addr, &self.bootstrap_relay_peer)
        {
            // Build relay circuit address
            let circuit = relay_addr
                .clone()
                .with(libp2p::multiaddr::Protocol::P2p(*relay_peer))
                .with(libp2p::multiaddr::Protocol::P2pCircuit)
                .with(libp2p::multiaddr::Protocol::P2p(*target_peer_id));

            Some(circuit)
        } else {
            None
        }
    }

    /// Check if reservation has expired
    pub fn is_reservation_expired(&self) -> bool {
        if let ReservationStatus::Reserved { expires_at } = self.reservation_status {
            Instant::now() > expires_at
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relay_manager_creation() {
        let manager = RelayManager::new(None, None);
        assert_eq!(manager.reservation_status, ReservationStatus::NotReserved);
        assert!(!manager.has_reservation());
    }

    #[test]
    fn test_reservation_lifecycle() {
        let mut manager = RelayManager::new(None, None);

        // Start as not reserved
        assert!(!manager.has_reservation());

        // Mark pending
        manager.mark_reservation_pending();
        assert!(!manager.has_reservation());
        assert!(matches!(
            manager.reservation_status,
            ReservationStatus::Pending { .. }
        ));

        // Mark reserved
        manager.mark_reservation_reserved(3600);
        assert!(manager.has_reservation());
        assert!(matches!(
            manager.reservation_status,
            ReservationStatus::Reserved { .. }
        ));

        // Mark failed
        manager.mark_reservation_failed("Test error".to_string());
        assert!(!manager.has_reservation());
        assert!(matches!(
            manager.reservation_status,
            ReservationStatus::Failed { .. }
        ));
    }

    #[test]
    fn test_listen_addr() {
        let relay_addr: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();
        let mut manager = RelayManager::new(None, Some(relay_addr.clone()));

        // No reservation → no listen address
        assert_eq!(manager.listen_addr(), None);

        // With reservation → listen address available
        manager.mark_reservation_reserved(3600);
        assert_eq!(manager.listen_addr(), Some(relay_addr));
    }

    #[test]
    fn test_circuit_addr() {
        let relay_addr: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();
        let relay_peer = PeerId::random();
        let target_peer = PeerId::random();

        let manager = RelayManager::new(Some(relay_peer), Some(relay_addr));

        let circuit = manager.circuit_addr(&target_peer);
        assert!(circuit.is_some());

        let circuit_str = circuit.unwrap().to_string();
        assert!(circuit_str.contains("p2p-circuit"));
    }

    #[test]
    fn test_reservation_expiry() {
        let mut manager = RelayManager::new(None, None);

        // Not expired when not reserved
        assert!(!manager.is_reservation_expired());

        // Reserve with 0 seconds → immediately expired
        manager.mark_reservation_reserved(0);
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(manager.is_reservation_expired());
    }
}
