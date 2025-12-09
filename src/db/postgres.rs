use crate::utils::errors::{AppError, Result};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

pub async fn create_pool(database_url: &str, max_connections: u32) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect(database_url)
        .await
        .map_err(|e| AppError::DatabaseError(e))
}

pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| AppError::InternalError(format!("Migration failed: {}", e)))
}

pub async fn run_migrations_if_needed(pool: &PgPool) -> Result<()> {
    // Check if _sqlx_migrations table exists
    let needs_migration = sqlx::query_scalar::<_, bool>(
        "SELECT NOT EXISTS (
            SELECT FROM pg_tables 
            WHERE schemaname = 'public' 
            AND tablename = '_sqlx_migrations'
        )",
    )
    .fetch_one(pool)
    .await?;

    if needs_migration {
        tracing::info!("🔧 First run detected, running migrations...");
        sqlx::migrate!("./migrations")
            .run(pool)
            .await
            .map_err(|e| AppError::InternalError(format!("Migration failed: {}", e)))?;
        tracing::info!("✅ Migrations completed");
    } else {
        tracing::info!("✅ Database already migrated");
    }

    Ok(())
}
