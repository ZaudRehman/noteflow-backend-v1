use redis::{aio::ConnectionManager, Client};
use crate::utils::errors::{AppError, Result};

pub async fn create_redis_client(redis_url: &str) -> Result<ConnectionManager> {
    let client = Client::open(redis_url)
        .map_err(|e| AppError::RedisError(e))?;
    
    ConnectionManager::new(client)
        .await
        .map_err(|e| AppError::RedisError(e))
}

pub struct RedisManager {
    pub conn: ConnectionManager,
}

impl RedisManager {
    pub fn new(conn: ConnectionManager) -> Self {
        Self { conn }
    }

    pub async fn publish(&mut self, channel: &str, message: &str) -> Result<()> {
        redis::cmd("PUBLISH")
            .arg(channel)
            .arg(message)
            .query_async(&mut self.conn)
            .await
            .map_err(|e| AppError::RedisError(e))
    }

    pub async fn set_with_expiry(&mut self, key: &str, value: &str, seconds: usize) -> Result<()> {
        redis::cmd("SETEX")
            .arg(key)
            .arg(seconds)
            .arg(value)
            .query_async(&mut self.conn)
            .await
            .map_err(|e| AppError::RedisError(e))
    }

    pub async fn get(&mut self, key: &str) -> Result<Option<String>> {
        redis::cmd("GET")
            .arg(key)
            .query_async(&mut self.conn)
            .await
            .map_err(|e| AppError::RedisError(e))
    }

    pub async fn delete(&mut self, key: &str) -> Result<()> {
        redis::cmd("DEL")
            .arg(key)
            .query_async(&mut self.conn)
            .await
            .map_err(|e| AppError::RedisError(e))
    }
}