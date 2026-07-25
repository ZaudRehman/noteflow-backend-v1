use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::Next,
    response::Response,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use crate::db::RedisManager;
use crate::utils::errors::AppError;

/// Dual-mode rate limiter: tries Redis first, falls back to in-memory.
/// Redis persistence survives restarts; in-memory is always available.
#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<RwLock<HashMap<String, Vec<u64>>>>,
    limit: u32,
    window_secs: u64,
    redis: Option<Arc<RedisManager>>,
}

impl RateLimiter {
    pub fn new(limit: u32, window_secs: u64, redis: Option<Arc<RedisManager>>) -> Self {
        Self {
            requests: Arc::new(RwLock::new(HashMap::new())),
            limit,
            window_secs,
            redis,
        }
    }

    pub async fn check_rate_limit(&self, key: &str) -> bool {
        if let Some(ref redis) = self.redis {
            match redis
                .check_rate_limit(key, self.limit, self.window_secs)
                .await
            {
                Ok((allowed, _)) => {
                    if !allowed {
                        tracing::warn!("Rate limit exceeded (redis) for key: {}", key);
                    }
                    return allowed;
                }
                Err(e) => {
                    tracing::warn!("Redis rate limit failed, falling back to in-memory: {}", e);
                }
            }
        }
        self.check_rate_limit_in_memory(key).await
    }

    async fn check_rate_limit_in_memory(&self, key: &str) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut requests = self.requests.write().await;
        let timestamps = requests.entry(key.to_string()).or_insert_with(Vec::new);
        timestamps.retain(|&t| now - t < self.window_secs);

        if timestamps.len() >= self.limit as usize {
            tracing::warn!("Rate limit exceeded (memory) for key: {}", key);
            return false;
        }

        timestamps.push(now);
        true
    }

    pub async fn get_remaining(&self, key: &str) -> u32 {
        if let Some(ref redis) = self.redis {
            if let Ok((_, remaining)) = redis
                .check_rate_limit(key, self.limit, self.window_secs)
                .await
            {
                return remaining;
            }
        }
        self.get_remaining_in_memory(key).await
    }

    async fn get_remaining_in_memory(&self, key: &str) -> u32 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let requests = self.requests.read().await;
        if let Some(timestamps) = requests.get(key) {
            let valid_count = timestamps
                .iter()
                .filter(|&&t| now - t < self.window_secs)
                .count();
            self.limit.saturating_sub(valid_count as u32)
        } else {
            self.limit
        }
    }

    /// Only needed for in-memory entries; Redis handles its own expiry
    pub async fn cleanup(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut requests = self.requests.write().await;
        requests.retain(|_, timestamps| {
            timestamps.retain(|&t| now - t < self.window_secs);
            !timestamps.is_empty()
        });

        tracing::debug!(
            "Rate limiter in-memory cleanup done. Active keys: {}",
            requests.len()
        );
    }
}

pub async fn rate_limit_middleware(
    State(rate_limiter): State<Arc<RateLimiter>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let ip = addr.ip().to_string();

    if !rate_limiter.check_rate_limit(&ip).await {
        return Err(AppError::RateLimitExceeded);
    }

    let remaining = rate_limiter.get_remaining(&ip).await;
    tracing::debug!("Request from {} - Remaining: {}", ip, remaining);

    Ok(next.run(req).await)
}

pub fn start_rate_limit_cleanup(rate_limiter: Arc<RateLimiter>) {
    // Only spawn cleanup if Redis is not configured (in-memory needs periodic cleanup)
    if rate_limiter.redis.is_some() {
        return;
    }
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            rate_limiter.cleanup().await;
        }
    });
}
