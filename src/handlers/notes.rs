use axum::{
    extract::{State, Path, Query},
    http::StatusCode,
    Extension,
    Json,
};
use uuid::Uuid;
use std::sync::Arc;
use crate::models::note::{CreateNoteRequest, UpdateNoteRequest, NoteResponse, NoteListResponse, NoteQueryParams};
use crate::models::user::User;
use crate::services::NoteService;
use crate::utils::errors::Result;

pub async fn create_note(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<User>,
    Json(req): Json<CreateNoteRequest>,
) -> Result<(StatusCode, Json<NoteResponse>)> {
    let note = note_service.create(user.id, req).await?;
    Ok((StatusCode::CREATED, Json(note)))
}

pub async fn get_note(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<User>,
    Path(note_id): Path<Uuid>,
) -> Result<Json<NoteResponse>> {
    let note = note_service.get(note_id, user.id).await?;
    Ok(Json(note))
}

pub async fn list_notes(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<User>,
    Query(params): Query<NoteQueryParams>,
) -> Result<Json<NoteListResponse>> {
    let notes = note_service.list(user.id, params).await?;
    Ok(Json(notes))
}

pub async fn update_note(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<User>,
    Path(note_id): Path<Uuid>,
    Json(req): Json<UpdateNoteRequest>,
) -> Result<Json<NoteResponse>> {
    let note = note_service.update(note_id, user.id, req).await?;
    Ok(Json(note))
}

pub async fn delete_note(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<User>,
    Path(note_id): Path<Uuid>,
) -> Result<StatusCode> {
    note_service.delete(note_id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}