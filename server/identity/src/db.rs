//! Database operations for Identity Server

use crate::{error::Result, models::*};
use base64::{engine::general_purpose, Engine as _};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};

/// Initialize database connection pool
pub async fn init_pool(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    // Keep existing deployments compatible with the transparency feature.
    // The canonical SQL files create this table for fresh databases, while
    // this idempotent bootstrap also upgrades an already-running database.
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS key_transparency_log (
            sequence BIGSERIAL PRIMARY KEY,
            peer_id TEXT NOT NULL,
            public_key BYTEA NOT NULL,
            previous_hash BYTEA NOT NULL,
            entry_hash BYTEA NOT NULL UNIQUE,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
        )"#,
    )
    .execute(&pool)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_key_transparency_peer ON key_transparency_log(peer_id, sequence DESC)")
        .execute(&pool)
        .await?;
    backfill_transparency_log(&pool).await?;
    Ok(pool)
}

/// Add pre-existing registrations to the log exactly once during startup.
async fn backfill_transparency_log(pool: &PgPool) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(820_514)")
        .execute(&mut *tx)
        .await?;
    let rows = sqlx::query(
        "SELECT u.peer_id, u.public_key FROM usernames u LEFT JOIN key_transparency_log l ON l.peer_id = u.peer_id WHERE l.peer_id IS NULL ORDER BY u.created_at ASC, u.peer_id ASC",
    )
    .fetch_all(&mut *tx)
    .await?;
    for row in rows {
        let peer_id: String = row.try_get("peer_id")?;
        let public_key: Vec<u8> = row.try_get("public_key")?;
        append_transparency_entry(&mut tx, &peer_id, &public_key).await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Validate username format (3-20 chars, lowercase alphanumeric + underscore)
pub fn validate_username(username: &str) -> Result<()> {
    let regex = regex::Regex::new(r"^[a-z0-9_]{3,20}$").unwrap();

    if !regex.is_match(username) {
        return Err(crate::error::AppError::InvalidUsername(
            "Username must be 3-20 characters, lowercase alphanumeric and underscore only"
                .to_string(),
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

    let mut tx = pool.begin().await?;
    // Serialize writers so the hash chain has one unambiguous predecessor.
    sqlx::query("SELECT pg_advisory_xact_lock(820_514)")
        .execute(&mut *tx)
        .await?;
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
    .fetch_one(&mut *tx)
    .await;

    match result {
        Ok(row) => {
            let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
            append_transparency_entry(&mut tx, peer_id, public_key).await?;
            tx.commit().await?;
            Ok(RegisterResponse {
                username: username.to_string(),
                peer_id: peer_id.to_string(),
                created_at,
            })
        }
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            Err(crate::error::AppError::UsernameTaken(username.to_string()))
        }
        Err(e) => {
            tx.rollback().await.ok();
            Err(e.into())
        }
    }
}

/// Append a new identity binding to the tamper-evident hash chain.
async fn append_transparency_entry(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    peer_id: &str,
    public_key: &[u8],
) -> Result<()> {
    let previous: Option<(i64, Vec<u8>)> = sqlx::query_as(
        "SELECT sequence, entry_hash FROM key_transparency_log ORDER BY sequence DESC LIMIT 1",
    )
    .fetch_optional(&mut **tx)
    .await?;
    let previous_hash = previous
        .map(|(_, hash)| hash)
        .unwrap_or_else(|| vec![0; 32]);
    let entry_hash = crate::transparency::entry_hash(&previous_hash, peer_id, public_key).to_vec();

    sqlx::query(
        "INSERT INTO key_transparency_log (peer_id, public_key, previous_hash, entry_hash) VALUES ($1, $2, $3, $4)",
    )
    .bind(peer_id)
    .bind(public_key)
    .bind(&previous_hash)
    .bind(&entry_hash)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Return the identity's log entry and the current root for independent auditing.
pub async fn transparency_for_peer(pool: &PgPool, peer_id: &str) -> Result<TransparencyResponse> {
    let row = sqlx::query(
        "SELECT sequence, peer_id, public_key, previous_hash, entry_hash FROM key_transparency_log WHERE peer_id = $1 ORDER BY sequence DESC LIMIT 1",
    )
    .bind(peer_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| crate::error::AppError::UsernameNotFound(peer_id.to_string()))?;
    let root = sqlx::query(
        "SELECT sequence, entry_hash FROM key_transparency_log ORDER BY sequence DESC LIMIT 1",
    )
    .fetch_one(pool)
    .await?;
    let public_key: Vec<u8> = row.try_get("public_key")?;
    let entry_hash: Vec<u8> = row.try_get("entry_hash")?;
    let previous_hash: Vec<u8> = row.try_get("previous_hash")?;
    let root_hash: Vec<u8> = root.try_get("entry_hash")?;
    Ok(TransparencyResponse {
        peer_id: row.try_get("peer_id")?,
        public_key: general_purpose::STANDARD.encode(&public_key),
        fingerprint: fingerprint(&public_key),
        sequence: row.try_get("sequence")?,
        entry_hash: general_purpose::STANDARD.encode(entry_hash),
        previous_hash: general_purpose::STANDARD.encode(previous_hash),
        log_root_sequence: root.try_get("sequence")?,
        log_root_hash: general_purpose::STANDARD.encode(root_hash),
    })
}

/// Return a bounded, ordered segment of the transparency log for auditors.
pub async fn transparency_log_segment(
    pool: &PgPool,
    from_sequence: i64,
    limit: i64,
) -> Result<Vec<TransparencyLogEntry>> {
    let rows = sqlx::query(
        "SELECT sequence, peer_id, public_key, previous_hash, entry_hash, created_at FROM key_transparency_log WHERE sequence >= $1 ORDER BY sequence ASC LIMIT $2",
    )
    .bind(from_sequence.max(1))
    .bind(limit.clamp(1, 1000))
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|row| {
            let public_key: Vec<u8> = row.try_get("public_key")?;
            let previous_hash: Vec<u8> = row.try_get("previous_hash")?;
            let entry_hash: Vec<u8> = row.try_get("entry_hash")?;
            Ok(TransparencyLogEntry {
                sequence: row.try_get("sequence")?,
                peer_id: row.try_get("peer_id")?,
                public_key: general_purpose::STANDARD.encode(public_key),
                previous_hash: general_purpose::STANDARD.encode(previous_hash),
                entry_hash: general_purpose::STANDARD.encode(entry_hash),
                created_at: row.try_get("created_at")?,
            })
        })
        .collect()
}

fn fingerprint(public_key: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"zaplivre-identity-fingerprint-v1\0");
    hasher.update(public_key);
    hex::encode_upper(hasher.finalize())
        .as_bytes()
        .chunks(4)
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or_default())
        .collect::<Vec<_>>()
        .join(" ")
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
        None => Err(crate::error::AppError::UsernameNotFound(
            username.to_string(),
        )),
    }
}

/// Lookup a registered prekey bundle by peer identity.
pub async fn lookup_peer_id(pool: &PgPool, peer_id: &str) -> Result<LookupResponse> {
    let row = sqlx::query_as::<_, UsernameRow>(
        r#"
        SELECT username, peer_id, public_key, prekey_bundle, created_at, last_updated
        FROM usernames
        WHERE peer_id = $1
        "#,
    )
    .bind(peer_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => row
            .to_lookup_response()
            .map_err(|e| crate::error::AppError::Internal(e.into())),
        None => Err(crate::error::AppError::UsernameNotFound(
            peer_id.to_string(),
        )),
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
        None => Err(crate::error::AppError::UsernameNotFound(
            peer_id.to_string(),
        )),
    }
}

/// Check database health
pub async fn check_health(pool: &PgPool) -> Result<f64> {
    let start = std::time::Instant::now();

    sqlx::query("SELECT 1 as check").fetch_one(pool).await?;

    let latency = start.elapsed().as_secs_f64() * 1000.0; // Convert to ms
    Ok(latency)
}
