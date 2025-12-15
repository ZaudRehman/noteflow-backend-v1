// src/models/note.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Note {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub content: String,
    pub last_edited_by: Option<Uuid>,
    pub is_favorited: bool,
    pub is_archived: bool,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NoteResponse {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub last_edited_by: Option<Uuid>,
    pub is_favorited: bool,
    pub is_archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub active_users: Vec<ActiveUserInfo>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateNoteRequest {
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    pub content: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateNoteRequest {
    #[validate(length(min = 1, max = 255))]
    pub title: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NoteListResponse {
    pub notes: Vec<NoteResponse>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

#[derive(Debug, Deserialize)]
pub struct NoteQueryParams {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub tag: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NoteFilterParams {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub filter: Option<String>, // "favorites", "archived", "all"
    pub tag_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub notes: Vec<NoteResponse>,
    pub total: i64,
    pub query: String,
}

#[derive(Debug, Serialize)]
pub struct ActiveUserInfo {
    pub user_id: Uuid,
    pub display_name: String,
    pub cursor_line: i32,
    pub cursor_column: i32,
}
