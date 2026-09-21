//! Shared HTTP client construction.

use std::time::Duration;

/// Time allowed to establish a connection.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Time allowed for a whole request, body included.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// HTTP client with bounded connect and request times.
///
/// `reqwest::Client::new()` has no timeout at all. The FFI command loop awaits
/// these requests, so a server that accepts the connection and never answers
/// would freeze every call behind it (conversation list, call audio) for good.
pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        // Building only fails if the TLS backend cannot initialize; the
        // default client would fail the very same way.
        .unwrap_or_default()
}
