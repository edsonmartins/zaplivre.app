//! Identity Server - Main entry point

use axum::{
    middleware,
    routing::{get, post, put},
    Router,
};
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use zaplivre_identity_server::{db, handlers, rate_limit, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "zaplivre_identity_server=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Identity Server v{}", env!("CARGO_PKG_VERSION"));

    // Load environment variables
    dotenvy::dotenv().ok();

    // Get configuration from environment
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://zaplivre:zaplivre@localhost/zaplivre_identity".to_string());

    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost".to_string());

    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8083".to_string());

    // Initialize database connection pool
    tracing::info!("Connecting to PostgreSQL...");
    let db_pool = db::init_pool(&database_url).await?;
    tracing::info!("Connected to PostgreSQL");

    // Initialize Redis connection
    tracing::info!("Connecting to Redis...");
    let redis_client = redis::Client::open(redis_url)?;
    let redis_conn = redis::aio::ConnectionManager::new(redis_client).await?;
    tracing::info!("Connected to Redis");

    // Create application state
    let state = Arc::new(AppState::new(db_pool, redis_conn));

    // Build router
    let app = Router::new()
        // API routes
        .route("/api/v1/register", post(handlers::register_handler))
        .route("/api/v1/lookup", get(handlers::lookup_handler))
        .route("/api/v1/prekeys", put(handlers::update_prekeys_handler))
        // Rate limiting middleware for API routes
        .layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit::rate_limit_middleware,
        ))
        // Health check (no rate limit)
        .route("/health", get(handlers::health_handler))
        // Shared state
        .with_state(state)
        // Middleware
        // SEC-16: sem CORS permissivo - clientes nativos não precisam de CORS;
        // browsers não devem chamar esta API diretamente
        .layer(CorsLayer::new())
        .layer(TraceLayer::new_for_http());

    // Start server
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("Identity Server listening on {}", bind_addr);

    // with_connect_info: o rate limit usa o IP real da conexão (SEC-15)
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;

    Ok(())
}
