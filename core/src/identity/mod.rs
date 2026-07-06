//! Identity management module
//!
//! Handles Ed25519 keypairs, peer identity, and prekeys (X25519).
//!
//! # Examples
//!
//! ```no_run
//! use zaplivre_core::identity::{Identity, FileIdentityStorage, IdentityStorage};
//!
//! // Generate a new identity
//! let identity = Identity::generate(100);
//! println!("Peer ID: {}", identity.peer_id());
//!
//! // Save to storage
//! let storage = FileIdentityStorage::new("./data");
//! storage.save_identity(&identity).unwrap();
//!
//! // Load from storage
//! let loaded = storage.load_identity().unwrap().unwrap();
//! assert_eq!(identity.peer_id(), loaded.peer_id());
//! ```

pub mod keypair;
pub mod prekeys;
pub mod storage;

pub use keypair::{Keypair, PublicKey, SignalKeypair};
pub use prekeys::{PreKey, PreKeyBundle, PreKeyPool, OneTimePreKey};
pub use storage::{Identity, IdentityStorage, FileIdentityStorage, MemoryIdentityStorage};
