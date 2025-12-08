use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Revision {
    pub id: Uuid,
    pub note_id: Uuid,
    pub content: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct RevisionResponse {
    pub id: Uuid,
    pub note_id: Uuid,
    pub content: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

impl From<Revision> for RevisionResponse {
    fn from(revision: Revision) -> Self {
        Self {
            id: revision.id,
            note_id: revision.note_id,
            content: revision.content,
            created_by: revision.created_by,
            created_at: revision.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RevisionListResponse {
    pub revisions: Vec<RevisionResponse>,
    pub total: i64,
}