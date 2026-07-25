// src/handlers/auth.rs
use crate::models::user::{
    AuthResponse, ForgotPasswordRequest, LoginRequest, LogoutRequest, LogoutResponse,
    RefreshTokenRequest, RegisterRequest, ResetPasswordRequest, SessionListResponse, User,
    UserProfile,
};
use crate::services::AuthService;
use crate::utils::errors::Result;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Extension, Json,
};
use chrono::Utc;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

/// POST /api/v1/auth/register - Register new user
pub async fn register(
    State(auth_service): State<Arc<AuthService>>,
    headers: HeaderMap,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>)> {
    let response = auth_service.register(req).await?;

    // Extract user agent and IP for token tracking
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let ip_address = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    // Store refresh token
    let expires_at = Utc::now() + chrono::Duration::seconds(2592000); // 30 days
    auth_service
        .store_refresh_token(
            response.user.id,
            &response.refresh_token,
            expires_at,
            user_agent,
            ip_address,
        )
        .await?;

    tracing::info!(
        "New user registered: {} ({})",
        response.user.email,
        response.user.id
    );

    Ok((StatusCode::CREATED, Json(response)))
}

/// POST /api/v1/auth/login - Authenticate user
pub async fn login(
    State(auth_service): State<Arc<AuthService>>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>> {
    let response = auth_service.login(req).await?;

    // Extract user agent and IP for token tracking
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let ip_address = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    // Store refresh token
    let expires_at = Utc::now() + chrono::Duration::seconds(2592000); // 30 days
    auth_service
        .store_refresh_token(
            response.user.id,
            &response.refresh_token,
            expires_at,
            user_agent,
            ip_address,
        )
        .await?;

    tracing::info!(
        "User logged in: {} ({})",
        response.user.email,
        response.user.id
    );

    Ok(Json(response))
}

/// POST /api/v1/auth/refresh - Refresh access token
pub async fn refresh(
    State(auth_service): State<Arc<AuthService>>,
    Json(req): Json<RefreshTokenRequest>,
) -> Result<Json<serde_json::Value>> {
    // Verify token is not revoked
    auth_service
        .verify_refresh_token_not_revoked(&req.refresh_token)
        .await?;

    let (access_token, refresh_token) = auth_service.refresh_token(&req.refresh_token).await?;

    Ok(Json(serde_json::json!({
        "access_token": access_token,
        "refresh_token": refresh_token
    })))
}

/// GET /api/v1/auth/me - Get current user profile
pub async fn get_current_user(
    Extension(user): Extension<User>,
    State(auth_service): State<Arc<AuthService>>,
) -> Result<Json<UserProfile>> {
    let profile = auth_service.get_current_user(user.id).await?;
    Ok(Json(profile))
}

/// POST /api/v1/auth/logout - Revoke refresh token
pub async fn logout(
    State(auth_service): State<Arc<AuthService>>,
    Extension(user): Extension<User>,
    Json(req): Json<LogoutRequest>,
) -> Result<Json<LogoutResponse>> {
    auth_service
        .revoke_refresh_token(&req.refresh_token)
        .await?;

    tracing::info!("User {} logged out", user.id);

    Ok(Json(LogoutResponse {
        message: "Successfully logged out".into(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct SessionListQuery {
    pub current_token: Option<String>,
}

/// GET /api/v1/auth/sessions - List active sessions
pub async fn list_sessions(
    State(auth_service): State<Arc<AuthService>>,
    Extension(user): Extension<User>,
    Query(query): Query<SessionListQuery>,
) -> Result<Json<SessionListResponse>> {
    let current_hash = query
        .current_token
        .map(|t| {
            let mut hasher = Sha256::new();
            hasher.update(t.as_bytes());
            format!("{:x}", hasher.finalize())
        })
        .unwrap_or_default();

    let sessions = auth_service.list_sessions(user.id, &current_hash).await?;
    Ok(Json(sessions))
}

/// DELETE /api/v1/auth/sessions/:session_id - Revoke a specific session
pub async fn revoke_session(
    State(auth_service): State<Arc<AuthService>>,
    Extension(user): Extension<User>,
    Path(session_id): Path<Uuid>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    auth_service.revoke_session(session_id, user.id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "message": "Session revoked" })),
    ))
}

/// POST /api/v1/auth/forgot-password - Request password reset
pub async fn forgot_password(
    State(auth_service): State<Arc<AuthService>>,
    Json(req): Json<ForgotPasswordRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let message = auth_service.create_password_reset_token(&req.email).await?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "message": message })),
    ))
}

/// POST /api/v1/auth/reset-password - Reset password with token
pub async fn reset_password(
    State(auth_service): State<Arc<AuthService>>,
    Json(req): Json<ResetPasswordRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    auth_service
        .reset_password(&req.token, &req.new_password)
        .await?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "message": "Password reset successful" })),
    ))
}
