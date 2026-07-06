use libp2p::{
    dcutr, identify, kad, ping, relay,
    swarm::NetworkBehaviour,
    PeerId,
};
use std::time::Duration;

use crate::config::Config;

/// Custom NetworkBehaviour for the Bootstrap Node
///
/// Combines Kademlia DHT for peer discovery, Circuit Relay v2 for NAT traversal,
/// DCUtR for hole punching, Identify and Ping protocols.
#[derive(NetworkBehaviour)]
pub struct BootstrapBehaviour {
    /// Kademlia DHT for peer discovery
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,

    /// Identify protocol for peer information exchange
    pub identify: identify::Behaviour,

    /// Ping protocol for keep-alive
    pub ping: ping::Behaviour,

    /// Circuit Relay v2 server for NAT traversal
    pub relay: relay::Behaviour,

    /// DCUtR for direct connection upgrade through relay
    pub dcutr: dcutr::Behaviour,
}

impl BootstrapBehaviour {
    /// Create a new BootstrapBehaviour instance
    pub fn new(
        local_peer_id: PeerId,
        local_public_key: libp2p::identity::PublicKey,
        config: &Config,
    ) -> Self {
        // Kademlia DHT configuration
        let mut kad_config = kad::Config::default();
        kad_config.set_query_timeout(Duration::from_secs(60));
        kad_config.set_replication_factor(20.try_into().unwrap());

        let store = kad::store::MemoryStore::new(local_peer_id);
        let kademlia = kad::Behaviour::with_config(local_peer_id, store, kad_config);

        // Identify configuration
        let identify = identify::Behaviour::new(
            identify::Config::new(
                "/zaplivre/1.0.0".to_string(),
                local_public_key,
            )
        );

        // Ping (keep-alive)
        let ping = ping::Behaviour::new(ping::Config::new());

        // Relay server configuration
        let relay = if config.relay_enabled {
            tracing::info!(
                "🔗 Relay server enabled (max {} circuits, {} per peer, {} bytes/s)",
                config.relay_max_circuits,
                config.relay_max_per_peer,
                config.relay_max_bytes_per_second
            );

            relay::Behaviour::new(
                local_peer_id,
                relay::Config {
                    max_reservations: config.relay_max_circuits,
                    max_reservations_per_peer: config.relay_max_per_peer,
                    reservation_duration: Duration::from_secs(3600), // 1 hour
                    max_circuits: config.relay_max_circuits,
                    max_circuits_per_peer: config.relay_max_per_peer,
                    max_circuit_duration: Duration::from_secs(120), // 2 minutes
                    max_circuit_bytes: config.relay_max_bytes_per_second,
                    ..Default::default()
                },
            )
        } else {
            tracing::info!("❌ Relay server disabled");
            relay::Behaviour::new(local_peer_id, relay::Config::default())
        };

        // DCUtR for hole punching coordination
        let dcutr = dcutr::Behaviour::new(local_peer_id);

        Self {
            kademlia,
            identify,
            ping,
            relay,
            dcutr,
        }
    }
}
