//! Database connection and operations

use base64::{engine::general_purpose, Engine as _};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use std::time::Duration;
use uuid::Uuid;

use crate::models::{OfflineMessage, StoreMessageRequest};

/// Database manager
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new database connection pool
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .acquire_timeout(Duration::from_secs(30))
            .connect(database_url)
            .await?;

        tracing::info!("✅ Database connection pool established");

        Ok(Self { pool })
    }

    /// Get the connection pool
    #[allow(dead_code)]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Store a new offline message
    pub async fn store_message(
        &self,
        req: &StoreMessageRequest,
    ) -> Result<(Uuid, String), sqlx::Error> {
        let payload_bytes = general_purpose::STANDARD
            .decode(&req.encrypted_payload)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        let message_type = req
            .message_type
            .clone()
            .unwrap_or_else(|| "text".to_string());

        let row = sqlx::query(
            r#"
            INSERT INTO offline_messages (
                recipient_peer_id,
                sender_peer_id,
                encrypted_payload,
                message_type,
                message_id,
                payload_size_bytes
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (message_id) DO NOTHING
            RETURNING id, created_at, expires_at
            "#,
        )
        .bind(&req.recipient_peer_id)
        .bind(&req.sender_peer_id)
        .bind(&payload_bytes)
        .bind(&message_type)
        .bind(&req.message_id)
        .bind(payload_bytes.len() as i32)
        .fetch_optional(&self.pool)
        .await?;

        // PSH-04: duplicata (retry do cliente) é idempotente - o ON CONFLICT
        // DO NOTHING não retorna linha; buscar o registro existente em vez de
        // responder 500 (fetch_one falhava com RowNotFound)
        let id: Uuid = match row {
            Some(row) => row.get("id"),
            None => {
                tracing::debug!("📩 Duplicate message {} (idempotent)", req.message_id);
                sqlx::query("SELECT id FROM offline_messages WHERE message_id = $1")
                    .bind(&req.message_id)
                    .fetch_one(&self.pool)
                    .await?
                    .get("id")
            }
        };

        tracing::info!(
            "📩 Stored message {} for {} (from: {}, size: {} bytes)",
            req.message_id,
            req.recipient_peer_id,
            req.sender_peer_id,
            payload_bytes.len()
        );

        Ok((id, req.message_id.clone()))
    }

    /// Retrieve pending messages for a recipient
    pub async fn retrieve_messages(
        &self,
        peer_id: &str,
        limit: Option<i32>,
    ) -> Result<Vec<OfflineMessage>, sqlx::Error> {
        let limit = limit.unwrap_or(100).min(1000); // Max 1000 messages

        let messages = sqlx::query_as::<_, OfflineMessage>(
            r#"
            SELECT
                id,
                recipient_peer_id,
                sender_peer_id,
                encrypted_payload,
                message_type,
                message_id,
                created_at,
                expires_at,
                delivered_at,
                status as "status: MessageStatus",
                delivery_attempts,
                last_attempt_at,
                payload_size_bytes
            FROM offline_messages
            WHERE recipient_peer_id = $1
              AND status = 'pending'
              AND expires_at > NOW()
            ORDER BY created_at ASC
            LIMIT $2
            "#,
        )
        .bind(peer_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        tracing::info!(
            "📨 Retrieved {} pending messages for {}",
            messages.len(),
            peer_id
        );

        Ok(messages)
    }

    /// Marca mensagens como entregues - apenas as endereçadas ao peer
    /// autenticado (SEC-09: um peer não pode dar ack em mensagens alheias)
    pub async fn delete_messages(
        &self,
        message_ids: &[String],
        recipient_peer_id: &str,
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE offline_messages
            SET status = 'delivered',
                delivered_at = NOW()
            WHERE message_id = ANY($1)
              AND recipient_peer_id = $2
              AND status = 'pending'
            "#,
        )
        .bind(message_ids)
        .bind(recipient_peer_id)
        .execute(&self.pool)
        .await?;

        tracing::info!("✅ Marked {} messages as delivered", result.rows_affected());

        Ok(result.rows_affected() as i64)
    }

    /// Delete expired messages (TTL cleanup)
    pub async fn delete_expired_messages(&self) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM offline_messages
            WHERE status = 'pending'
              AND expires_at < NOW()
            "#,
        )
        .execute(&self.pool)
        .await?;

        let deleted = result.rows_affected();

        if deleted > 0 {
            tracing::info!("🗑️ Deleted {} expired messages", deleted);
        }

        Ok(deleted as i64)
    }

    /// PSH-05: purga mensagens já ENTREGUES com mais de 7 dias
    /// (antes cresciam indefinidamente - só as pending expiradas eram limpas)
    pub async fn purge_delivered_messages(&self) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM offline_messages
            WHERE status = 'delivered'
              AND delivered_at < NOW() - INTERVAL '7 days'
            "#,
        )
        .execute(&self.pool)
        .await?;

        let deleted = result.rows_affected();
        if deleted > 0 {
            tracing::info!("🗑️ Purged {} delivered messages (>7d)", deleted);
        }

        Ok(deleted as i64)
    }

    /// Get count of pending messages
    pub async fn count_pending_messages(&self) -> Result<i64, sqlx::Error> {
        let row =
            sqlx::query("SELECT COUNT(*) as count FROM offline_messages WHERE status = 'pending'")
                .fetch_one(&self.pool)
                .await?;

        let count: i64 = row.get("count");
        Ok(count)
    }

    /// Health check - verify database connection
    pub async fn health_check(&self) -> Result<String, sqlx::Error> {
        sqlx::query("SELECT 1").fetch_one(&self.pool).await?;

        Ok("healthy".to_string())
    }

    /// Increment message stats
    pub async fn increment_stats(&self, delivery_type: &str) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT increment_message_stats($1)")
            .bind(delivery_type)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires database
    async fn test_database_connection() {
        let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgresql://zaplivre:zaplivre_dev_password@localhost:5432/zaplivre".to_string()
        });

        let db = Database::new(&db_url).await;
        assert!(db.is_ok());
    }
}
