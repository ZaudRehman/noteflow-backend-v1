// src/models/collaboration.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum WsMessage {
    #[serde(rename = "note:created")]
    NoteCreated {
        note_id: Uuid,
        user_id: Uuid,
        title: String,
        timestamp: DateTime<Utc>,
    },

    #[serde(rename = "note:updated")]
    NoteUpdated {
        note_id: Uuid,
        user_id: Uuid,
        title: Option<String>,
        content_delta: Option<String>,
        timestamp: DateTime<Utc>,
    },

    #[serde(rename = "note:deleted")]
    NoteDeleted {
        note_id: Uuid,
        user_id: Uuid,
        timestamp: DateTime<Utc>,
    },

    #[serde(rename = "tag:created")]
    TagCreated {
        tag_id: Uuid,
        name: String,
        user_id: Uuid,
        timestamp: DateTime<Utc>,
    },

    #[serde(rename = "tag:updated")]
    TagUpdated {
        tag_id: Uuid,
        name: String,
        user_id: Uuid,
        timestamp: DateTime<Utc>,
    },

    #[serde(rename = "tag:deleted")]
    TagDeleted {
        tag_id: Uuid,
        user_id: Uuid,
        timestamp: DateTime<Utc>,
    },

    #[serde(rename = "cursor:move")]
    CursorMove {
        note_id: Uuid,
        user_id: Uuid,
        user_name: String,
        position: CursorPosition,
        timestamp: DateTime<Utc>,
    },

    #[serde(rename = "user:joined")]
    UserJoined {
        note_id: Uuid,
        user_id: Uuid,
        user_name: String,
        timestamp: DateTime<Utc>,
    },

    #[serde(rename = "user:left")]
    UserLeft {
        note_id: Uuid,
        user_id: Uuid,
        timestamp: DateTime<Utc>,
    },

    #[serde(rename = "error")]
    Error {
        message: String,
        timestamp: DateTime<Utc>,
    },

    #[serde(rename = "ping")]
    Ping { timestamp: DateTime<Utc> },

    #[serde(rename = "pong")]
    Pong { timestamp: DateTime<Utc> },

    // === CRDT Operations ===

    #[serde(rename = "op:insert")]
    OpInsert {
        note_id: Uuid,
        client_id: String,
        position: usize,
        text: String,
        timestamp: DateTime<Utc>,
    },

    #[serde(rename = "op:delete")]
    OpDelete {
        note_id: Uuid,
        client_id: String,
        position: usize,
        length: usize,
        timestamp: DateTime<Utc>,
    },

    #[serde(rename = "op:sync_request")]
    OpSyncRequest {
        note_id: Uuid,
        last_known_id: i64,
        timestamp: DateTime<Utc>,
    },

    #[serde(rename = "op:sync_batch")]
    OpSyncBatch {
        note_id: Uuid,
        ops: Vec<CollabOpData>,
        timestamp: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CursorPosition {
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ActiveSession {
    pub id: Uuid,
    pub note_id: Uuid,
    pub user_id: Uuid,
    pub cursor_line: i32,
    pub cursor_column: i32,
    pub last_seen_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NoteCollaborator {
    pub note_id: Uuid,
    pub user_id: Uuid,
    pub permission: String,
    pub invited_by: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct CollabOpData {
    pub id: i64,
    pub note_id: Uuid,
    pub client_id: String,
    pub op_type: String,
    pub position: i32,
    pub text_content: Option<String>,
    pub length: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
