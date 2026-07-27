use crate::models::revision::*;
use crate::models::user::User;
use crate::services::RevisionService;
use crate::utils::errors::Result;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct RevisionListQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

/// GET /api/v1/notes/:note_id/history
#[utoipa::path(
    get,
    path = "/api/v1/notes/{note_id}/history",
    tag = "Revisions",
    params(
        ("note_id", description = "Note UUID"),
        RevisionListQuery,
    ),
    responses(
        (status = 200, description = "List of revisions", body = RevisionListResponse),
    ),
)]
pub async fn list_revisions(
    State(revision_service): State<Arc<RevisionService>>,
    Extension(user): Extension<User>,
    Path(note_id): Path<Uuid>,
    Query(query): Query<RevisionListQuery>,
) -> Result<Json<RevisionListResponse>> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);
    let revisions = revision_service.list(note_id, user.id, page, limit).await?;
    Ok(Json(revisions))
}

/// GET /api/v1/notes/:note_id/history/:revision_id
#[utoipa::path(
    get,
    path = "/api/v1/notes/{note_id}/history/{revision_id}",
    tag = "Revisions",
    params(
        ("note_id", description = "Note UUID"),
        ("revision_id", description = "Revision UUID"),
    ),
    responses(
        (status = 200, description = "Revision retrieved", body = RevisionResponse),
        (status = 404, description = "Revision not found"),
    ),
)]
pub async fn get_revision(
    State(revision_service): State<Arc<RevisionService>>,
    Extension(user): Extension<User>,
    Path((note_id, revision_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<RevisionResponse>> {
    let revision = revision_service.get(note_id, revision_id, user.id).await?;
    Ok(Json(revision))
}

/// POST /api/v1/notes/:note_id/history/:revision_id/restore
#[utoipa::path(
    post,
    path = "/api/v1/notes/{note_id}/history/{revision_id}/restore",
    tag = "Revisions",
    params(
        ("note_id", description = "Note UUID"),
        ("revision_id", description = "Revision UUID"),
    ),
    responses(
        (status = 200, description = "Revision restored"),
        (status = 404, description = "Revision not found"),
    ),
)]
pub async fn restore_revision(
    State(revision_service): State<Arc<RevisionService>>,
    Extension(user): Extension<User>,
    Path((note_id, revision_id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    revision_service.restore(note_id, revision_id, user.id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "message": "Revision restored" })),
    ))
}
