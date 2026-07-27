use axum::{
    extract::{ConnectInfo, Request, State},
    http::HeaderValue,
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

    pub fn limit(&self) -> u32 {
        self.limit
    }

    /// Check rate limit and return (allowed, remaining_after_this_request).
    pub async fn check_rate_limit(&self, key: &str) -> (bool, u32) {
        if let Some(ref redis) = self.redis {
            match redis
                .check_rate_limit(key, self.limit, self.window_secs)
                .await
            {
                Ok((allowed, remaining)) => {
                    if !allowed {
                        tracing::warn!("Rate limit exceeded (redis) for key: {}", key);
                    }
                    return (allowed, remaining);
                }
                Err(e) => {
                    tracing::warn!("Redis rate limit failed, falling back to in-memory: {}", e);
                }
            }
        }
        self.check_rate_limit_in_memory(key).await
    }

    async fn check_rate_limit_in_memory(&self, key: &str) -> (bool, u32) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut requests = self.requests.write().await;
        let timestamps = requests.entry(key.to_string()).or_insert_with(Vec::new);
        timestamps.retain(|&t| now - t < self.window_secs);

        let count = timestamps.len() as u32;
        if count >= self.limit {
            tracing::warn!("Rate limit exceeded (memory) for key: {}", key);
            return (false, 0);
        }

        timestamps.push(now);
        (true, self.limit - count - 1)
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
    let (allowed, remaining) = rate_limiter.check_rate_limit(&ip).await;

    if !allowed {
        return Err(AppError::RateLimitExceeded);
    }

    tracing::debug!("Request from {} - Remaining: {}", ip, remaining);

    let mut response = next.run(req).await;
    let limit_str = HeaderValue::from(rate_limiter.limit());
    let remaining_str = HeaderValue::from(remaining);
    response
        .headers_mut()
        .insert("X-RateLimit-Limit", limit_str);
    response
        .headers_mut()
        .insert("X-RateLimit-Remaining", remaining_str);

    Ok(response)
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
