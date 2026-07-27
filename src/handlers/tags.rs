// src/handlers/tags.rs
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::note::NoteListResponse;
use crate::models::{tag::*, user::User};
use crate::services::TagService;
use crate::utils::errors::Result;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct TagNotesQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

/// GET /api/v1/tags
#[utoipa::path(
    get,
    path = "/api/v1/tags",
    tag = "Tags",
    responses(
        (status = 200, description = "List of tags", body = TagListResponse),
    ),
)]
pub async fn list_tags(
    State(tag_service): State<Arc<TagService>>,
    Extension(user): Extension<User>,
) -> Result<Json<TagListResponse>> {
    let tags = tag_service.list(user.id).await?;
    Ok(Json(tags))
}

/// POST /api/v1/tags
#[utoipa::path(
    post,
    path = "/api/v1/tags",
    tag = "Tags",
    request_body = CreateTagRequest,
    responses(
        (status = 201, description = "Tag created", body = TagResponse),
        (status = 400, description = "Validation error"),
    ),
)]
pub async fn create_tag(
    State(tag_service): State<Arc<TagService>>,
    Extension(user): Extension<User>,
    Json(req): Json<CreateTagRequest>,
) -> Result<(StatusCode, Json<TagResponse>)> {
    let tag = tag_service.create(user.id, req).await?;
    tracing::info!("Tag created: {} by user {}", tag.name, user.id);
    Ok((StatusCode::CREATED, Json(tag)))
}

/// GET /api/v1/tags/:id
#[utoipa::path(
    get,
    path = "/api/v1/tags/{id}",
    tag = "Tags",
    params(
        ("id", description = "Tag UUID"),
    ),
    responses(
        (status = 200, description = "Tag retrieved", body = TagResponse),
        (status = 404, description = "Tag not found"),
    ),
)]
pub async fn get_tag(
    State(tag_service): State<Arc<TagService>>,
    Extension(user): Extension<User>,
    Path(tag_id): Path<Uuid>,
) -> Result<Json<TagResponse>> {
    let tag = tag_service.get(tag_id, user.id).await?;
    Ok(Json(tag))
}

/// PUT /api/v1/tags/:id
#[utoipa::path(
    put,
    path = "/api/v1/tags/{id}",
    tag = "Tags",
    request_body = UpdateTagRequest,
    params(
        ("id", description = "Tag UUID"),
    ),
    responses(
        (status = 200, description = "Tag updated", body = TagResponse),
    ),
)]
pub async fn update_tag(
    State(tag_service): State<Arc<TagService>>,
    Extension(user): Extension<User>,
    Path(tag_id): Path<Uuid>,
    Json(req): Json<UpdateTagRequest>,
) -> Result<Json<TagResponse>> {
    let tag = tag_service.update(tag_id, user.id, req).await?;
    Ok(Json(tag))
}

/// DELETE /api/v1/tags/:id
#[utoipa::path(
    delete,
    path = "/api/v1/tags/{id}",
    tag = "Tags",
    params(
        ("id", description = "Tag UUID"),
    ),
    responses(
        (status = 204, description = "Tag deleted"),
    ),
)]
pub async fn delete_tag(
    State(tag_service): State<Arc<TagService>>,
    Extension(user): Extension<User>,
    Path(tag_id): Path<Uuid>,
) -> Result<StatusCode> {
    tag_service.delete(tag_id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/notes/:note_id/tags
#[utoipa::path(
    post,
    path = "/api/v1/notes/{note_id}/tags",
    tag = "Tags",
    request_body = AddTagRequest,
    params(
        ("note_id", description = "Note UUID"),
    ),
    responses(
        (status = 201, description = "Tag added to note"),
        (status = 400, description = "Validation error"),
    ),
)]
pub async fn add_tag_to_note(
    State(tag_service): State<Arc<TagService>>,
    Extension(user): Extension<User>,
    Path(note_id): Path<Uuid>,
    Json(req): Json<AddTagRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    tag_service
        .add_to_note(note_id, req.tag_id, user.id)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "message": "Tag added to note" })),
    ))
}

/// DELETE /api/v1/notes/:note_id/tags/:tag_id
#[utoipa::path(
    delete,
    path = "/api/v1/notes/{note_id}/tags/{tag_id}",
    tag = "Tags",
    params(
        ("note_id", description = "Note UUID"),
        ("tag_id", description = "Tag UUID"),
    ),
    responses(
        (status = 204, description = "Tag removed from note"),
    ),
)]
pub async fn remove_tag_from_note(
    State(tag_service): State<Arc<TagService>>,
    Extension(user): Extension<User>,
    Path((note_id, tag_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    tag_service
        .remove_from_note(note_id, tag_id, user.id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/tags/:id/notes
#[utoipa::path(
    get,
    path = "/api/v1/tags/{id}/notes",
    tag = "Tags",
    params(
        ("id", description = "Tag UUID"),
        TagNotesQuery,
    ),
    responses(
        (status = 200, description = "Notes with tag", body = NoteListResponse),
    ),
)]
pub async fn get_notes_by_tag(
    State(tag_service): State<Arc<TagService>>,
    Extension(user): Extension<User>,
    Path(tag_id): Path<Uuid>,
    Query(query): Query<TagNotesQuery>,
) -> Result<Json<NoteListResponse>> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);

    let notes = tag_service
        .get_notes_by_tag(tag_id, user.id, page, limit)
        .await?;
    Ok(Json(notes))
}
