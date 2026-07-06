//! FFI Module for UniFFI bindings
//!
//! This module exposes the ZapLivre core library to Kotlin and Swift
//! using UniFFI automatic bindings generation.

mod client;
mod types;

pub use client::ZapLivreClient;
pub use types::*;
