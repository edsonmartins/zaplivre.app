//! Event System
//!
//! Events emitted by the ZapLivre client.

use libp2p::PeerId;

use crate::protocol::Message;

/// Events that can be emitted by the ZapLivre client
#[derive(Debug, Clone)]
pub enum ClientEvent {
    /// A new message was received
    MessageReceived {
        message_id: String,
        from: PeerId,
        message: Message,
    },

    /// A message was successfully sent
    MessageSent {
        message_id: String,
        to: PeerId,
    },

    /// A message delivery confirmation (ACK) was received
    MessageDelivered {
        message_id: String,
        to: PeerId,
    },

    /// A message was read by the recipient
    MessageRead {
        message_id: String,
        by: PeerId,
        read_at: i64,
    },

    /// A peer started typing
    TypingStarted {
        peer_id: PeerId,
    },

    /// A peer stopped typing
    TypingStopped {
        peer_id: PeerId,
    },

    /// Connected to a peer
    PeerConnected {
        peer_id: PeerId,
    },

    /// Disconnected from a peer
    PeerDisconnected {
        peer_id: PeerId,
    },

    /// A peer was discovered via mDNS
    PeerDiscovered {
        peer_id: PeerId,
        addresses: Vec<String>,
    },

    /// Connection to network established
    NetworkOnline,

    /// Connection to network lost
    NetworkOffline,

    /// An error occurred
    Error {
        error: String,
    },
}

/// Callback trait for handling events
pub trait EventCallback: Send + Sync {
    fn on_event(&self, event: ClientEvent);
}

/// Simple function-based callback
pub struct FunctionCallback<F>
where
    F: Fn(ClientEvent) + Send + Sync,
{
    func: F,
}

impl<F> FunctionCallback<F>
where
    F: Fn(ClientEvent) + Send + Sync,
{
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

impl<F> EventCallback for FunctionCallback<F>
where
    F: Fn(ClientEvent) + Send + Sync,
{
    fn on_event(&self, event: ClientEvent) {
        (self.func)(event)
    }
}
