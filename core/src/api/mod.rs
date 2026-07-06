//! Public API module
//!
//! Main Client API for ZapLivre.

pub mod builder;
pub mod client;
pub mod events;

pub use builder::ClientBuilder;
pub use client::Client;
pub use events::{ClientEvent, EventCallback, FunctionCallback};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Client not initialized")]
    NotInitialized,

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Crypto error: {0}")]
    CryptoError(String),

    #[error("Storage error: {0}")]
    StorageError(String),
}

pub type Result<T> = std::result::Result<T, ApiError>;
