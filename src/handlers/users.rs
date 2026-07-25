// src/handlers/users.rs
use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    Extension, Json,
};
use std::sync::Arc;

use crate::models::user::*;
use crate::services::UserService;
use crate::utils::errors::{AppError, Result};

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
) -> Result<Json<UserProfile>> {
    let profile = user_service.update_preferences(user.id, req).await?;
    Ok(Json(profile))
}

/// POST /api/v1/users/avatar - Upload avatar image
/// Accepts multipart/form-data with field "image" (file) or JSON with "image" (base64)
pub async fn upload_avatar(
    State(user_service): State<Arc<UserService>>,
    Extension(user): Extension<User>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<UserProfile>)> {
    let mut image_data: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::InternalError(format!("Failed to read multipart field: {}", e))
    })? {
        if field.name() == Some("image") {
            let content_type = field
                .content_type()
                .map(|m| m.to_string())
                .unwrap_or_default();

            let bytes = field.bytes().await.map_err(|e| {
                AppError::InternalError(format!("Failed to read image data: {}", e))
            })?;

            if bytes.is_empty() {
                return Err(AppError::ValidationError("No image data received".into()));
            }

            if bytes.len() > 5_000_000 {
                return Err(AppError::ValidationError("Image too large (max 5MB)".into()));
            }

            // Determine MIME prefix
            let mime_prefix = if content_type.contains("png") {
                "data:image/png;base64,"
            } else if content_type.contains("gif") {
                "data:image/gif;base64,"
            } else if content_type.contains("webp") {
                "data:image/webp;base64,"
            } else {
                "data:image/jpeg;base64,"
            };

            let b64 = base64::engine::general_purpose::STANDARD;
            let encoded = base64::Engine::encode(&b64, &bytes);
            image_data = Some(format!("{}{}", mime_prefix, encoded));
            break;
        }
    }

    let raw = image_data.ok_or_else(|| {
        AppError::ValidationError(
            "Missing field 'image' in multipart form data".into(),
        )
    })?;

    let avatar_url = user_service.upload_avatar(&raw, &user.id).await?;

    let profile = user_service
        .update_profile(
            user.id,
            UpdateProfileRequest {
                display_name: None,
                avatar_url: Some(avatar_url),
            },
        )
        .await?;

    tracing::info!("Avatar updated for user {}", user.id);
    Ok((StatusCode::OK, Json(profile)))
}
