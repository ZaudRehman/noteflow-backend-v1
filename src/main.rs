// src/main.rs 
use axum::{
    http::HeaderValue,
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use noteflow_backend::{
    config::Config,
    db::{create_pool, create_redis_client, run_migrations_if_needed},
    handlers,
    middleware::{auth_middleware, rate_limit_middleware, start_cleanup_task, RateLimiter},
    services::{AuthService, NoteService, TagService, UserService, CollaborationService},
    utils::jwt::JwtManager,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "noteflow_backend=debug,tower_http=debug,axum=debug,sqlx=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    tracing::info!("🚀 Starting NoteFlow Backend v1.0.0");

    // Load environment variables
    dotenvy::dotenv().ok();
    let config = Config::from_env().expect("Failed to load configuration");
    tracing::info!("✅ Configuration loaded");

    // Database connection with verification
    tracing::info!("📊 Connecting to PostgreSQL...");
    let pool = create_pool(&config.database_url, config.database_max_connections).await?;
    // Verify database connection
    sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .expect("Failed to verify database connection");
    tracing::info!(
        "✅ PostgreSQL connected ({} max connections)",
        config.database_max_connections
    );

    // Run migrations
    run_migrations_if_needed(&pool).await?;
    tracing::info!("✅ Database migrations complete");

    // Redis connection (optional for MVP)
    tracing::info!("🔴 Connecting to Redis...");
    let _redis_client = create_redis_client(&config.redis_url).await?;
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
    let tag_service = Arc::new(TagService::new(pool.clone()));
    let user_service = Arc::new(UserService::new(pool.clone()));
    let collab_service = CollaborationService::new(pool.clone());
    tracing::info!("✅ All services initialized");

    // Rate limiters
    let anonymous_limiter = Arc::new(RateLimiter::new(config.rate_limit_anonymous, 60));
    let auth_limiter = Arc::new(RateLimiter::new(config.rate_limit_authenticated, 60));
    start_cleanup_task(anonymous_limiter.clone());
    start_cleanup_task(auth_limiter.clone());
    tracing::info!("✅ Rate limiters initialized");

    // Parse CORS allowed origins
    let cors_origins: Vec<HeaderValue> = config
        .cors_allowed_origins
        .iter()
        .filter_map(|origin| origin.parse::<HeaderValue>().ok())
        .collect();
    
    tracing::info!("🌐 CORS allowed origins: {:?}", config.cors_allowed_origins);

    // === PUBLIC ROUTES ===
    let public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/auth/register", post(handlers::auth::register))
        .route("/api/v1/auth/login", post(handlers::auth::login))
        .route("/api/v1/auth/refresh", post(handlers::auth::refresh))
        .route("/api/v1/auth/forgot-password", post(handlers::auth::forgot_password))
        .route("/api/v1/auth/reset-password", post(handlers::auth::reset_password))
        .with_state(auth_service.clone())
        .layer(middleware::from_fn_with_state(
            anonymous_limiter.clone(),
            rate_limit_middleware,
        ));

    // === PROTECTED AUTH ROUTES ===
    let auth_routes = Router::new()
        .route("/api/v1/auth/me", get(handlers::auth::get_current_user))
        .route("/api/v1/auth/logout", post(handlers::auth::logout))
        .with_state(auth_service);

    // === NOTE ROUTES ===
    let note_routes = Router::new()
        .route("/api/v1/notes", get(handlers::notes::list_notes_filtered))
        .route("/api/v1/notes", post(handlers::notes::create_note))
        .route("/api/v1/notes/:id", get(handlers::notes::get_note))
        .route("/api/v1/notes/:id", put(handlers::notes::update_note))
        .route("/api/v1/notes/:id", delete(handlers::notes::delete_note))
        .route("/api/v1/notes/:id/favorite", post(handlers::notes::toggle_favorite))
        .route("/api/v1/notes/:id/archive", post(handlers::notes::toggle_archive))
        .with_state(note_service.clone());

    // === TAG ROUTES ===
    let tag_routes = Router::new()
        .route("/api/v1/tags", get(handlers::tags::list_tags))
        .route("/api/v1/tags", post(handlers::tags::create_tag))
        .route("/api/v1/tags/:id", get(handlers::tags::get_tag))
        .route("/api/v1/tags/:id", put(handlers::tags::update_tag))
        .route("/api/v1/tags/:id", delete(handlers::tags::delete_tag))
        .route("/api/v1/tags/:id/notes", get(handlers::tags::get_notes_by_tag))
        .route("/api/v1/notes/:note_id/tags", post(handlers::tags::add_tag_to_note))
        .route(
            "/api/v1/notes/:note_id/tags/:tag_id",
            delete(handlers::tags::remove_tag_from_note),
        )
        .with_state(tag_service);

    // === USER/PROFILE ROUTES ===
    let user_routes = Router::new()
        .route("/api/v1/users/profile", get(handlers::users::get_profile))
        .route("/api/v1/users/profile", put(handlers::users::update_profile))
        .route("/api/v1/users/preferences", put(handlers::users::update_preferences))
        .with_state(user_service);

    // === SEARCH ROUTE ===
    let search_route = Router::new()
        .route("/api/v1/search", get(handlers::notes::search))
        .with_state(note_service);

    // === WEBSOCKET COLLABORATION ===
    let ws_route = Router::new()
        .route("/api/v1/notes/:id/ws", get(handlers::websocket::note_websocket_handler))
        .with_state(collab_service);

    // === COMBINE PROTECTED ROUTES ===
    let protected_routes = Router::new()
        .merge(auth_routes)
        .merge(note_routes)
        .merge(tag_routes)
        .merge(user_routes)
        .merge(search_route)
        .merge(ws_route)
        .layer(middleware::from_fn_with_state(
            (jwt_manager.clone(), pool.clone()),
            auth_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            auth_limiter.clone(),
            rate_limit_middleware,
        ));

    // === FINAL APP ===
    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        // CORS with specific origins
        .layer(
            CorsLayer::new()
                .allow_origin(cors_origins)
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::PUT,
                    axum::http::Method::PATCH,
                    axum::http::Method::DELETE,
                    axum::http::Method::OPTIONS,
                ])
                .allow_headers([
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::ACCEPT,
                ])
                .allow_credentials(true),
        )
        // Compression
        .layer(CompressionLayer::new())
        // Tracing
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        );

    // Bind server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("🌐 Server listening on http://{}", addr);
    print_api_endpoints();
    tracing::info!("✨ NoteFlow Backend ready!");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}

fn print_api_endpoints() {
    tracing::info!("📝 Available API Endpoints:");
    tracing::info!("  === Public ===");
    tracing::info!("  GET    /health");
    tracing::info!("  POST   /api/v1/auth/register");
    tracing::info!("  POST   /api/v1/auth/login");
    tracing::info!("  POST   /api/v1/auth/refresh");
    tracing::info!("  POST   /api/v1/auth/forgot-password");
    tracing::info!("  POST   /api/v1/auth/reset-password");
    tracing::info!("  === Auth (Protected) ===");
    tracing::info!("  GET    /api/v1/auth/me");
    tracing::info!("  POST   /api/v1/auth/logout");
    tracing::info!("  === Notes (Protected) ===");
    tracing::info!("  GET    /api/v1/notes");
    tracing::info!("  POST   /api/v1/notes");
    tracing::info!("  GET    /api/v1/notes/:id");
    tracing::info!("  PUT    /api/v1/notes/:id");
    tracing::info!("  DELETE /api/v1/notes/:id");
    tracing::info!("  POST   /api/v1/notes/:id/favorite");
    tracing::info!("  POST   /api/v1/notes/:id/archive");
    tracing::info!("  === Tags (Protected) ===");
    tracing::info!("  GET    /api/v1/tags");
    tracing::info!("  POST   /api/v1/tags");
    tracing::info!("  PUT    /api/v1/tags/:id");
    tracing::info!("  DELETE /api/v1/tags/:id");
    tracing::info!("  GET    /api/v1/tags/:id/notes");
    tracing::info!("  POST   /api/v1/notes/:note_id/tags");
    tracing::info!("  DELETE /api/v1/notes/:note_id/tags/:tag_id");
    tracing::info!("  === User (Protected) ===");
    tracing::info!("  GET    /api/v1/users/profile");
    tracing::info!("  PUT    /api/v1/users/profile");
    tracing::info!("  PUT    /api/v1/users/preferences");
    tracing::info!("  === Search (Protected) ===");
    tracing::info!("  GET    /api/v1/search?q=query");
    tracing::info!("  === WebSocket (Protected) ===");
    tracing::info!("  WS     /api/v1/notes/:id/ws");
}
