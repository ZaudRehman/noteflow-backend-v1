// src/handlers/users.rs
use axum::{extract::State, http::StatusCode, Extension, Json};
use std::sync::Arc;

use crate::models::user::*;
use crate::services::UserService;
use crate::utils::errors::Result;

/// GET /api/v1/users/profile
pub async fn get_profile(
    State(user_service): State<Arc<UserService>>,
    Extension(user): Extension<User>,
) -> Result<Json<UserProfile>> {
    let profile = user_service.get_profile(user.id).await?;
    Ok(Json(profile))
}

/// PUT /api/v1/users/profile
pub async fn update_profile(
    State(user_service): State<Arc<UserService>>,
    Extension(user): Extension<User>,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<UserProfile>> {
    let updated = user_service.update_profile(user.id, req).await?;
    tracing::info!("User {} updated profile", user.id);
    Ok(Json(updated))
}

/// PUT /api/v1/users/preferences
pub async fn update_preferences(
    State(user_service): State<Arc<UserService>>,
    Extension(user): Extension<User>,
    Json(req): Json<UpdatePreferencesRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    user_service.update_preferences(user.id, req).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "message": "Preferences updated" })),
    ))
}
