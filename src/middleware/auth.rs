use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::user::User;
use crate::utils::{errors::AppError, jwt::JwtManager};

/// Middleware to authenticate requests using JWT tokens
/// Extracts the Bearer token from Authorization header, verifies it,fetches the user from database and injects user into request extensions
pub async fn auth_middleware(
    State((jwt_manager, pool)): State<(Arc<JwtManager>, PgPool)>,
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
    
    // Check if current path is public
    if public_routes.iter().any(|route| path == *route || path.starts_with(route)) {
        tracing::debug!("Public route, skipping auth: {}", path);
        return Ok(next.run(req).await);
    }
    
    // 🔥 FIX: Skip auth for OPTIONS requests (CORS preflight)
    if method == "OPTIONS" {
        tracing::debug!("OPTIONS request, skipping auth");
        return Ok(next.run(req).await);
    }
    
    tracing::debug!("Protected route, checking auth: {}", path);
    
    // Extract token from Authorization header
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| {
            tracing::warn!("Missing or invalid Authorization header for: {}", path);
            AppError::AuthenticationError("Missing authorization token".to_string())
        })?;
    
    // Verify JWT token
    let claims = jwt_manager.verify_access_token(token).map_err(|e| {
        tracing::warn!("Token verification failed: {}", e);
        e
    })?;
    
    // Parse user ID from claims
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        tracing::error!("Invalid user ID format in token: {}", claims.sub);
        AppError::AuthenticationError("Invalid user ID in token".to_string())
    })?;
    
    // Fetch user from database
    let user = sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE id = $1",
        user_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| {
        tracing::warn!("User not found for ID: {}", user_id);
        AppError::AuthenticationError("User not found".to_string())
    })?;
    
    tracing::debug!("Authenticated user: {} ({})", user.email, user.id);
    
    // Insert user into request extensions for handlers to access
    req.extensions_mut().insert(user);
    
    // Continue to next middleware/handler
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
                        if let Ok(Some(user)) = sqlx::query_as!(
                            User,
                            "SELECT * FROM users WHERE id = $1",
                            user_id
                        )
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