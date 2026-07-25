use crate::utils::errors::{AppError, Result};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

/// Create connection pool with retry (exponential backoff: 1s, 2s, 4s, 8s, 16s)
pub async fn create_pool(database_url: &str, max_connections: u32) -> Result<PgPool> {
    let mut attempt = 0;
    let max_attempts = 5;

    loop {
        match PgPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(Duration::from_secs(10))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(1800))
            .connect(database_url)
            .await
        {
            Ok(pool) => return Ok(pool),
            Err(e) => {
                attempt += 1;
                if attempt >= max_attempts {
                    return Err(AppError::DatabaseError(e));
                }
                let delay = Duration::from_secs(1u64 << (attempt - 1));
                tracing::warn!(
                    "Database connection attempt {} failed, retrying in {:?}: {}",
                    attempt,
                    delay,
                    e
                );
                tokio::time::sleep(delay).await;
            }
        }
    }
}

pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| AppError::InternalError(format!("Migration failed: {}", e)))
}

pub async fn run_migrations_if_needed(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| AppError::InternalError(format!("Migration failed: {}", e)))?;
    tracing::info!("✅ Migrations up-to-date");
    Ok(())
}
