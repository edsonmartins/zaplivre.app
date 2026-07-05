//! Rate limiting middleware using Redis

use axum::{
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::net::SocketAddr;
use redis::AsyncCommands;
use std::sync::Arc;

use crate::AppState;

/// Rate limit configuration per endpoint
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub max_requests: u32,
    pub window_seconds: u64,
}

impl RateLimitConfig {
    /// Registration: 5 requests per hour
    pub fn register() -> Self {
        Self {
            max_requests: 5,
            window_seconds: 3600, // 1 hour
        }
    }

    /// Lookup: 100 requests per hour
    pub fn lookup() -> Self {
        Self {
            max_requests: 100,
            window_seconds: 3600,
        }
    }

    /// Update prekeys: 50 requests per hour
    pub fn update_prekeys() -> Self {
        Self {
            max_requests: 50,
            window_seconds: 3600,
        }
    }
}

/// Extract client identifier: IP real da conexão como base (não forjável);
/// x-forwarded-for só é considerado se a conexão vier de rede privada
/// (proxy reverso confiável) - SEC-15
fn get_client_id(req: &Request) -> String {
    let socket_ip = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip());

    let is_trusted_proxy = socket_ip
        .map(|ip| match ip {
            std::net::IpAddr::V4(v4) => {
                v4.is_private() || v4.is_loopback()
            }
            std::net::IpAddr::V6(v6) => v6.is_loopback(),
        })
        .unwrap_or(false);

    if is_trusted_proxy {
        if let Some(forwarded) = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
        {
            return forwarded.trim().to_string();
        }
    }

    socket_ip
        .map(|ip| ip.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // CI/testes de integração: RATE_LIMIT_DISABLED=1 desliga o limitador.
    // Nunca usar em produção.
    if std::env::var("RATE_LIMIT_DISABLED").as_deref() == Ok("1") {
        return Ok(next.run(req).await);
    }

    let path = req.uri().path();

    // Determine rate limit config based on path
    let config = match path {
        p if p.starts_with("/api/v1/register") => RateLimitConfig::register(),
        p if p.starts_with("/api/v1/lookup") => RateLimitConfig::lookup(),
        p if p.starts_with("/api/v1/prekeys") => RateLimitConfig::update_prekeys(),
        _ => {
            // No rate limit for other endpoints (like /health)
            return Ok(next.run(req).await);
        }
    };

    let client_id = get_client_id(&req);
    let key = format!("ratelimit:{}:{}", path, client_id);

    // Check rate limit
    let mut conn = state.redis.clone();

    // Get current count
    let count: Option<u32> = conn
        .get(&key)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let current_count = count.unwrap_or(0);

    if current_count >= config.max_requests {
        tracing::warn!(
            "Rate limit exceeded for {} on {}",
            client_id,
            path
        );
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    // Increment counter
    let new_count: u32 = conn
        .incr(&key, 1)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Set expiry on first request
    if new_count == 1 {
        let _: () = conn
            .expire(&key, config.window_seconds as i64)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Add rate limit headers
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert(
        "X-RateLimit-Limit",
        config.max_requests.to_string().parse().unwrap(),
    );
    headers.insert(
        "X-RateLimit-Remaining",
        (config.max_requests.saturating_sub(new_count))
            .to_string()
            .parse()
            .unwrap(),
    );

    Ok(response)
}
