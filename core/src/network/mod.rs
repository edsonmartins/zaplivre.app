//! Networking module
//!
//! Implements P2P networking using libp2p (Kademlia DHT, GossipSub, Relay).

pub mod behaviour;
pub mod connection;
pub mod message_handler;
pub mod messaging;
pub mod nat_detection;
pub mod relay;
pub mod retry;
pub mod swarm;
pub mod transport;
// pub mod dht;
// pub mod gossip;

pub use behaviour::ZapLivreBehaviour;
pub use connection::{ConnectionManager, ConnectionState, ConnectionStrategy, ConnectionType};
pub use message_handler::{MessageEvent, MessageHandler};
pub use messaging::ZapLivreCodec;
pub use nat_detection::{ConnectionStrategy as NatConnectionStrategy, NatDetector, NatType};
pub use relay::{RelayManager, ReservationStatus};
pub use retry::RetryPolicy;
pub use swarm::NetworkManager;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Peer not found: {0}")]
    PeerNotFound(String),

    #[error("NAT traversal failed")]
    NatTraversalFailed,

    #[error("Timeout")]
    Timeout,
}

impl From<NetworkError> for crate::utils::error::ZapLivreError {
    fn from(err: NetworkError) -> Self {
        crate::utils::error::ZapLivreError::Network(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, NetworkError>;
