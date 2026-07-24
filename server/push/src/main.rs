//! ZapLivre Push Notification Server
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
mod auth;
mod fcm;

use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub db_pool: sqlx::PgPool,
    pub fcm_client: Option<Arc<fcm::FcmClient>>,
    pub apns_client: Option<Arc<apns::ApnsClient>>,
    pub service_secret: Arc<str>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Setup logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "zaplivre_push=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🚀 ZapLivre Push Notification Server starting...");

    // Get configuration from environment
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8081".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid number");
    let service_secret =
        std::env::var("PUSH_SERVICE_SECRET").expect("PUSH_SERVICE_SECRET must be set");
    if service_secret.len() < 32 {
        return Err("PUSH_SERVICE_SECRET must contain at least 32 characters".into());
    }
    let max_concurrent = std::env::var("PUSH_MAX_CONCURRENT")
        .unwrap_or_else(|_| "128".to_string())
        .parse::<usize>()?;
    if !(1..=4096).contains(&max_concurrent) {
        return Err("PUSH_MAX_CONCURRENT must be between 1 and 4096".into());
    }

    // Connect to database
    tracing::info!("📦 Connecting to database...");
    let db_pool = sqlx::PgPool::connect(&database_url).await?;
    tracing::info!("✅ Database connected");

    // Initialize FCM client (HTTP v1 via service account) - opcional
    // (PSH-01: a Legacy API com FCM_SERVER_KEY foi desligada pelo Google)
    let fcm_client = match std::env::var("FCM_SERVICE_ACCOUNT_PATH") {
        Ok(path) if !path.trim().is_empty() => {
            tracing::info!("🔥 Initializing FCM v1 client...");
            match fcm::FcmClient::from_service_account_file(&path) {
                Ok(client) => {
                    tracing::info!("✅ FCM v1 client ready (project: {})", client.project_id());
                    Some(Arc::new(client))
                }
                Err(e) => {
                    tracing::error!("❌ Failed to initialize FCM client: {}", e);
                    tracing::warn!("⚠️  Continuing without FCM support");
                    None
                }
            }
        }
        _ => {
            tracing::info!("ℹ️  FCM not configured - Android push notifications disabled");
            tracing::info!("   Set FCM_SERVICE_ACCOUNT_PATH to the service account JSON to enable");
            None
        }
    };

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
            tracing::info!(
                "   Set APNS_KEY_PATH, APNS_KEY_ID, APNS_TEAM_ID, APNS_BUNDLE_ID to enable"
            );
            None
        }
    };

    // Create app state
    let state = AppState {
        db_pool,
        fcm_client,
        apns_client,
        service_secret: Arc::from(service_secret),
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
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(ConcurrencyLimitLayer::new(max_concurrent))
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
