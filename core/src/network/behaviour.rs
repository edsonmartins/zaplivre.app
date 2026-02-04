//! Network Behaviour
//!
//! Custom libp2p NetworkBehaviour combining:
//! - Kademlia DHT (peer discovery and routing)
//! - mDNS (local network discovery)
//! - Identify (peer information exchange)
//! - Ping (keep-alive)
//! - GossipSub (will be used for group messaging)
//! - VoIP Signaling (WebRTC signaling over P2P)

use libp2p::{
    dcutr, gossipsub, identify, kad, mdns, ping, relay, request_response, PeerId, StreamProtocol,
};
use libp2p::swarm::NetworkBehaviour;
use std::time::Duration;

use super::messaging::MePassaCodec;
use crate::utils::error::MePassaError;
#[cfg(any(feature = "voip", feature = "video"))]
use crate::voip::signaling::SignalingCodec;

/// MePassa network behaviour
#[derive(NetworkBehaviour)]
pub struct MePassaBehaviour {
    /// Kademlia DHT for peer discovery and routing
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
    /// mDNS for local network discovery
    pub mdns: mdns::tokio::Behaviour,
    /// Identify protocol for peer information exchange
    pub identify: identify::Behaviour,
    /// Ping for connection keep-alive
    pub ping: ping::Behaviour,
    /// GossipSub for pub/sub messaging (groups)
    pub gossipsub: gossipsub::Behaviour,
    /// Request/Response for direct messaging
    pub request_response: request_response::Behaviour<MePassaCodec>,
    /// Relay client (v2) for reservations and relayed connections
    pub relay: relay::client::Behaviour,
    /// Request/Response for VoIP signaling (WebRTC)
    #[cfg(any(feature = "voip", feature = "video"))]
    pub voip_signaling: request_response::Behaviour<SignalingCodec>,
    /// DCUtR for hole punching (requires relay transport)
    pub dcutr: dcutr::Behaviour,
}

impl MePassaBehaviour {
    /// Create a new MePassa network behaviour
    pub fn new(
        local_peer_id: PeerId,
        keypair: &libp2p::identity::Keypair,
        relay: relay::client::Behaviour,
    ) -> crate::utils::error::Result<Self> {
        // Kademlia DHT configuration
        let mut kad_config = kad::Config::default();
        kad_config.set_query_timeout(Duration::from_secs(60));

        let store = kad::store::MemoryStore::new(local_peer_id);
        let kademlia = kad::Behaviour::with_config(local_peer_id, store, kad_config);

        // mDNS for local discovery (using tokio runtime)
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id).map_err(|e| {
            MePassaError::Network(format!("Failed to create mDNS behaviour: {}", e))
        })?;

        // Identify protocol
        let identify = identify::Behaviour::new(identify::Config::new(
            "/mepassa/1.0.0".to_string(),
            keypair.public(),
        ));

        // Ping for keep-alive
        let ping = ping::Behaviour::new(ping::Config::new().with_interval(Duration::from_secs(30)));

        // GossipSub for group messaging
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .message_id_fn(|message| {
                // Custom message ID based on content hash
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};

                let mut hasher = DefaultHasher::new();
                message.data.hash(&mut hasher);
                gossipsub::MessageId::from(hasher.finish().to_string())
            })
            .build()
            .map_err(|e| MePassaError::Network(format!("Failed to create GossipSub config: {}", e)))?;

        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config,
        )
        .map_err(|e| MePassaError::Network(format!("Failed to create GossipSub behaviour: {}", e)))?;

        // Request/Response for direct messaging
        let protocols = std::iter::once((
            StreamProtocol::new("/mepassa/message/1.0.0"),
            request_response::ProtocolSupport::Full,
        ));
        let request_response = request_response::Behaviour::with_codec(
            MePassaCodec,
            protocols,
            request_response::Config::default(),
        );

        // Request/Response for VoIP signaling (WebRTC)
        #[cfg(any(feature = "voip", feature = "video"))]
        let voip_protocols = std::iter::once((
            StreamProtocol::new("/mepassa/voip/1.0.0"),
            request_response::ProtocolSupport::Full,
        ));
        #[cfg(any(feature = "voip", feature = "video"))]
        let voip_signaling = request_response::Behaviour::with_codec(
            SignalingCodec,
            voip_protocols,
            request_response::Config::default(),
        );

        // DCUtR for hole punching
        // Note: Relay functionality is integrated at transport level in libp2p 0.53
        let dcutr = dcutr::Behaviour::new(local_peer_id);

        Ok(Self {
            kademlia,
            mdns,
            identify,
            ping,
            gossipsub,
            request_response,
            relay,
            #[cfg(any(feature = "voip", feature = "video"))]
            voip_signaling,
            dcutr,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity;

    #[test]
    fn test_create_behaviour() {
        let keypair = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(keypair.public());

        let (_relay_transport, relay) = relay::client::new(local_peer_id);
        let behaviour = MePassaBehaviour::new(local_peer_id, &keypair, relay);

        assert!(behaviour.is_ok());
    }

    #[test]
    fn test_multiple_behaviours() {
        let keypair1 = identity::Keypair::generate_ed25519();
        let keypair2 = identity::Keypair::generate_ed25519();

        let peer1 = PeerId::from(keypair1.public());
        let peer2 = PeerId::from(keypair2.public());

        let (_relay_transport1, relay1) = relay::client::new(peer1);
        let (_relay_transport2, relay2) = relay::client::new(peer2);
        let behaviour1 = MePassaBehaviour::new(peer1, &keypair1, relay1);
        let behaviour2 = MePassaBehaviour::new(peer2, &keypair2, relay2);

        assert!(behaviour1.is_ok());
        assert!(behaviour2.is_ok());
    }
}
