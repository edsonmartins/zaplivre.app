//! TURN Credentials Service
//!
//! Microservice for generating time-limited TURN credentials for WebRTC.
//! Uses HMAC-SHA1 as per RFC 5389.

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::cors::CorsLayer;

mod auth;
mod config;
mod handlers;
mod request_auth;

use config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,turn_credentials=debug".to_string()),
        )
        .init();

    tracing::info!("🚀 Starting TURN credentials service");

    // Load configuration
    let config = Config::from_env()?;
    config.validate()?;
    let max_concurrent = std::env::var("TURN_MAX_CONCURRENT")
        .unwrap_or_else(|_| "64".to_string())
        .parse::<usize>()?;
    if !(1..=4096).contains(&max_concurrent) {
        anyhow::bail!("TURN_MAX_CONCURRENT must be between 1 and 4096");
    }

    tracing::info!("   TURN URIs: {:?}", config.turn_uris);
    tracing::info!("   Server port: {}", config.server_port);

    // Create CORS layer (allow all origins in development)
    // SEC-16: sem CORS permissivo - clientes nativos não usam CORS
    let cors = CorsLayer::new();

    // Build router
    let app = Router::new()
        .route("/health", get(handlers::health_check))
        .route(
            "/api/turn/credentials",
            post(handlers::generate_credentials),
        )
        .layer(DefaultBodyLimit::max(4 * 1024))
        .layer(ConcurrencyLimitLayer::new(max_concurrent))
        .layer(cors)
        .with_state(config.clone());

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server_port));
    tracing::info!("🔐 TURN credentials server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
