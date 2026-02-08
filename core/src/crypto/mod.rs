//! Cryptography module
//!
//! Implements Signal Protocol E2E encryption.

pub mod signal;
pub mod group;
pub mod storage;

pub use signal::{SignalEncryptedMessage, SignalSessionManager};
pub use group::{SenderKey, GroupSession, GroupSessionManager};
pub use storage::{decrypt_for_storage, encrypt_for_storage};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Session not found")]
    SessionNotFound,

    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Invalid message format")]
    InvalidMessageFormat,
}

pub type Result<T> = std::result::Result<T, CryptoError>;
