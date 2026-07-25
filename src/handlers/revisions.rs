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

#[derive(Debug, Deserialize)]
pub struct RevisionListQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

/// GET /api/v1/notes/:note_id/history
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
pub async fn get_revision(
    State(revision_service): State<Arc<RevisionService>>,
    Extension(user): Extension<User>,
    Path((note_id, revision_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<RevisionResponse>> {
    let revision = revision_service.get(note_id, revision_id, user.id).await?;
    Ok(Json(revision))
}

/// POST /api/v1/notes/:note_id/history/:revision_id/restore
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
