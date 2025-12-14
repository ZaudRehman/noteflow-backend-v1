// src/handlers/notes.rs
use crate::models::note::{
    CreateNoteRequest, NoteListResponse, NoteQueryParams, NoteResponse, UpdateNoteRequest,
};
use crate::models::user::User;
use crate::services::NoteService;
use crate::utils::errors::Result;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use std::sync::Arc;
use uuid::Uuid;

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

/// POST /api/v1/notes/:id/favorite
pub async fn toggle_favorite(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<User>,
    Path(note_id): Path<Uuid>,
) -> Result<Json<NoteResponseEnhanced>> {
    let note = note_service.toggle_favorite(note_id, user.id).await?;
    Ok(Json(note))
}

/// POST /api/v1/notes/:id/archive
pub async fn toggle_archive(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<User>,
    Path(note_id): Path<Uuid>,
) -> Result<Json<NoteResponseEnhanced>> {
    let note = note_service.toggle_archive(note_id, user.id).await?;
    Ok(Json(note))
}

/// GET /api/v1/search
pub async fn search(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<User>,
    Query(params): Query<SearchParams>,
) -> Result<Json<SearchResponse>> {
    let results = note_service.search(user.id, params).await?;
    Ok(Json(results))
}

/// GET /api/v1/notes - Enhanced with filters
pub async fn list_notes_filtered(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<User>,
    Query(params): Query<NoteFilterParams>,
) -> Result<Json<NoteListResponseEnhanced>> {
    let notes = note_service.list_filtered(user.id, params).await?;
    Ok(Json(notes))
}
