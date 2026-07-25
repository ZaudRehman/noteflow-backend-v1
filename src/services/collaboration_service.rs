// src/services/collaboration_service.rs
use axum::extract::ws::{Message, WebSocket};
use chrono::Utc;
use futures::{sink::SinkExt, stream::StreamExt};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use crate::db::RedisManager;
use crate::models::{collaboration::*, user::User};
use crate::utils::errors::{AppError, Result};

pub struct CollaborationService {
    pool: PgPool,
    tx: broadcast::Sender<WsMessage>,
    redis: Option<Arc<RedisManager>>,
    redis_url: String,
    /// Track which notes have active Redis subscribers to avoid duplicates
    note_subscribers: Arc<RwLock<HashMap<Uuid, ()>>>,
}

impl CollaborationService {
    pub fn new(
        pool: PgPool,
        redis: Option<Arc<RedisManager>>,
        redis_url: &str,
    ) -> Arc<Self> {
        let (tx, _) = broadcast::channel(1000);
        Arc::new(Self {
            pool,
            tx,
            redis,
            redis_url: redis_url.to_string(),
            note_subscribers: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Publish to both local in-memory broadcast AND Redis (for other instances)
    fn publish(&self, msg: &WsMessage) {
        // Local broadcast
        let _ = self.tx.send(msg.clone());

        // Redis broadcast (cross-instance)
        if let Some(ref redis) = self.redis {
            if let Ok(json) = serde_json::to_string(msg) {
                let note_id = match msg {
                    WsMessage::NoteCreated { note_id, .. }
                    | WsMessage::NoteUpdated { note_id, .. }
                    | WsMessage::NoteDeleted { note_id, .. }
                    | WsMessage::CursorMove { note_id, .. }
                    | WsMessage::UserJoined { note_id, .. }
                    | WsMessage::UserLeft { note_id, .. } => note_id,
                    _ => return,
                };
                let channel = format!("note:{}", note_id);
                let _ = redis.publish(&channel, &json);
            }
        }
    }

    /// Ensure a Redis subscriber exists for this note (spawned once per note)
    async fn ensure_redis_subscriber(&self, note_id: Uuid) {
        let mut subscribers = self.note_subscribers.write().await;
        if subscribers.contains_key(&note_id) {
            return; // Already subscribed
        }

        subscribers.insert(note_id, ());

        let channel = format!("note:{}", note_id);
        let redis_url = self.redis_url.clone();
        let tx = self.tx.clone();

        // Drop the lock before spawning the task
        drop(subscribers);

        if redis_url.is_empty() {
            return;
        }

        tokio::spawn(async move {
            use futures::StreamExt;
            use redis::Client;

            let client = match Client::open(redis_url) {
                Ok(c) => c,
                Err(_) => return,
            };
            let conn = match client.get_async_connection().await {
                Ok(c) => c,
                Err(_) => return,
            };
            let mut pubsub = conn.into_pubsub();
            if pubsub.subscribe(&channel).await.is_err() {
                return;
            }

            let mut stream = pubsub.on_message();
            while let Some(msg) = stream.next().await {
                let payload: String = msg.get_payload().unwrap_or_default();
                if payload.is_empty() {
                    continue;
                }
                if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&payload) {
                    let _ = tx.send(ws_msg);
                }
            }
        });
    }

    pub async fn handle_connection(self: Arc<Self>, ws: WebSocket, user: User, note_id: Uuid) {
        // Verify user has access to note
        if let Err(e) = self.verify_note_access(note_id, user.id).await {
            tracing::warn!(
                "Access denied for user {} to note {}: {}",
                user.id,
                note_id,
                e
            );
            return;
        }

        tracing::info!("User {} joined note {} collaboration", user.id, note_id);

        // Ensure Redis subscriber exists for this note
        self.ensure_redis_subscriber(note_id).await;

        let (sender, mut receiver) = ws.split();
        let mut rx = self.tx.subscribe();

        // Create active session
        if let Err(e) = self.create_or_update_session(note_id, user.id, 0, 0).await {
            tracing::error!("Failed to create session: {}", e);
            return;
        }

        // Send join event to all other users
        let join_msg = WsMessage::UserJoined {
            note_id,
            user_id: user.id,
            user_name: user.display_name.clone(),
            timestamp: Utc::now(),
        };
        self.publish(&join_msg);

        let user_id = user.id;
        let user_name = user.display_name.clone();
        let service_clone = Arc::clone(&self);

        // Create channel for sending messages to WebSocket
        let (ws_tx, mut ws_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

        // Spawn task to forward messages to WebSocket
        let sender_task = tokio::spawn(async move {
            let mut sender = sender;
            while let Some(msg) = ws_rx.recv().await {
                if sender.send(msg).await.is_err() {
                    break;
                }
            }
        });

        // Spawn task to receive broadcast messages and filter them
        let broadcast_task = {
            let ws_tx = ws_tx.clone();
            tokio::spawn(async move {
                while let Ok(msg) = rx.recv().await {
                    let should_send = match &msg {
                        WsMessage::CursorMove {
                            note_id: msg_note_id,
                            user_id: msg_user_id,
                            ..
                        } => *msg_note_id == note_id && *msg_user_id != user_id,
                        WsMessage::NoteUpdated {
                            note_id: msg_note_id,
                            ..
                        } => *msg_note_id == note_id,
                        WsMessage::UserJoined {
                            note_id: msg_note_id,
                            user_id: msg_user_id,
                            ..
                        } => *msg_note_id == note_id && *msg_user_id != user_id,
                        WsMessage::UserLeft {
                            note_id: msg_note_id,
                            ..
                        } => *msg_note_id == note_id,
                        _ => false,
                    };

                    if should_send {
                        if let Ok(json) = serde_json::to_string(&msg) {
                            if ws_tx.send(Message::Text(json)).is_err() {
                                break;
                            }
                        }
                    }
                }
            })
        };

        // Handle incoming messages from this client
        while let Some(result) = receiver.next().await {
            match result {
                Ok(Message::Text(text)) => {
                    if let Ok(msg) = serde_json::from_str::<WsMessage>(&text) {
                        match msg {
                            WsMessage::CursorMove { position, .. } => {
                                let _ = service_clone
                                    .create_or_update_session(
                                        note_id,
                                        user_id,
                                        position.line as i32,
                                        position.column as i32,
                                    )
                                    .await;

                                let broadcast_msg = WsMessage::CursorMove {
                                    note_id,
                                    user_id,
                                    user_name: user_name.clone(),
                                    position,
                                    timestamp: Utc::now(),
                                };
                                service_clone.publish(&broadcast_msg);
                            }
                            WsMessage::NoteUpdated { content_delta, .. } => {
                                let broadcast_msg = WsMessage::NoteUpdated {
                                    note_id,
                                    user_id,
                                    title: None,
                                    content_delta,
                                    timestamp: Utc::now(),
                                };
                                service_clone.publish(&broadcast_msg);
                            }
                            WsMessage::Ping { .. } => {
                                let pong = WsMessage::Pong {
                                    timestamp: Utc::now(),
                                };
                                if let Ok(json) = serde_json::to_string(&pong) {
                                    let _ = ws_tx.send(Message::Text(json));
                                }
                            }
                            _ => {
                                tracing::debug!("Received unhandled message type");
                            }
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("WebSocket closed by client");
                    break;
                }
                Ok(Message::Ping(data)) => {
                    let _ = ws_tx.send(Message::Pong(data));
                }
                Err(e) => {
                    tracing::error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        // Cleanup on disconnect
        sender_task.abort();
        broadcast_task.abort();

        let _ = self.delete_session(note_id, user_id).await;

        let leave_msg = WsMessage::UserLeft {
            note_id,
            user_id,
            timestamp: Utc::now(),
        };
        self.publish(&leave_msg);

        tracing::info!("User {} left note {} collaboration", user_id, note_id);
    }

    async fn verify_note_access(&self, note_id: Uuid, user_id: Uuid) -> Result<()> {
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM notes WHERE id = $1 AND user_id = $2 AND is_deleted = false)",
            note_id,
            user_id
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(false);

        if !exists {
            return Err(AppError::NotFound("Note not found or access denied".into()));
        }
        Ok(())
    }

    async fn create_or_update_session(
        &self,
        note_id: Uuid,
        user_id: Uuid,
        cursor_line: i32,
        cursor_column: i32,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO active_sessions (note_id, user_id, cursor_line, cursor_column, last_seen_at)
            VALUES ($1, $2, $3, $4, NOW())
            ON CONFLICT (note_id, user_id) 
            DO UPDATE SET 
                cursor_line = $3,
                cursor_column = $4,
                last_seen_at = NOW()
            "#,
            note_id,
            user_id,
            cursor_line,
            cursor_column
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_session(&self, note_id: Uuid, user_id: Uuid) -> Result<()> {
        sqlx::query!(
            "DELETE FROM active_sessions WHERE note_id = $1 AND user_id = $2",
            note_id,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
