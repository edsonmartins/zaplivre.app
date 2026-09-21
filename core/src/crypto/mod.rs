//! Cryptography module
//!
//! Implements Signal Protocol E2E encryption.

pub mod group;
pub mod signal;
pub mod storage;

pub use group::{GroupSession, GroupSessionManager, SenderKey};
pub use signal::{SignalEncryptedMessage, SignalSessionManager};
pub use storage::{decrypt_for_storage, encrypt_for_storage};

use thiserror::Error;

/// Política SEC-01: mensagens sem sessão E2E nunca trafegam em plaintext por
/// padrão, inclusive em builds debug usados na homologação. O downgrade só é
/// habilitado explicitamente para desenvolvimento local, e vale tanto para o
/// envio quanto para a recepção.
///
/// The switch only exists in debug builds: a release binary cannot be talked
/// into plaintext by whoever controls its environment.
pub fn plaintext_allowed() -> bool {
    cfg!(debug_assertions)
        && std::env::var("ZAPLIVRE_ALLOW_PLAINTEXT")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
}

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
