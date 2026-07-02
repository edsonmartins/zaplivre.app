//! Group Chat Module
//!
//! Group messaging functionality using GossipSub (libp2p pub/sub).
//!
//! Features:
//! - Create/Join/Leave groups (up to 256 members)
//! - Group admin controls (add/remove members, change name, etc.)
//! - End-to-end encryption using Signal Protocol Sender Keys
//! - Persistent storage (SQLite)
//! - Message ordering and deduplication
//!
//! Architecture:
//! - GossipSub for message broadcasting (P2P pub/sub)
//! - Sender Keys for group E2E encryption (one key per sender)
//! - Admin-only operations via signed messages
//! - Optimistic UI updates with eventual consistency

pub mod envelope;
pub mod manager;
pub mod storage;
pub mod types;

// Re-exports
pub use envelope::{GroupControlEnvelope, GROUP_CONTROL_PREFIX};
pub use manager::GroupManager;
pub use types::{Group, GroupMember, GroupMessage, GroupRole, GroupEvent};
