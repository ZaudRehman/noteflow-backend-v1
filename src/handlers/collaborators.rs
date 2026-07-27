use crate::models::note::{AddCollaboratorRequest, CollaboratorListResponse, UpdateCollaboratorRequest};
use crate::models::user::User;
use crate::services::NoteCollaboratorService;
use crate::utils::errors::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use std::sync::Arc;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/v1/notes/{note_id}/collaborators",
    tag = "Collaborators",
    params(
        ("note_id", description = "Note UUID"),
    ),
    responses(
        (status = 200, description = "List of collaborators", body = CollaboratorListResponse),
    ),
)]
pub async fn list_collaborators(
    State(service): State<Arc<NoteCollaboratorService>>,
    Extension(user): Extension<User>,
    Path(note_id): Path<Uuid>,
) -> Result<Json<CollaboratorListResponse>> {
    let result = service.list(note_id, user.id).await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/api/v1/notes/{note_id}/collaborators",
    tag = "Collaborators",
    request_body = AddCollaboratorRequest,
    params(
        ("note_id", description = "Note UUID"),
    ),
    responses(
        (status = 201, description = "Collaborator added", body = CollaboratorListResponse),
        (status = 400, description = "Validation error"),
    ),
)]
pub async fn add_collaborator(
    State(service): State<Arc<NoteCollaboratorService>>,
    Extension(user): Extension<User>,
    Path(note_id): Path<Uuid>,
    Json(req): Json<AddCollaboratorRequest>,
) -> Result<(StatusCode, Json<CollaboratorListResponse>)> {
    service.add(note_id, user.id, req).await?;
    let result = service.list(note_id, user.id).await?;
    Ok((StatusCode::CREATED, Json(result)))
}

#[utoipa::path(
    put,
    path = "/api/v1/notes/{note_id}/collaborators/{target_user_id}",
    tag = "Collaborators",
    request_body = UpdateCollaboratorRequest,
    params(
        ("note_id", description = "Note UUID"),
        ("target_user_id", description = "Target user UUID"),
    ),
    responses(
        (status = 200, description = "Collaborator updated", body = CollaboratorListResponse),
    ),
)]
pub async fn update_collaborator(
    State(service): State<Arc<NoteCollaboratorService>>,
    Extension(user): Extension<User>,
    Path((note_id, target_user_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateCollaboratorRequest>,
) -> Result<Json<CollaboratorListResponse>> {
    service.update_permission(note_id, target_user_id, user.id, req).await?;
    let result = service.list(note_id, user.id).await?;
    Ok(Json(result))
}

#[utoipa::path(
    delete,
    path = "/api/v1/notes/{note_id}/collaborators/{target_user_id}",
    tag = "Collaborators",
    params(
        ("note_id", description = "Note UUID"),
        ("target_user_id", description = "Target user UUID"),
    ),
    responses(
        (status = 204, description = "Collaborator removed"),
    ),
)]
pub async fn remove_collaborator(
    State(service): State<Arc<NoteCollaboratorService>>,
    Extension(user): Extension<User>,
    Path((note_id, target_user_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    service.remove(note_id, target_user_id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
