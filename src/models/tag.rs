// src/models/tag.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Tag {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagResponse {
    pub id: Uuid,
    pub name: String,
    pub note_count: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct TagWithCount {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub note_count: i64,
}

impl From<TagWithCount> for TagResponse {
    fn from(t: TagWithCount) -> Self {
        Self {
            id: t.id,
            name: t.name,
            note_count: t.note_count,
            created_at: t.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTagRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct AddTagRequest {
    pub tag_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct TagListResponse {
    pub tags: Vec<TagResponse>,
    pub total: i64,
}
