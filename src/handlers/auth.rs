use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;
use crate::models::user::{RegisterRequest, LoginRequest, RefreshTokenRequest, AuthResponse};
use crate::services::AuthService;
use crate::utils::errors::Result;

pub async fn register(
    State(auth_service): State<Arc<AuthService>>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>)> {
    let response = auth_service.register(req).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn login(
    State(auth_service): State<Arc<AuthService>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>> {
    let response = auth_service.login(req).await?;
    Ok(Json(response))
}

pub async fn refresh(
    State(auth_service): State<Arc<AuthService>>,
    Json(req): Json<RefreshTokenRequest>,
) -> Result<Json<serde_json::Value>> {
    let (access_token, refresh_token) = auth_service.refresh_token(&req.refresh_token).await?;
    Ok(Json(serde_json::json!({
        "access_token": access_token,
        "refresh_token": refresh_token
    })))
}