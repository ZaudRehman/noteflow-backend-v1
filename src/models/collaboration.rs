// src/models/collaboration.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
