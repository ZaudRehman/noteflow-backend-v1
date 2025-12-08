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
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct NoteResponse {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub last_edited_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
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

#[derive(Debug, Serialize)]
pub struct NoteListResponse {
    pub notes: Vec<NoteResponse>,
    pub total: i64,
}

#[derive(Debug, Deserialize)]
pub struct NoteQueryParams {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub tag: Option<String>,
}