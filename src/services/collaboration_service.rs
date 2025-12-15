// src/services/collaboration_service.rs
use axum::extract::ws::{Message, WebSocket};
use chrono::Utc;
use futures::{sink::SinkExt, stream::StreamExt};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::models::{collaboration::*, user::User};
use crate::utils::errors::{AppError, Result};

pub struct CollaborationService {
    pool: PgPool,
    tx: broadcast::Sender<WsMessage>,
}

impl CollaborationService {
    pub fn new(pool: PgPool) -> Arc<Self> {
        let (tx, _) = broadcast::channel(1000);
        Arc::new(Self { pool, tx })
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
        let _ = self.tx.send(join_msg);

        // Clone values needed for the spawned tasks
        let user_id = user.id;
        let user_name = user.display_name.clone();
        let service_clone = Arc::clone(&self);
        
        // Create channel for sending messages to WebSocket
        let (ws_tx, mut ws_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
        
        // Spawn task to forward broadcast messages to this WebSocket
        let sender_task = tokio::spawn(async move {
            let mut sender = sender;
            
            // Forward messages from ws_rx to actual WebSocket
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
                    // Filter messages for this note and don't echo user's own cursor moves
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
                                // Update cursor position in database
                                let _ = service_clone
                                    .create_or_update_session(
                                        note_id,
                                        user_id,
                                        position.line as i32,
                                        position.column as i32,
                                    )
                                    .await;

                                // Broadcast cursor position
                                let broadcast_msg = WsMessage::CursorMove {
                                    note_id,
                                    user_id,
                                    user_name: user_name.clone(),
                                    position,
                                    timestamp: Utc::now(),
                                };
                                let _ = service_clone.tx.send(broadcast_msg);
                            }
                            WsMessage::NoteUpdated { content_delta, .. } => {
                                // Broadcast note update to other users
                                let broadcast_msg = WsMessage::NoteUpdated {
                                    note_id,
                                    user_id,
                                    title: None,
                                    content_delta,
                                    timestamp: Utc::now(),
                                };
                                let _ = service_clone.tx.send(broadcast_msg);
                            }
                            WsMessage::Ping { .. } => {
                                // Respond with pong
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
        let _ = self.tx.send(leave_msg);

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
