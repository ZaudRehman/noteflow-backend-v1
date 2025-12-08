use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ActiveSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub note_id: Uuid,
    pub connection_id: String,
    pub last_active: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ActiveUserInfo {
    pub user_id: Uuid,
    pub display_name: String,
    pub connection_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub message_type: MessageType,
    pub note_id: Uuid,
    pub user_id: Uuid,
    pub content: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Edit,
    CursorMove,
    UserJoined,
    UserLeft,
    Sync,
}