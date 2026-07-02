//! MePassa Push Notification Server
//!
//! Handles push notifications for FCM (Firebase Cloud Messaging) and APNs (Apple Push Notification Service).
//!
//! # Architecture
//! - Axum web framework for REST API
//! - PostgreSQL for storing device tokens
//! - FCM for sending Android notifications
//! - APNs for sending iOS notifications

mod api;
mod apns;
mod fcm;

use axum::{
    routing::{delete, get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub db_pool: sqlx::PgPool,
    pub fcm_client: Arc<fcm::FcmClient>,
    pub apns_client: Option<Arc<apns::ApnsClient>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Setup logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mepassa_push=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🚀 MePassa Push Notification Server starting...");

    // Get configuration from environment
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let fcm_server_key = std::env::var("FCM_SERVER_KEY")
        .expect("FCM_SERVER_KEY must be set");
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8081".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid number");

    // Connect to database
    tracing::info!("📦 Connecting to database...");
    let db_pool = sqlx::PgPool::connect(&database_url).await?;
    tracing::info!("✅ Database connected");

    // Initialize FCM client
    tracing::info!("🔥 Initializing FCM client...");
    let fcm_client = Arc::new(fcm::FcmClient::new(fcm_server_key));
    tracing::info!("✅ FCM client ready");

    // Initialize APNs client (optional - only if credentials are provided)
    let apns_client = match (
        std::env::var("APNS_KEY_PATH").ok(),
        std::env::var("APNS_KEY_ID").ok(),
        std::env::var("APNS_TEAM_ID").ok(),
        std::env::var("APNS_BUNDLE_ID").ok(),
    ) {
        (Some(key_path), Some(key_id), Some(team_id), Some(bundle_id)) => {
            tracing::info!("🍎 Initializing APNs client...");
            let production = std::env::var("APNS_PRODUCTION")
                .unwrap_or_else(|_| "false".to_string())
                .parse::<bool>()
                .unwrap_or(false);

            match apns::ApnsClient::new(&key_path, key_id, team_id, bundle_id, production) {
                Ok(client) => {
                    tracing::info!("✅ APNs client ready - {}", client.info());
                    Some(Arc::new(client))
                }
                Err(e) => {
                    tracing::error!("❌ Failed to initialize APNs client: {}", e);
                    tracing::warn!("⚠️  Continuing without APNs support");
                    None
                }
            }
        }
        _ => {
            tracing::info!("ℹ️  APNs credentials not configured - iOS push notifications disabled");
            tracing::info!("   Set APNS_KEY_PATH, APNS_KEY_ID, APNS_TEAM_ID, APNS_BUNDLE_ID to enable");
            None
        }
    };

    // Create app state
    let state = AppState {
        db_pool,
        fcm_client,
        apns_client,
    };

    // Setup CORS
    // SEC-16: sem CORS permissivo - clientes nativos não usam CORS
    let cors = CorsLayer::new();

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/register", post(api::register::handle))
        .route("/api/v1/send", post(api::send::handle))
        .route("/api/v1/unregister", delete(api::unregister::handle))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("🎧 Push server listening on {}", addr);
    tracing::info!("📡 Ready to handle push notifications!");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}
