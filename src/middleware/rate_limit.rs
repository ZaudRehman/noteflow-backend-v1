use axum::{
    body::Body,
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use crate::utils::errors::AppError;

/// Rate limiter implementation using a sliding window algorithm
#[derive(Clone)]
pub struct RateLimiter {
    /// Stores request timestamps for each IP/key
    requests: Arc<RwLock<HashMap<String, Vec<u64>>>>,
    /// Maximum number of requests allowed
    limit: u32,
    /// Time window in seconds
    window_secs: u64,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(limit: u32, window_secs: u64) -> Self {
        Self {
            requests: Arc::new(RwLock::new(HashMap::new())),
            limit,
            window_secs,
        }
    }

    /// Check if a request should be rate limited
    pub async fn check_rate_limit(&self, key: &str) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut requests = self.requests.write().await;
        let timestamps = requests.entry(key.to_string()).or_insert_with(Vec::new);

        // Remove timestamps outside the current window
        timestamps.retain(|&t| now - t < self.window_secs);

        // Check if limit exceeded
        if timestamps.len() >= self.limit as usize {
            tracing::warn!("Rate limit exceeded for key: {}", key);
            return false;
        }

        // Add current timestamp
        timestamps.push(now);
        true
    }

    /// Get remaining requests for a key
    pub async fn get_remaining(&self, key: &str) -> u32 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let requests = self.requests.read().await;
        if let Some(timestamps) = requests.get(key) {
            let valid_count = timestamps.iter().filter(|&&t| now - t < self.window_secs).count();
            self.limit.saturating_sub(valid_count as u32)
        } else {
            self.limit
        }
    }

    /// Clean up old entries to prevent memory leaks
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

        tracing::debug!("Rate limiter cleanup completed. Active keys: {}", requests.len());
    }
}

/// Limits requests per IP address using a sliding window algorithm
pub async fn rate_limit_middleware(
    State(rate_limiter): State<Arc<RateLimiter>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let ip = addr.ip().to_string();

    if !rate_limiter.check_rate_limit(&ip).await {
        tracing::warn!("Rate limit exceeded for IP: {}", ip);
        return Err(AppError::RateLimitExceeded);
    }

    let remaining = rate_limiter.get_remaining(&ip).await;
    tracing::debug!("Request from {} - Remaining: {}", ip, remaining);

    Ok(next.run(req).await)
}

/// Start background task to periodically clean up rate limiter storage
pub fn start_cleanup_task(rate_limiter: Arc<RateLimiter>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 minutes
        loop {
            interval.tick().await;
            rate_limiter.cleanup().await;
        }
    });
}
