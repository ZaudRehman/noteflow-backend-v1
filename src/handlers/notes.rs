// src/handlers/notes.rs
use crate::models::note::{
    CreateNoteRequest, NoteFilterParams, NoteListResponse, NoteQueryParams, NoteResponse,
    SearchParams, SearchResponse, UpdateNoteRequest,
};
use crate::models::user::User;
use crate::services::{ExportService, NoteService, StyleOptions};
use crate::utils::errors::Result;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[utoipa::path(
    post,
    path = "/api/v1/notes",
    tag = "Notes",
    request_body = CreateNoteRequest,
    responses(
        (status = 201, description = "Note created", body = NoteResponse),
        (status = 400, description = "Validation error"),
    ),
)]
pub async fn create_note(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<User>,
    Json(req): Json<CreateNoteRequest>,
) -> Result<(StatusCode, Json<NoteResponse>)> {
    let note = note_service.create(user.id, req).await?;
    Ok((StatusCode::CREATED, Json(note)))
}

#[utoipa::path(
    get,
    path = "/api/v1/notes/{id}",
    tag = "Notes",
    params(
        ("id", description = "Note UUID"),
    ),
    responses(
        (status = 200, description = "Note retrieved", body = NoteResponse),
        (status = 404, description = "Note not found"),
    ),
)]
pub async fn get_note(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<User>,
    Path(note_id): Path<Uuid>,
) -> Result<Json<NoteResponse>> {
    let note = note_service.get(note_id, user.id).await?;
    Ok(Json(note))
}

#[utoipa::path(
    get,
    path = "/api/v1/notes",
    tag = "Notes",
    params(NoteQueryParams),
    responses(
        (status = 200, description = "List of notes", body = NoteListResponse),
    ),
)]
pub async fn list_notes(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<User>,
    Query(params): Query<NoteQueryParams>,
) -> Result<Json<NoteListResponse>> {
    let notes = note_service.list(user.id, params).await?;
    Ok(Json(notes))
}

#[utoipa::path(
    put,
    path = "/api/v1/notes/{id}",
    tag = "Notes",
    request_body = UpdateNoteRequest,
    params(
        ("id", description = "Note UUID"),
    ),
    responses(
        (status = 200, description = "Note updated", body = NoteResponse),
        (status = 400, description = "Validation error"),
    ),
)]
pub async fn update_note(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<User>,
    Path(note_id): Path<Uuid>,
    Json(req): Json<UpdateNoteRequest>,
) -> Result<Json<NoteResponse>> {
    let note = note_service.update(note_id, user.id, req).await?;
    Ok(Json(note))
}

#[utoipa::path(
    delete,
    path = "/api/v1/notes/{id}",
    tag = "Notes",
    params(
        ("id", description = "Note UUID"),
    ),
    responses(
        (status = 204, description = "Note deleted"),
    ),
)]
pub async fn delete_note(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<User>,
    Path(note_id): Path<Uuid>,
) -> Result<StatusCode> {
    note_service.delete(note_id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/notes/:id/favorite
#[utoipa::path(
    post,
    path = "/api/v1/notes/{id}/favorite",
    tag = "Notes",
    params(
        ("id", description = "Note UUID"),
    ),
    responses(
        (status = 200, description = "Toggled favorite", body = NoteResponse),
    ),
)]
pub async fn toggle_favorite(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<User>,
    Path(note_id): Path<Uuid>,
) -> Result<Json<NoteResponse>> {
    let note = note_service.toggle_favorite(note_id, user.id).await?;
    Ok(Json(note))
}

/// POST /api/v1/notes/:id/archive
#[utoipa::path(
    post,
    path = "/api/v1/notes/{id}/archive",
    tag = "Notes",
    params(
        ("id", description = "Note UUID"),
    ),
    responses(
        (status = 200, description = "Toggled archive", body = NoteResponse),
    ),
)]
pub async fn toggle_archive(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<User>,
    Path(note_id): Path<Uuid>,
) -> Result<Json<NoteResponse>> {
    let note = note_service.toggle_archive(note_id, user.id).await?;
    Ok(Json(note))
}

/// GET /api/v1/search
#[utoipa::path(
    get,
    path = "/api/v1/search",
    tag = "Search",
    params(SearchParams),
    responses(
        (status = 200, description = "Search results", body = SearchResponse),
    ),
)]
pub async fn search(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<User>,
    Query(params): Query<SearchParams>,
) -> Result<Json<SearchResponse>> {
    let results = note_service.search(user.id, params).await?;
    Ok(Json(results))
}

/// GET /api/v1/notes/:id/export?format=markdown|html|txt|rtf|pdf|epub|json
#[utoipa::path(
    get,
    path = "/api/v1/notes/{id}/export",
    tag = "Notes",
    params(
        ("id", description = "Note UUID"),
        ("format", description = "Export format (json, markdown, html, txt, rtf, pdf, epub)"),
    ),
    responses(
        (status = 200, description = "Exported note content", content_type = "application/octet-stream"),
    ),
)]
pub async fn export_note(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<User>,
    Path(note_id): Path<Uuid>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<axum::response::Response> {
    use axum::response::IntoResponse;

    let note = note_service.get(note_id, user.id).await?;
    let blocks = &note.blocks;
    let format = params.get("format").map(|s| s.as_str()).unwrap_or("json");
    let opts = StyleOptions::from_params(&params);

    let (status, content_type, body): (StatusCode, &str, Vec<u8>) = match format {
        "markdown" => {
            let md = ExportService::blocks_to_markdown(blocks, &note.title, &opts);
            (StatusCode::OK, "text/markdown; charset=utf-8", md.into_bytes())
        }
        "html" => {
            let html = ExportService::blocks_to_html(blocks, &note.title, &opts);
            (StatusCode::OK, "text/html; charset=utf-8", html.into_bytes())
        }
        "txt" => {
            let txt = ExportService::blocks_to_plain_text(blocks, &note.title, &opts);
            (StatusCode::OK, "text/plain; charset=utf-8", txt.into_bytes())
        }
        "rtf" => {
            let rtf = ExportService::blocks_to_rtf(blocks, &note.title, &opts);
            (StatusCode::OK, "application/rtf", rtf)
        }
        "pdf" => {
            let pdf = ExportService::blocks_to_pdf(blocks, &note.title, &opts)
                .map_err(|e| crate::utils::errors::AppError::InternalError(e))?;
            (StatusCode::OK, "application/pdf", pdf)
        }
        "epub" => {
            let epub = ExportService::blocks_to_epub(blocks, &note.title, &opts)
                .map_err(|e| crate::utils::errors::AppError::InternalError(e))?;
            (StatusCode::OK, "application/epub+zip", epub)
        }
        _ => {
            let json = serde_json::to_vec_pretty(&note)
                .map_err(|e| crate::utils::errors::AppError::InternalError(e.to_string()))?;
            (StatusCode::OK, "application/json; charset=utf-8", json)
        }
    };

    Ok((
        status,
        [("Content-Type", content_type)],
        body,
    ).into_response())
}

/// GET /api/v1/notes - Enhanced with filters
#[utoipa::path(
    get,
    path = "/api/v1/notes",
    tag = "Notes (Filtered)",
    params(NoteFilterParams),
    responses(
        (status = 200, description = "Filtered notes list", body = NoteListResponse),
    ),
)]
pub async fn list_notes_filtered(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<User>,
    Query(params): Query<NoteFilterParams>,
) -> Result<Json<NoteListResponse>> {
    let notes = note_service.list_filtered(user.id, params).await?;
    Ok(Json(notes))
}

/// POST /api/v1/notes/:id/restore
#[utoipa::path(
    post,
    path = "/api/v1/notes/{id}/restore",
    tag = "Notes",
    params(
        ("id", description = "Note UUID"),
    ),
    responses(
        (status = 200, description = "Note restored", body = NoteResponse),
        (status = 404, description = "Note not found"),
    ),
)]
pub async fn restore_note(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<User>,
    Path(note_id): Path<Uuid>,
) -> Result<Json<NoteResponse>> {
    let note = note_service.restore_note(note_id, user.id).await?;
    Ok(Json(note))
}

/// DELETE /api/v1/notes/:id/permanent
#[utoipa::path(
    delete,
    path = "/api/v1/notes/{id}/permanent",
    tag = "Notes",
    params(
        ("id", description = "Note UUID"),
    ),
    responses(
        (status = 204, description = "Note permanently deleted"),
        (status = 404, description = "Note not found"),
    ),
)]
pub async fn permanent_delete_note(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<User>,
    Path(note_id): Path<Uuid>,
) -> Result<StatusCode> {
    note_service.permanent_delete(note_id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/notes/trash
#[utoipa::path(
    delete,
    path = "/api/v1/notes/trash",
    tag = "Notes",
    responses(
        (status = 204, description = "Trash emptied"),
    ),
)]
pub async fn empty_trash(
    State(note_service): State<Arc<NoteService>>,
    Extension(user): Extension<User>,
) -> Result<StatusCode> {
    note_service.empty_trash(user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
