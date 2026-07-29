use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::RedisManager;
use crate::models::user::User;
use crate::utils::{errors::AppError, jwt::JwtManager};
use tracing::Span;

/// Middleware to authenticate requests using JWT tokens or WS tickets.
/// State is (JwtManager, PgPool, Option<Arc<RedisManager>>).
/// For WebSocket upgrades without an Authorization header, a short-lived
/// ticket from ?ticket= is validated against Redis instead of a raw JWT.
pub async fn auth_middleware(
    State((jwt_manager, pool, redis)): State<(Arc<JwtManager>, PgPool, Option<Arc<RedisManager>>)>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let path = req.uri().path();
    let method = req.method();

    // 🔥 FIX: Skip auth for public routes
    let public_routes = vec![
        "/health",
        "/auth/register",
        "/auth/login",
        "/auth/refresh",
        "/api/v1/auth/register",
        "/api/v1/auth/login",
        "/api/v1/auth/refresh",
    ];

    if public_routes
        .iter()
        .any(|route| path == *route || path.starts_with(route))
    {
        tracing::debug!("Public route, skipping auth: {}", path);
        return Ok(next.run(req).await);
    }

    // 🔥 FIX: Skip auth for OPTIONS requests (CORS preflight)
    if method == "OPTIONS" {
        tracing::debug!("OPTIONS request, skipping auth");
        return Ok(next.run(req).await);
    }

    tracing::debug!("Protected route, checking auth: {}", path);

    // -- Extract auth material: header first, then query string --
    let (auth_method, token_or_ticket): (&str, String) = match req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
    {
        Some(t) => ("jwt", t.to_string()),
        None => {
            // Check query string for "ticket=" (short-lived WS ticket) or
            // fall back to "token=" (legacy, kept temporarily for migration).
            let q = req.uri().query().unwrap_or("");
            q.split('&')
                .find(|p| p.starts_with("ticket="))
                .map(|p| ("ticket", p["ticket=".len()..].to_string()))
                .or_else(|| {
                    q.split('&')
                        .find(|p| p.starts_with("token="))
                        .map(|p| ("token", p["token=".len()..].to_string()))
                })
                .ok_or_else(|| {
                    tracing::warn!("Missing or invalid Authorization header/param for: {}", path);
                    AppError::AuthenticationError("Missing authorization token".to_string())
                })?
        }
    };

    // -- Resolve user from JWT or ticket --
    let user_id = match auth_method {
        "jwt" | "token" => {
            let claims = jwt_manager.verify_access_token(&token_or_ticket).map_err(|e| {
                tracing::warn!("Token verification failed: {}", e);
                e
            })?;
            Uuid::parse_str(&claims.sub).map_err(|_| {
                tracing::error!("Invalid user ID format in token: {}", claims.sub);
                AppError::AuthenticationError("Invalid user ID in token".to_string())
            })?
        }
        "ticket" => {
            match &redis {
                Some(redis) => {
                    let key = format!("ws_ticket:{}", token_or_ticket);
                    let val = redis.get(&key).await?.ok_or_else(|| {
                        tracing::warn!("Invalid or expired ticket for: {}", path);
                        AppError::AuthenticationError("Invalid or expired ticket".to_string())
                    })?;
                    // value is "user_id:note_id"
                    let uid = val.split(':').next().unwrap_or("").to_string();
                    redis.delete(&key).await?; // single-use
                    Uuid::parse_str(&uid).map_err(|_| {
                        tracing::error!("Invalid user ID in ticket payload");
                        AppError::AuthenticationError("Invalid ticket payload".to_string())
                    })?
                }
                None => {
                    tracing::warn!("Redis not available — cannot validate ticket for: {}", path);
                    return Err(AppError::AuthenticationError(
                        "Ticket auth requires Redis".to_string(),
                    ));
                }
            }
        }
        _ => unreachable!(),
    };

    // Fetch user from database
    let user = sqlx::query_as!(
        User,
        r#"SELECT 
            id, email, password_hash, display_name, 
            preferences, theme, avatar_url, 
            reset_token, reset_token_expires, last_login_at,
            created_at, updated_at 
        FROM users WHERE id = $1"#,
        user_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| {
        tracing::warn!("User not found for ID: {}", user_id);
        AppError::AuthenticationError("User not found".to_string())
    })?;

    tracing::debug!("Authenticated user: {} ({})", user.email, user.id);
    Span::current().record("user_id", &tracing::field::display(user.id));

    req.extensions_mut().insert(user);

    Ok(next.run(req).await)
}

pub async fn optional_auth_middleware(
    State((jwt_manager, pool)): State<(Arc<JwtManager>, PgPool)>,
    mut req: Request,
    next: Next,
) -> Response {
    if let Some(auth_header) = req.headers().get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                if let Ok(claims) = jwt_manager.verify_access_token(token) {
                    if let Ok(user_id) = Uuid::parse_str(&claims.sub) {
                        if let Ok(Some(user)) = sqlx::query_as::<_, User>(
                            "SELECT id, email, password_hash, display_name, avatar_url, theme, preferences, created_at, updated_at, last_login_at, reset_token, reset_token_expires FROM users WHERE id = $1"
                        )
                        .bind(user_id)
                        .fetch_optional(&pool)
                        .await
                        {
                            req.extensions_mut().insert(user);
                        }
                    }
                }
            }
        }
    }

    next.run(req).await
}
