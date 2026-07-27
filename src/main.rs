// src/main.rs 
use axum::{
    extract::{Request, State},
    http::HeaderValue,
    middleware,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    limit::RequestBodyLimitLayer,
    trace::TraceLayer,
};
use tracing::Span;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use noteflow_backend::{
    config::Config,
    db::{create_pool, create_redis_client, run_migrations_if_needed, RedisManager},
    handlers,
    middleware::{
        auth_middleware, rate_limit_middleware, request_id_middleware, start_rate_limit_cleanup,
        RateLimiter,
    },
    models,
    services::{
        AuthService, CollaborationService, NoteCollaboratorService, NoteService,
        NotificationService, RevisionService, TagService, UserService,
    },
    utils::jwt::JwtManager,
    VERSION,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Noteflow API",
        description = "Collaborative note-taking API",
        version = "0.1.0",
        contact(name = "Zaud Rehman")
    ),
    paths(
        handlers::auth::register,
        handlers::auth::login,
        handlers::auth::refresh,
        handlers::auth::get_current_user,
        handlers::auth::logout,
        handlers::auth::list_sessions,
        handlers::auth::revoke_session,
        handlers::auth::forgot_password,
        handlers::auth::reset_password,
        handlers::notes::create_note,
        handlers::notes::get_note,
        handlers::notes::list_notes,
        handlers::notes::update_note,
        handlers::notes::delete_note,
        handlers::notes::toggle_favorite,
        handlers::notes::toggle_archive,
        handlers::notes::search,
        handlers::notes::list_notes_filtered,
        handlers::notes::export_note,
        handlers::tags::list_tags,
        handlers::tags::create_tag,
        handlers::tags::get_tag,
        handlers::tags::update_tag,
        handlers::tags::delete_tag,
        handlers::tags::add_tag_to_note,
        handlers::tags::remove_tag_from_note,
        handlers::tags::get_notes_by_tag,
        handlers::collaborators::list_collaborators,
        handlers::collaborators::add_collaborator,
        handlers::collaborators::update_collaborator,
        handlers::collaborators::remove_collaborator,
        handlers::revisions::list_revisions,
        handlers::revisions::get_revision,
        handlers::revisions::restore_revision,
        handlers::users::get_profile,
        handlers::users::update_profile,
        handlers::users::update_preferences,
        handlers::users::upload_avatar,
        handlers::notifications::push_subscribe,
        handlers::notifications::push_unsubscribe,
        handlers::notifications::list_push_subscriptions,
    ),
    components(
        schemas(
            models::note::NoteResponse,
            models::note::CreateNoteRequest,
            models::note::UpdateNoteRequest,
            models::note::NoteListResponse,
            models::note::NoteQueryParams,
            models::note::NoteFilterParams,
            models::note::SearchParams,
            models::note::SearchResponse,
            models::note::ActiveUserInfo,
            models::note::CollaboratorInfo,
            models::note::AddCollaboratorRequest,
            models::note::UpdateCollaboratorRequest,
            models::note::CollaboratorListResponse,
            models::user::UserResponse,
            models::user::RegisterRequest,
            models::user::LoginRequest,
            models::user::AuthResponse,
            models::user::RefreshTokenRequest,
            models::user::UserProfile,
            models::user::UpdateProfileRequest,
            models::user::UpdatePreferencesRequest,
            models::user::ForgotPasswordRequest,
            models::user::ResetPasswordRequest,
            models::user::LogoutRequest,
            models::user::LogoutResponse,
            models::user::SessionInfo,
            models::user::SessionListResponse,
            models::tag::TagResponse,
            models::tag::CreateTagRequest,
            models::tag::UpdateTagRequest,
            models::tag::AddTagRequest,
            models::tag::TagListResponse,
            models::revision::RevisionResponse,
            models::revision::RevisionListResponse,
            models::collaboration::WsMessage,
            models::collaboration::CursorPosition,
        )
    ),
    tags(
        (name = "Authentication", description = "Authentication endpoints"),
        (name = "Notes", description = "Note management endpoints"),
        (name = "Search", description = "Search endpoints"),
        (name = "Notes (Filtered)", description = "Filtered note listing endpoints"),
        (name = "Tags", description = "Tag management endpoints"),
        (name = "Collaborators", description = "Collaborator management endpoints"),
        (name = "Revisions", description = "Note revision history endpoints"),
        (name = "Users", description = "User profile endpoints"),
        (name = "Notifications", description = "Push notification endpoints"),
    ),
)]
pub struct ApiDoc;

#[derive(Clone)]
struct HealthState {
    pool: sqlx::PgPool,
    redis: Arc<RedisManager>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Panic hook: log panics through tracing with structured fields
    std::panic::set_hook(Box::new(|panic_info| {
        let payload = panic_info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| panic_info.payload().downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or("unknown panic");
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown".into());
        tracing::error!(
            panic.message = %payload,
            panic.location = %location,
            "Unhandled panic"
        );
    }));

    // Initialize structured logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "noteflow_backend=debug,tower_http=debug,axum=debug,sqlx=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json().with_target(false).flatten_event(true))
        .init();

    tracing::info!("🚀 Starting NoteFlow Backend v{}", VERSION);

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

    // Redis connection
    tracing::info!("🔴 Connecting to Redis...");
    let redis_conn = create_redis_client(&config.redis_url).await?;
    let redis_manager = Arc::new(RedisManager::new(redis_conn));
    tracing::info!("✅ Redis connected");

    // Initialize JWT manager
    let jwt_manager = Arc::new(JwtManager::new(
        config.jwt_secret.clone(),
        config.jwt_access_expiration,
        config.jwt_refresh_expiration,
    ));
    tracing::info!("🔐 JWT manager initialized");

    // Metrics (Prometheus)
    let prometheus_handle = PrometheusBuilder::new()
        .add_global_label("service", "noteflow-backend")
        .set_buckets(&[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0])
        .unwrap_or_else(|_| PrometheusBuilder::new())
        .install_recorder()
        .expect("Failed to install Prometheus recorder");
    let metrics_handle = Arc::new(prometheus_handle);
    tracing::info!("📊 Metrics initialized");

    // Rate limiters (Redis-backed with in-memory fallback)
    let anonymous_limiter = Arc::new(RateLimiter::new(
        config.rate_limit_anonymous,
        60,
        Some(redis_manager.clone()),
    ));
    let auth_limiter = Arc::new(RateLimiter::new(
        config.rate_limit_authenticated,
        60,
        Some(redis_manager.clone()),
    ));
    // Per-credential login limiter: 5 attempts per 60s per email
    let login_limiter = Arc::new(RateLimiter::new(
        5,
        60,
        Some(redis_manager.clone()),
    ));
    start_rate_limit_cleanup(anonymous_limiter.clone());
    start_rate_limit_cleanup(auth_limiter.clone());
    tracing::info!("✅ Rate limiters initialized");

    // Initialize services
    let notification_service = Arc::new(NotificationService::new(pool.clone(), Arc::new(config.clone())));
    let auth_service = Arc::new(AuthService::new(pool.clone(), jwt_manager.clone(), login_limiter, notification_service.clone()));
    let collab_manage_service = Arc::new(NoteCollaboratorService::new(pool.clone(), config.clone()));
    let note_service = Arc::new(NoteService::new(pool.clone(), config.clone(), collab_manage_service.clone(), notification_service.clone()));
    let tag_service = Arc::new(TagService::new(pool.clone(), collab_manage_service.clone()));
    let user_service = Arc::new(UserService::new(pool.clone(), config.clone()));
    let revision_service = Arc::new(RevisionService::new(pool.clone()));
    let collab_service = CollaborationService::new(
        pool.clone(),
        Some(redis_manager.clone()),
        &config.redis_url,
        config.max_ws_connections,
    );
    collab_service.spawn_background_tasks();
    tracing::info!("✅ All services initialized");

    // Parse CORS allowed origins
    let cors_origins: Vec<HeaderValue> = config
        .cors_allowed_origins
        .iter()
        .filter_map(|origin| origin.parse::<HeaderValue>().ok())
        .collect();
    
    tracing::info!("🌐 CORS allowed origins: {:?}", config.cors_allowed_origins);

    // === HEALTH ROUTE (separate state) ===
    let health_state = HealthState {
        pool: pool.clone(),
        redis: redis_manager.clone(),
    };
    let health_route = Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(metrics_handler))
        .with_state(health_state);

    // === PUBLIC ROUTES ===
    let public_routes = Router::new()
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
        .route("/api/v1/auth/sessions", get(handlers::auth::list_sessions))
        .route("/api/v1/auth/sessions/:session_id", delete(handlers::auth::revoke_session))
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
        .route("/api/v1/notes/:id/export", get(handlers::notes::export_note))
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
        .route("/api/v1/users/avatar", post(handlers::users::upload_avatar))
        .with_state(user_service);

    // === SEARCH ROUTE ===
    let search_route = Router::new()
        .route("/api/v1/search", get(handlers::notes::search))
        .with_state(note_service);

    // === NOTIFICATION ROUTES ===
    let notification_routes = Router::new()
        .route("/api/v1/notifications/push/subscriptions", get(handlers::notifications::list_push_subscriptions))
        .route("/api/v1/notifications/push/subscribe", post(handlers::notifications::push_subscribe))
        .route("/api/v1/notifications/push/subscribe/:id", delete(handlers::notifications::push_unsubscribe))
        .with_state(notification_service);

    // === REVISION HISTORY ROUTES ===
    let revision_routes = Router::new()
        .route("/api/v1/notes/:note_id/history", get(handlers::revisions::list_revisions))
        .route("/api/v1/notes/:note_id/history/:revision_id", get(handlers::revisions::get_revision))
        .route("/api/v1/notes/:note_id/history/:revision_id/restore", post(handlers::revisions::restore_revision))
        .with_state(revision_service);

    // === WEBSOCKET COLLABORATION ===
    let ws_route = Router::new()
        .route("/api/v1/notes/:id/ws", get(handlers::websocket::note_websocket_handler))
        .with_state(collab_service);

    // === COLLABORATOR ROUTES ===
    let collaborator_routes = Router::new()
        .route("/api/v1/notes/:note_id/collaborators", get(handlers::collaborators::list_collaborators))
        .route("/api/v1/notes/:note_id/collaborators", post(handlers::collaborators::add_collaborator))
        .route("/api/v1/notes/:note_id/collaborators/:target_user_id", put(handlers::collaborators::update_collaborator))
        .route("/api/v1/notes/:note_id/collaborators/:target_user_id", delete(handlers::collaborators::remove_collaborator))
        .with_state(collab_manage_service);

    // === COMBINE PROTECTED ROUTES ===
    let protected_routes = Router::new()
        .merge(auth_routes)
        .merge(note_routes)
        .merge(tag_routes)
        .merge(user_routes)
        .merge(search_route)
        .merge(notification_routes)
        .merge(revision_routes)
        .merge(ws_route)
        .merge(collaborator_routes)
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
        .merge(health_route)
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(public_routes)
        .merge(protected_routes)
        // Metrics handle
        .layer(Extension(metrics_handle))
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
                .allow_credentials(true)
                .max_age(std::time::Duration::from_secs(86400)),
        )
        // Request body size limit (5MB)
        .layer(RequestBodyLimitLayer::new(
            5 * 1024 * 1024,
        ))
        // Compression
        .layer(CompressionLayer::new())
        // Request ID middleware (creates tracing span with request_id, sets X-Request-Id header)
        .layer(middleware::from_fn(request_id_middleware))
        // Tracing with structured spans
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    tracing::info_span!(
                        "http_request",
                        request_id = tracing::field::Empty,
                        method = %request.method(),
                        uri = %request.uri().path_and_query().map(|pq| pq.as_str()).unwrap_or(""),
                        status_code = tracing::field::Empty,
                    )
                })
                .on_response(|response: &axum::http::Response<_>, _latency: Duration, span: &Span| {
                    span.record("status_code", &response.status().as_u16());
                }),
        );

    // Bind server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("🌐 Server listening on http://{}", addr);
    print_api_endpoints();

    // Start self-ping to prevent Render free-tier sleep
    start_self_ping(config.self_url.clone());
    tracing::info!("✨ NoteFlow Backend ready!");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    tracing::info!("👋 Server shutdown complete");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { tracing::info!("Received Ctrl+C, starting graceful shutdown..."); }
        _ = terminate => { tracing::info!("Received SIGTERM, starting graceful shutdown..."); }
    }
}

async fn metrics_handler(
    Extension(handle): Extension<Arc<PrometheusHandle>>,
) -> String {
    handle.render()
}

async fn health_check(
    State(state): State<HealthState>,
) -> impl IntoResponse {
    let db_ok = sqlx::query("SELECT 1")
        .execute(&state.pool)
        .await
        .is_ok();
    let redis_ok = redis::cmd("PING")
        .query_async::<_, String>(&mut *state.redis.conn.write().await)
        .await
        .is_ok();

    let status = if db_ok && redis_ok { "ok" } else { "degraded" };
    let http_status = if db_ok { 200 } else { 503 };

    (
        axum::http::StatusCode::from_u16(http_status).unwrap(),
        Json(json!({
            "status": status,
            "version": VERSION,
            "database": db_ok,
            "redis": redis_ok,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })),
    )
}

fn start_self_ping(self_url: String) {
    if self_url.is_empty() || self_url.contains("localhost") || self_url.contains("127.0.0.1") {
        tracing::info!("⏭️  Self-ping disabled (local URL: {})", self_url);
        return;
    }

    let health_url = format!("{}/health", self_url.trim_end_matches('/'));
    tracing::info!("⏰ Starting self-ping every 10 minutes to {}", health_url);

    tokio::spawn(async move {
        // Initial delay to let the server start
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;

        loop {
            match reqwest::get(&health_url).await {
                Ok(resp) => {
                    let status = resp.status();
                    tracing::debug!("Self-ping to {} returned {}", health_url, status);
                }
                Err(e) => {
                    tracing::warn!("Self-ping to {} failed: {}", health_url, e);
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(600)).await;
        }
    });
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
    tracing::info!("  GET    /api/v1/auth/sessions");
    tracing::info!("  DELETE /api/v1/auth/sessions/:session_id");
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
    tracing::info!("  === Users (Protected) ===");
    tracing::info!("  GET    /api/v1/users/profile");
    tracing::info!("  PUT    /api/v1/users/profile");
    tracing::info!("  PUT    /api/v1/users/preferences");
    tracing::info!("  POST   /api/v1/users/avatar");
    tracing::info!("  === Notifications (Protected) ===");
    tracing::info!("  GET    /api/v1/notifications/push/subscriptions");
    tracing::info!("  POST   /api/v1/notifications/push/subscribe");
    tracing::info!("  DELETE /api/v1/notifications/push/subscribe/:id");
    tracing::info!("  === Search (Protected) ===");
    tracing::info!("  GET    /api/v1/search?q=query");
    tracing::info!("  === Revisions (Protected) ===");
    tracing::info!("  GET    /api/v1/notes/:note_id/history");
    tracing::info!("  GET    /api/v1/notes/:note_id/history/:revision_id");
    tracing::info!("  POST   /api/v1/notes/:note_id/history/:revision_id/restore");
    tracing::info!("  === Collaborators (Protected) ===");
    tracing::info!("  GET    /api/v1/notes/:note_id/collaborators");
    tracing::info!("  POST   /api/v1/notes/:note_id/collaborators");
    tracing::info!("  PUT    /api/v1/notes/:note_id/collaborators/:target_user_id");
    tracing::info!("  DELETE /api/v1/notes/:note_id/collaborators/:target_user_id");
    tracing::info!("  === WebSocket (Protected) ===");
    tracing::info!("  WS     /api/v1/notes/:id/ws");
}
