use serde::Serialize;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use warp::Filter;

/// Health check response structure
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    peer_count: usize,
    uptime_seconds: u64,
}

/// Start the health check HTTP server
///
/// Provides a simple GET /health endpoint that returns the current status,
/// number of connected peers, and uptime in seconds.
///
/// # Arguments
/// * `port` - The port to listen on
/// * `peer_count` - Shared atomic counter of connected peers
pub async fn start_server(port: u16, peer_count: Arc<AtomicUsize>) {
    let start_time = std::time::Instant::now();

    let health = warp::path("health").map(move || {
        let response = HealthResponse {
            status: "OK".to_string(),
            peer_count: peer_count.load(Ordering::Relaxed),
            uptime_seconds: start_time.elapsed().as_secs(),
        };
        warp::reply::json(&response)
    });

    tracing::info!("🏥 Health check server starting on port {}", port);

    warp::serve(health).run(([0, 0, 0, 0], port)).await;
}
