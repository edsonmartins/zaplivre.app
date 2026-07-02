//! Database operations for Identity Server

use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use crate::{error::Result, models::*};

/// Initialize database connection pool
pub async fn init_pool(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    Ok(pool)
}

/// Validate username format (3-20 chars, lowercase alphanumeric + underscore)
pub fn validate_username(username: &str) -> Result<()> {
    let regex = regex::Regex::new(r"^[a-z0-9_]{3,20}$").unwrap();

    if !regex.is_match(username) {
        return Err(crate::error::AppError::InvalidUsername(
            "Username must be 3-20 characters, lowercase alphanumeric and underscore only".to_string()
        ));
    }

    Ok(())
}

/// Register a new username
pub async fn register_username(
    pool: &PgPool,
    username: &str,
    peer_id: &str,
    public_key: &[u8],
    prekey_bundle: &PreKeyBundle,
) -> Result<RegisterResponse> {
    validate_username(username)?;

    let prekey_bundle_json = serde_json::to_value(prekey_bundle)
        .map_err(|e| crate::error::AppError::Internal(e.into()))?;

    let result = sqlx::query(
        r#"
        INSERT INTO usernames (username, peer_id, public_key, prekey_bundle)
        VALUES ($1, $2, $3, $4)
        RETURNING created_at
        "#,
    )
    .bind(username)
    .bind(peer_id)
    .bind(public_key)
    .bind(prekey_bundle_json)
    .fetch_one(pool)
    .await;

    match result {
        Ok(row) => {
            let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
            Ok(RegisterResponse {
                username: username.to_string(),
                peer_id: peer_id.to_string(),
                created_at,
            })
        }
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            Err(crate::error::AppError::UsernameTaken(username.to_string()))
        }
        Err(e) => Err(e.into()),
    }
}

/// Lookup username
pub async fn lookup_username(pool: &PgPool, username: &str) -> Result<LookupResponse> {
    let row = sqlx::query_as::<_, UsernameRow>(
        r#"
        SELECT username, peer_id, public_key, prekey_bundle, created_at, last_updated
        FROM usernames
        WHERE username = $1
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => row
            .to_lookup_response()
            .map_err(|e| crate::error::AppError::Internal(e.into())),
        None => Err(crate::error::AppError::UsernameNotFound(username.to_string())),
    }
}

/// Lookup the registered public key for a peer (signature verification)
pub async fn get_public_key_by_peer_id(pool: &PgPool, peer_id: &str) -> Result<Option<Vec<u8>>> {
    let row = sqlx::query("SELECT public_key FROM usernames WHERE peer_id = $1")
        .bind(peer_id)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| r.get::<Vec<u8>, _>("public_key")))
}

/// Update prekeys for a username
pub async fn update_prekeys(
    pool: &PgPool,
    peer_id: &str,
    prekey_bundle: &PreKeyBundle,
) -> Result<UpdatePrekeysResponse> {
    let prekey_bundle_json = serde_json::to_value(prekey_bundle)
        .map_err(|e| crate::error::AppError::Internal(e.into()))?;

    let result = sqlx::query(
        r#"
        UPDATE usernames
        SET prekey_bundle = $1, last_updated = NOW()
        WHERE peer_id = $2
        RETURNING last_updated
        "#,
    )
    .bind(prekey_bundle_json)
    .bind(peer_id)
    .fetch_optional(pool)
    .await?;

    match result {
        Some(row) => {
            let last_updated: chrono::DateTime<chrono::Utc> = row.try_get("last_updated")?;
            Ok(UpdatePrekeysResponse {
                updated_at: last_updated,
            })
        }
        None => Err(crate::error::AppError::UsernameNotFound(peer_id.to_string())),
    }
}

/// Check database health
pub async fn check_health(pool: &PgPool) -> Result<f64> {
    let start = std::time::Instant::now();

    sqlx::query("SELECT 1 as check")
        .fetch_one(pool)
        .await?;

    let latency = start.elapsed().as_secs_f64() * 1000.0; // Convert to ms
    Ok(latency)
}
