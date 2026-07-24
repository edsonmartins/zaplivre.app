//! ZapLivre Message Store
//!
//! Store & forward service for offline message delivery

use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
use std::env;

mod api;
mod auth;
mod database;
mod models;
mod push_notifier;
mod redis_client;
mod ttl_cleanup;

use database::Database;
use redis_client::RedisClient;
use ttl_cleanup::TtlCleanupJob;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,zaplivre_store=debug".into()),
        )
        .init();

    tracing::info!("🚀 ZapLivre Message Store starting...");

    // Load configuration from environment (SEC-12: sem credenciais default
    // embutidas no binário - falhar cedo com mensagem clara)
    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL must be set (see .env.example)");

    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set (see .env.example)");

    let server_port: u16 = env::var("SERVER_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("SERVER_PORT must be a valid port number");

    let enable_ttl_cleanup = env::var("ENABLE_TTL_CLEANUP")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    // Connect to database
    tracing::info!("📦 Connecting to database...");
    let database = Database::new(&database_url)
        .await
        .expect("Failed to connect to database");

    // Connect to Redis
    tracing::info!("📦 Connecting to Redis...");
    let redis = RedisClient::new(&redis_url).expect("Failed to connect to Redis");

    // Start TTL cleanup job in background
    if enable_ttl_cleanup {
        let cleanup_db = database.clone();
        tokio::spawn(async move {
            TtlCleanupJob::new(cleanup_db).start().await;
        });
    }

    // Push integration (PSH-02): notificar destinatário offline
    let push_server_url = env::var("PUSH_SERVER_URL").ok();
    let push_service_secret = if push_server_url.is_some() {
        let secret = env::var("PUSH_SERVICE_SECRET")
            .expect("PUSH_SERVICE_SECRET must be set when PUSH_SERVER_URL is configured");
        assert!(
            secret.len() >= 32,
            "PUSH_SERVICE_SECRET must contain at least 32 characters"
        );
        Some(secret)
    } else {
        None
    };
    let push_notifier = push_notifier::PushNotifier::new(push_server_url, push_service_secret);

    // Create shared state
    let db_data = web::Data::new(database);
    let redis_data = web::Data::new(redis);
    let push_data = web::Data::new(push_notifier);

    tracing::info!("🌐 Starting HTTP server on port {}", server_port);
    tracing::info!("   POST   /api/store           - Store offline message");
    tracing::info!("   GET    /api/store           - Retrieve pending messages");
    tracing::info!("   DELETE /api/store           - Acknowledge messages");
    tracing::info!("   GET    /api/stats           - Get statistics");
    tracing::info!("   GET    /health              - Health check");

    // Start HTTP server
    HttpServer::new(move || {
        // SEC-16: sem CORS permissivo - clientes nativos não usam CORS e
        // browsers não devem chamar esta API diretamente
        let cors = Cors::default();

        App::new()
            // Bound JSON bodies accepted by the store (encrypted envelopes are
            // small; oversized payloads must be rejected before allocation).
            .app_data(web::PayloadConfig::new(64 * 1024))
            // State
            .app_data(db_data.clone())
            .app_data(push_data.clone())
            .app_data(redis_data.clone())
            // Middleware
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .wrap(cors)
            // Routes
            .route("/health", web::get().to(api::health_check))
            .route("/api/stats", web::get().to(api::get_stats))
            .route("/api/store", web::post().to(api::store_message))
            .route("/api/store", web::get().to(api::retrieve_messages))
            .route("/api/store", web::delete().to(api::delete_messages))
    })
    .bind(("0.0.0.0", server_port))?
    .run()
    .await
}
