//! Protocol Module
//!
//! Defines message protocols and codecs for P2P communication.
//! Uses Protocol Buffers for efficient serialization.

pub mod codec;

// Generated protobuf code
#[allow(clippy::all)]
#[allow(warnings)]
pub mod pb {
    include!("generated/zaplivre.protocol.rs");
}

// Re-export common types
pub use pb::{
    AckMessage, AckStatus, EncryptedMessage, MediaChunk, MediaOffer, MediaRequest, Message,
    MessageType, ReadReceipt, TextMessage, TypingIndicator,
};
