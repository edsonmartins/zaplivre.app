//! Redis client for presence and notifications

use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, Client};

/// Redis client for presence tracking and pub/sub
#[derive(Clone)]
pub struct RedisClient {
    client: Client,
}

impl RedisClient {
    /// Create a new Redis client
    pub fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        let client = Client::open(redis_url)?;
        tracing::info!("✅ Redis client created");
        Ok(Self { client })
    }

    /// Get a connection to Redis
    async fn get_connection(&self) -> Result<MultiplexedConnection, redis::RedisError> {
        self.client.get_multiplexed_tokio_connection().await
    }

    /// Publish a notification that new messages are available
    pub async fn publish_message_notification(
        &self,
        peer_id: &str,
    ) -> Result<(), redis::RedisError> {
        let mut conn = self.get_connection().await?;

        let channel = format!("messages:{}", peer_id);
        let _: () = conn.publish(&channel, "new_message").await?;

        tracing::debug!("📢 Published notification to channel: {}", channel);

        Ok(())
    }

    /// Check if a peer is online (in presence set)
    /// Presença planejada (não usada ainda - integração futura com o push)
    #[allow(dead_code)]
    pub async fn is_peer_online(&self, peer_id: &str) -> Result<bool, redis::RedisError> {
        let mut conn = self.get_connection().await?;

        let key = format!("presence:{}", peer_id);
        let exists: bool = conn.exists(&key).await?;

        Ok(exists)
    }

    /// Set peer presence (online)
    #[allow(dead_code)]
    pub async fn set_peer_online(
        &self,
        peer_id: &str,
        ttl_seconds: u64,
    ) -> Result<(), redis::RedisError> {
        let mut conn = self.get_connection().await?;

        let key = format!("presence:{}", peer_id);
        let _: () = conn.set_ex(&key, "online", ttl_seconds).await?;

        tracing::debug!("✅ Set {} as online (TTL: {}s)", peer_id, ttl_seconds);

        Ok(())
    }

    /// Remove peer presence (offline)
    #[allow(dead_code)]
    pub async fn set_peer_offline(&self, peer_id: &str) -> Result<(), redis::RedisError> {
        let mut conn = self.get_connection().await?;

        let key = format!("presence:{}", peer_id);
        let _: () = conn.del(&key).await?;

        tracing::debug!("📴 Set {} as offline", peer_id);

        Ok(())
    }

    /// Health check
    pub async fn health_check(&self) -> Result<String, redis::RedisError> {
        let mut conn = self.get_connection().await?;
        let pong: String = redis::cmd("PING").query_async(&mut conn).await?;

        if pong == "PONG" {
            Ok("healthy".to_string())
        } else {
            Err(redis::RedisError::from((
                redis::ErrorKind::ResponseError,
                "Unexpected PING response",
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires Redis
    async fn test_redis_connection() {
        let redis_url = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://:zaplivre_redis_dev@localhost:6379".to_string());

        let redis = RedisClient::new(&redis_url);
        assert!(redis.is_ok());

        let redis = redis.unwrap();
        let health = redis.health_check().await;
        assert!(health.is_ok());
        assert_eq!(health.unwrap(), "healthy");
    }
}
