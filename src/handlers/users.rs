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
#[utoipa::path(
    get,
    path = "/api/v1/users/profile",
    tag = "Users",
    responses(
        (status = 200, description = "User profile", body = UserProfile),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn get_profile(
    State(user_service): State<Arc<UserService>>,
    Extension(user): Extension<User>,
) -> Result<Json<UserProfile>> {
    let profile = user_service.get_profile(user.id).await?;
    Ok(Json(profile))
}

/// PUT /api/v1/users/profile
#[utoipa::path(
    put,
    path = "/api/v1/users/profile",
    tag = "Users",
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "Profile updated", body = UserProfile),
    ),
)]
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
#[utoipa::path(
    put,
    path = "/api/v1/users/preferences",
    tag = "Users",
    request_body = UpdatePreferencesRequest,
    responses(
        (status = 200, description = "Preferences updated", body = UserProfile),
    ),
)]
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
#[utoipa::path(
    post,
    path = "/api/v1/users/avatar",
    tag = "Users",
    responses(
        (status = 200, description = "Avatar uploaded", body = UserProfile),
        (status = 400, description = "Validation error"),
    ),
)]
pub async fn upload_avatar(
    State(user_service): State<Arc<UserService>>,
    Extension(user): Extension<User>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<UserProfile>)> {
    let mut image_bytes: Option<Vec<u8>> = None;
    let mut content_type: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::InternalError(format!("Failed to read multipart field: {}", e))
    })? {
        if field.name() == Some("image") {
            content_type = field
                .content_type()
                .map(|m| m.to_string());

            let bytes = field.bytes().await.map_err(|e| {
                AppError::InternalError(format!("Failed to read image data: {}", e))
            })?;

            if bytes.is_empty() {
                return Err(AppError::ValidationError("No image data received".into()));
            }

            if bytes.len() > 5_000_000 {
                return Err(AppError::ValidationError("Image too large (max 5MB)".into()));
            }

            image_bytes = Some(bytes.to_vec());
            break;
        }
    }

    let bytes = image_bytes.ok_or_else(|| {
        AppError::ValidationError("Missing field 'image' in multipart form data".into())
    })?;

    let ct = content_type.unwrap_or_else(|| "image/jpeg".to_string());

    let avatar_url = user_service.upload_avatar(&bytes, &ct, &user.id).await?;

    let profile = user_service
        .update_profile(
            user.id,
            UpdateProfileRequest {
                display_name: None,
                avatar_url: Some(avatar_url.clone()),
            },
        )
        .await?;

    tracing::info!("Avatar uploaded for user {}: {}", user.id, avatar_url);
    Ok((StatusCode::OK, Json(profile)))
}
