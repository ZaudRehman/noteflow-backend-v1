use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use noteflow_backend::{
    config::Config,
    db::{create_pool, create_redis_client, run_migrations_if_needed},
    handlers,
    middleware::{auth_middleware, rate_limit_middleware, start_cleanup_task, RateLimiter},
    services::{AuthService, NoteService},
    utils::jwt::JwtManager,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "noteflow_backend=debug,tower_http=debug,sqlx=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🚀 Starting NoteFlow Backend...");

    // Load configuration from environment
    dotenvy::dotenv().ok();
    let config = Config::from_env().expect("Failed to load configuration from environment");
    tracing::info!("✅ Configuration loaded");

    // Create database connection pool
    tracing::info!("📊 Connecting to PostgreSQL...");
    let pool = create_pool(&config.database_url, config.database_max_connections).await?;
    tracing::info!(
        "✅ PostgreSQL connected with {} max connections",
        config.database_max_connections
    );

    // Run database migrations if needed
    run_migrations_if_needed(&pool).await?;

    // Create Redis connection
    tracing::info!("🔴 Connecting to Redis...");
    let _redis_conn = create_redis_client(&config.redis_url).await?;
    tracing::info!("✅ Redis connected");

    // Initialize JWT manager
    let jwt_manager = Arc::new(JwtManager::new(
        config.jwt_secret.clone(),
        config.jwt_access_expiration,
        config.jwt_refresh_expiration,
    ));
    tracing::info!("🔐 JWT manager initialized");

    // Initialize services
    let auth_service = Arc::new(AuthService::new(pool.clone(), jwt_manager.clone()));
    let note_service = Arc::new(NoteService::new(pool.clone(), config.clone()));
    tracing::info!("✅ Services initialized");

    // Initialize rate limiters
    let anonymous_rate_limiter = Arc::new(RateLimiter::new(
        config.rate_limit_anonymous,
        60, // 60 seconds window
    ));
    let authenticated_rate_limiter = Arc::new(RateLimiter::new(
        config.rate_limit_authenticated,
        60,
    ));

    // Start rate limiter cleanup tasks
    start_cleanup_task(anonymous_rate_limiter.clone());
    start_cleanup_task(authenticated_rate_limiter.clone());
    tracing::info!("✅ Rate limiters initialized");

    // Build public routes with /api/v1 prefix
    let public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/auth/register", post(handlers::auth::register))
        .route("/api/v1/auth/login", post(handlers::auth::login))
        .route("/api/v1/auth/refresh", post(handlers::auth::refresh))
        .with_state(auth_service)
        .layer(middleware::from_fn_with_state(
            anonymous_rate_limiter.clone(),
            rate_limit_middleware,
        ));

    // Build protected routes with /api/v1 prefix
    let protected_routes = Router::new()
        .route("/api/v1/notes", get(handlers::notes::list_notes))
        .route("/api/v1/notes", post(handlers::notes::create_note))
        .route("/api/v1/notes/:id", get(handlers::notes::get_note))
        .route("/api/v1/notes/:id", put(handlers::notes::update_note))
        .route("/api/v1/notes/:id", delete(handlers::notes::delete_note))
        .with_state(note_service)
        .layer(middleware::from_fn_with_state(
            (jwt_manager.clone(), pool.clone()),
            auth_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            authenticated_rate_limiter.clone(),
            rate_limit_middleware,
        ));

    // Combine all routes
    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        // 🔥 FIXED CORS: Specify explicit headers when using credentials
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers([
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::ACCEPT,
                ])
                .allow_credentials(true),
        )
        // Compression layer
        .layer(CompressionLayer::new())
        // Tracing/logging layer
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        );

    // Bind server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("🌐 Server listening on {}", addr);
    tracing::info!("📝 API Documentation:");
    tracing::info!("  - GET  /health                   - Health check");
    tracing::info!("  - POST /api/v1/auth/register     - Register new user");
    tracing::info!("  - POST /api/v1/auth/login        - User login");
    tracing::info!("  - POST /api/v1/auth/refresh      - Refresh access token");
    tracing::info!("  - GET  /api/v1/notes             - List notes (auth required)");
    tracing::info!("  - POST /api/v1/notes             - Create note (auth required)");
    tracing::info!("  - GET  /api/v1/notes/:id         - Get note (auth required)");
    tracing::info!("  - PUT  /api/v1/notes/:id         - Update note (auth required)");
    tracing::info!("  - DELETE /api/v1/notes/:id       - Delete note (auth required)");
    tracing::info!("✨ Server ready to accept connections!");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}
