use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::utils::errors::{AppError, Result};
use redis::{aio::ConnectionManager, Client};

/// Create Redis connection manager with retry (exponential backoff: 500ms, 1s, 2s, 4s, 8s)
pub async fn create_redis_client(redis_url: &str) -> Result<ConnectionManager> {
    let client = Client::open(redis_url).map_err(|e| AppError::RedisError(e))?;
    let mut attempt = 0;
    let max_attempts = 5;

    loop {
        match ConnectionManager::new(client.clone()).await {
            Ok(cm) => return Ok(cm),
            Err(e) => {
                attempt += 1;
                if attempt >= max_attempts {
                    return Err(AppError::RedisError(e));
                }
                // 500ms, 1s, 2s, 4s, 8s
                let delay_ms = 500 * (1u64 << (attempt - 1));
                tracing::warn!(
                    "Redis connection attempt {} failed, retrying in {}ms: {}",
                    attempt,
                    delay_ms,
                    e
                );
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
        }
    }
}

pub struct RedisManager {
    pub conn: Arc<RwLock<ConnectionManager>>,
}

impl RedisManager {
    pub fn new(conn: ConnectionManager) -> Self {
        Self {
            conn: Arc::new(RwLock::new(conn)),
        }
    }

    /// Check if connection is alive
    pub async fn ping(&mut self) -> bool {
        redis::cmd("PING")
            .query_async::<_, String>(&mut *self.conn.write().await)
            .await
            .is_ok()
    }

    pub async fn publish(&self, channel: &str, message: &str) -> Result<()> {
        redis::cmd("PUBLISH")
            .arg(channel)
            .arg(message)
            .query_async(&mut *self.conn.write().await)
            .await
            .map_err(|e| AppError::RedisError(e))
    }

    pub async fn set_with_expiry(&self, key: &str, value: &str, seconds: usize) -> Result<()> {
        redis::cmd("SETEX")
            .arg(key)
            .arg(seconds)
            .arg(value)
            .query_async(&mut *self.conn.write().await)
            .await
            .map_err(|e| AppError::RedisError(e))
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        redis::cmd("GET")
            .arg(key)
            .query_async(&mut *self.conn.write().await)
            .await
            .map_err(|e| AppError::RedisError(e))
    }

    pub async fn exists(&self, key: &str) -> Result<bool> {
        redis::cmd("EXISTS")
            .arg(key)
            .query_async(&mut *self.conn.write().await)
            .await
            .map_err(|e| AppError::RedisError(e))
    }

    pub async fn delete(&self, key: &str) -> Result<()> {
        redis::cmd("DEL")
            .arg(key)
            .query_async(&mut *self.conn.write().await)
            .await
            .map_err(|e| AppError::RedisError(e))
    }

    /// Increment a key and set expiry if it's new (returns new value)
    pub async fn incr_with_expiry(&self, key: &str, expiry_secs: usize) -> Result<i64> {
        let mut conn = self.conn.write().await;
        let val: i64 = redis::cmd("INCR")
            .arg(key)
            .query_async(&mut *conn)
            .await
            .map_err(|e| AppError::RedisError(e))?;

        if val == 1 {
            let _: () = redis::cmd("EXPIRE")
                .arg(key)
                .arg(expiry_secs)
                .query_async(&mut *conn)
                .await
                .map_err(|e| AppError::RedisError(e))?;
        }

        Ok(val)
    }

    /// Sliding window rate limit check using Redis sorted sets
    /// Returns (allowed: bool, remaining: u32)
    pub async fn check_rate_limit(
        &self,
        key: &str,
        limit: u32,
        window_secs: u64,
    ) -> Result<(bool, u32)> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as f64;
        let min_score = now - (window_secs as f64 * 1000.0);

        let mut conn = self.conn.write().await;

        let _: () = redis::cmd("ZREMRANGEBYSCORE")
            .arg(key)
            .arg(0)
            .arg(min_score)
            .query_async(&mut *conn)
            .await
            .map_err(|e| AppError::RedisError(e))?;

        let count: i64 = redis::cmd("ZCARD")
            .arg(key)
            .query_async(&mut *conn)
            .await
            .map_err(|e| AppError::RedisError(e))?;

        if count >= limit as i64 {
            return Ok((false, 0));
        }

        let member = format!("{}:{}", key, now);
        let _: () = redis::cmd("ZADD")
            .arg(key)
            .arg(now)
            .arg(&member)
            .query_async(&mut *conn)
            .await
            .map_err(|e| AppError::RedisError(e))?;

        let _: () = redis::cmd("EXPIRE")
            .arg(key)
            .arg(window_secs as usize)
            .query_async(&mut *conn)
            .await
            .map_err(|e| AppError::RedisError(e))?;

        let remaining = limit.saturating_sub((count + 1) as u32);
        Ok((true, remaining))
    }

}
