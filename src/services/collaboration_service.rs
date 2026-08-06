// src/services/collaboration_service.rs
use axum::extract::ws::{Message, WebSocket};
use chrono::Utc;
use futures::{sink::SinkExt, stream::StreamExt};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use crate::db::RedisManager;
use crate::models::{collaboration::*, user::User};
use crate::utils::errors::{AppError, Result};

fn should_relay_to_client(msg: &WsMessage, note_id: Uuid, user_id: Uuid) -> bool {
    match msg {
        WsMessage::CursorMove {
            note_id: nid,
            user_id: uid,
            ..
        } => *nid == note_id && *uid != user_id,
        WsMessage::NoteUpdated { note_id: nid, .. }
        | WsMessage::UserLeft { note_id: nid, .. }
        | WsMessage::OpInsert { note_id: nid, .. }
        | WsMessage::OpDelete { note_id: nid, .. }
        | WsMessage::OpSyncBatch { note_id: nid, .. }
        | WsMessage::BlockAdd { note_id: nid, .. }
        | WsMessage::BlockUpdate { note_id: nid, .. }
        | WsMessage::BlockRemove { note_id: nid, .. }
        | WsMessage::BlockMove { note_id: nid, .. }
        | WsMessage::BlockSyncBatch { note_id: nid, .. } => *nid == note_id,
        WsMessage::UserJoined {
            note_id: nid,
            user_id: uid,
            ..
        } => *nid == note_id && *uid != user_id,
        _ => false,
    }
}

pub struct CollaborationService {
    pool: PgPool,
    tx: broadcast::Sender<WsMessage>,
    redis: Option<Arc<RedisManager>>,
    redis_url: String,
    pub active_connections: AtomicUsize,
    pub max_connections: usize,
    /// Track which notes have active Redis subscribers to avoid duplicates
    note_subscribers: Arc<RwLock<HashMap<Uuid, ()>>>,
    /// Per-user concurrent connection count
    user_connections: Arc<RwLock<HashMap<Uuid, usize>>>,
    max_connections_per_user: usize,
}

impl CollaborationService {
    pub fn new(
        pool: PgPool,
        redis: Option<Arc<RedisManager>>,
        redis_url: &str,
        max_connections: usize,
    ) -> Arc<Self> {
        let (tx, _) = broadcast::channel(100_000);
        Arc::new(Self {
            pool,
            tx,
            redis,
            redis_url: redis_url.to_string(),
            active_connections: AtomicUsize::new(0),
            max_connections,
            note_subscribers: Arc::new(RwLock::new(HashMap::new())),
            user_connections: Arc::new(RwLock::new(HashMap::new())),
            max_connections_per_user: 5,
        })
    }

    /// Publish to both local in-memory broadcast AND Redis (for other instances)
    fn publish(&self, msg: &WsMessage) {
        let _ = self.tx.send(msg.clone());

        if let Some(ref redis) = self.redis {
            if let Ok(json) = serde_json::to_string(msg) {
                let note_id = match msg {
                    WsMessage::NoteCreated { note_id, .. }
                    | WsMessage::NoteUpdated { note_id, .. }
                    | WsMessage::NoteDeleted { note_id, .. }
                    | WsMessage::CursorMove { note_id, .. }
                    | WsMessage::UserJoined { note_id, .. }
                    | WsMessage::UserLeft { note_id, .. }
                    | WsMessage::OpInsert { note_id, .. }
                    | WsMessage::OpDelete { note_id, .. }
                    | WsMessage::OpSyncBatch { note_id, .. }
                    | WsMessage::BlockAdd { note_id, .. }
                    | WsMessage::BlockUpdate { note_id, .. }
                    | WsMessage::BlockRemove { note_id, .. }
                    | WsMessage::BlockMove { note_id, .. }
                    | WsMessage::BlockSyncBatch { note_id, .. } => note_id,
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

    fn try_acquire_connection(&self) -> bool {
        let prev = self.active_connections.fetch_add(1, Ordering::SeqCst);
        if prev >= self.max_connections {
            self.active_connections.fetch_sub(1, Ordering::SeqCst);
            tracing::warn!(
                "Connection rejected: {} active connections exceeds limit of {}",
                prev,
                self.max_connections
            );
            return false;
        }
        true
    }

    fn release_connection(&self) -> usize {
        self.active_connections.fetch_sub(1, Ordering::SeqCst) - 1
    }

    async fn handle_cursor_update(
        self: &Arc<Self>,
        note_id: Uuid,
        user_id: Uuid,
        user_name: &str,
        position: CursorPosition,
    ) {
        let _ = self
            .create_or_update_session(
                note_id,
                user_id,
                position.line as i32,
                position.column as i32,
            )
            .await;

        self.publish(&WsMessage::CursorMove {
            note_id,
            user_id,
            user_name: user_name.to_string(),
            position,
            timestamp: Utc::now(),
        });
    }

    async fn handle_note_update(
        self: &Arc<Self>,
        note_id: Uuid,
        user_id: Uuid,
        content_delta: Option<String>,
    ) {
        self.publish(&WsMessage::NoteUpdated {
            note_id,
            user_id,
            title: None,
            content_delta,
            timestamp: Utc::now(),
        });
    }

    async fn handle_incoming_message(
        self: &Arc<Self>,
        note_id: Uuid,
        user_id: Uuid,
        user_name: &str,
        text: String,
        ws_tx: &tokio::sync::mpsc::UnboundedSender<Message>,
    ) {
        let msg = match serde_json::from_str::<WsMessage>(&text) {
            Ok(m) => m,
            Err(_) => return,
        };

        // Write operations require owner / write / admin (or legacy 'edit')
        if matches!(
            msg,
            WsMessage::NoteUpdated { .. }
                | WsMessage::OpInsert { .. }
                | WsMessage::OpDelete { .. }
                | WsMessage::BlockAdd { .. }
                | WsMessage::BlockUpdate { .. }
                | WsMessage::BlockRemove { .. }
                | WsMessage::BlockMove { .. }
        ) && !self.verify_write_access(note_id, user_id).await
        {
            return;
        }

        match msg {
            WsMessage::CursorMove { position, .. } => {
                self.handle_cursor_update(note_id, user_id, user_name, position)
                    .await;
            }
            WsMessage::NoteUpdated { content_delta, .. } => {
                self.handle_note_update(note_id, user_id, content_delta)
                    .await;
            }
            WsMessage::OpInsert { client_id, position, text: op_text, .. } => {
                self.handle_op_insert(note_id, &client_id, position, &op_text)
                    .await;
            }
            WsMessage::OpDelete { client_id, position, length, .. } => {
                self.handle_op_delete(note_id, &client_id, position, length)
                    .await;
            }
            WsMessage::OpSyncRequest { last_known_id, .. } => {
                self.handle_op_sync_request(note_id, last_known_id, ws_tx)
                    .await;
            }
            WsMessage::BlockAdd { block_id, block_type, data, position, parent_id, client_id, .. } => {
                self.handle_block_add(note_id, block_id, &block_type, &data, position, parent_id, &client_id).await;
            }
            WsMessage::BlockUpdate { block_id, data, client_id, .. } => {
                self.handle_block_update(note_id, block_id, &data, &client_id).await;
            }
            WsMessage::BlockRemove { block_id, client_id, .. } => {
                self.handle_block_remove(note_id, block_id, &client_id).await;
            }
            WsMessage::BlockMove { block_id, new_position, new_parent_id, client_id, .. } => {
                self.handle_block_move(note_id, block_id, new_position, new_parent_id, &client_id).await;
            }
            WsMessage::BlockSyncBatch { .. } => {
                self.handle_block_sync_request(note_id, ws_tx).await;
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

    async fn cleanup_connection(
        self: &Arc<Self>,
        note_id: Uuid,
        user_id: Uuid,
        sender_task: tokio::task::JoinHandle<()>,
        broadcast_task: tokio::task::JoinHandle<()>,
    ) {
        sender_task.abort();
        broadcast_task.abort();

        let _ = self.delete_session(note_id, user_id).await;
        let active = self.release_connection();

        {
            let mut user_conns = self.user_connections.write().await;
            if let Some(count) = user_conns.get_mut(&user_id) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    user_conns.remove(&user_id);
                }
            }
        }

        self.publish(&WsMessage::UserLeft {
            note_id,
            user_id,
            timestamp: Utc::now(),
        });

        tracing::info!(
            "User {} left note {} collaboration ({} active connections)",
            user_id,
            note_id,
            active
        );
    }

    fn spawn_sender_task(
        sender: futures::stream::SplitSink<WebSocket, Message>,
        mut ws_rx: tokio::sync::mpsc::UnboundedReceiver<Message>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut sender = sender;
            while let Some(msg) = ws_rx.recv().await {
                if sender.send(msg).await.is_err() {
                    break;
                }
            }
        })
    }

    fn spawn_broadcast_task(
        mut rx: broadcast::Receiver<WsMessage>,
        ws_tx: tokio::sync::mpsc::UnboundedSender<Message>,
        note_id: Uuid,
        user_id: Uuid,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                if !should_relay_to_client(&msg, note_id, user_id) {
                    continue;
                }
                if let Ok(json) = serde_json::to_string(&msg) {
                    if ws_tx.send(Message::Text(json)).is_err() {
                        break;
                    }
                }
            }
        })
    }

    async fn handle_op_insert(
        self: &Arc<Self>,
        note_id: Uuid,
        client_id: &str,
        position: usize,
        text: &str,
    ) {
        self.publish(&WsMessage::OpInsert {
            note_id,
            client_id: client_id.to_string(),
            position,
            text: text.to_string(),
            timestamp: Utc::now(),
        });
    }

    async fn handle_op_delete(
        self: &Arc<Self>,
        note_id: Uuid,
        client_id: &str,
        position: usize,
        length: usize,
    ) {
        self.publish(&WsMessage::OpDelete {
            note_id,
            client_id: client_id.to_string(),
            position,
            length,
            timestamp: Utc::now(),
        });
    }

    async fn handle_op_sync_request(
        self: &Arc<Self>,
        note_id: Uuid,
        last_known_id: i64,
        ws_tx: &tokio::sync::mpsc::UnboundedSender<Message>,
    ) {
        let ops = sqlx::query_as::<_, crate::models::collaboration::CollabOpData>(
            r#"
            SELECT id, note_id, client_id, op_type, position, text_content, length, created_at
            FROM collab_operations
            WHERE note_id = $1 AND id > $2
            ORDER BY id ASC
            LIMIT 500
            "#,
        )
        .bind(note_id)
        .bind(last_known_id)
        .fetch_all(&self.pool)
        .await;

        match ops {
            Ok(ops) if !ops.is_empty() => {
                let batch = WsMessage::OpSyncBatch {
                    note_id,
                    ops,
                    timestamp: Utc::now(),
                };
                if let Ok(json) = serde_json::to_string(&batch) {
                    let _ = ws_tx.send(Message::Text(json));
                }
            }
            _ => {}
        }
    }

    // ── Block CRDT handlers ──

    async fn handle_block_add(
        self: &Arc<Self>,
        note_id: Uuid,
        block_id: Uuid,
        block_type: &str,
        data: &serde_json::Value,
        position: i32,
        parent_id: Option<Uuid>,
        client_id: &str,
    ) {
        use crate::utils::validation;
        if let Err(e) = validation::validate_block_type(block_type)
            .and_then(|_| validation::validate_block_position(position))
            .and_then(|_| validation::validate_block_data(Some(data), 512 * 1024))
        {
            tracing::warn!("Rejected block:add from client {}: {}", client_id, e);
            return;
        }

        let result = sqlx::query(
            r#"
            INSERT INTO note_blocks (id, note_id, block_type, data, position, parent_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (id) DO UPDATE SET
                block_type = EXCLUDED.block_type,
                data = EXCLUDED.data,
                position = EXCLUDED.position,
                parent_id = EXCLUDED.parent_id,
                updated_at = NOW()
            "#,
        )
        .bind(block_id)
        .bind(note_id)
        .bind(block_type)
        .bind(data)
        .bind(position)
        .bind(parent_id)
        .execute(&self.pool)
        .await;

        if let Err(e) = result {
            tracing::error!("Failed to insert block {}: {}", block_id, e);
            return;
        }

        self.rebuild_note_content(note_id).await;

        self.publish(&WsMessage::BlockAdd {
            note_id,
            block_id,
            block_type: block_type.to_string(),
            data: data.clone(),
            position,
            parent_id,
            client_id: client_id.to_string(),
            timestamp: Utc::now(),
        });
    }

    async fn handle_block_update(
        self: &Arc<Self>,
        note_id: Uuid,
        block_id: Uuid,
        data: &serde_json::Value,
        client_id: &str,
    ) {
        let result = sqlx::query(
            "UPDATE note_blocks SET data = $1, updated_at = NOW() WHERE id = $2 AND note_id = $3",
        )
        .bind(data)
        .bind(block_id)
        .bind(note_id)
        .execute(&self.pool)
        .await;

        if let Err(e) = result {
            tracing::error!("Failed to update block {}: {}", block_id, e);
            return;
        }

        self.rebuild_note_content(note_id).await;

        self.publish(&WsMessage::BlockUpdate {
            note_id,
            block_id,
            data: data.clone(),
            client_id: client_id.to_string(),
            timestamp: Utc::now(),
        });
    }

    async fn handle_block_remove(
        self: &Arc<Self>,
        note_id: Uuid,
        block_id: Uuid,
        client_id: &str,
    ) {
        let result = sqlx::query("DELETE FROM note_blocks WHERE id = $1 AND note_id = $2")
            .bind(block_id)
            .bind(note_id)
            .execute(&self.pool)
            .await;

        if let Err(e) = result {
            tracing::error!("Failed to delete block {}: {}", block_id, e);
            return;
        }

        self.rebuild_note_content(note_id).await;

        self.publish(&WsMessage::BlockRemove {
            note_id,
            block_id,
            client_id: client_id.to_string(),
            timestamp: Utc::now(),
        });
    }

    async fn handle_block_move(
        self: &Arc<Self>,
        note_id: Uuid,
        block_id: Uuid,
        new_position: i32,
        new_parent_id: Option<Uuid>,
        client_id: &str,
    ) {
        let result = sqlx::query(
            "UPDATE note_blocks SET position = $1, parent_id = $2, updated_at = NOW() WHERE id = $3 AND note_id = $4",
        )
        .bind(new_position)
        .bind(new_parent_id)
        .bind(block_id)
        .bind(note_id)
        .execute(&self.pool)
        .await;

        if let Err(e) = result {
            tracing::error!("Failed to move block {}: {}", block_id, e);
            return;
        }

        self.publish(&WsMessage::BlockMove {
            note_id,
            block_id,
            new_position,
            new_parent_id,
            client_id: client_id.to_string(),
            timestamp: Utc::now(),
        });
    }

    async fn handle_block_sync_request(
        self: &Arc<Self>,
        note_id: Uuid,
        ws_tx: &tokio::sync::mpsc::UnboundedSender<Message>,
    ) {
        let blocks = sqlx::query_as::<_, crate::models::block::Block>(
            r#"
            SELECT id, note_id, block_type, data, position, parent_id, created_at, updated_at
            FROM note_blocks
            WHERE note_id = $1
            ORDER BY position ASC, created_at ASC
            "#,
        )
        .bind(note_id)
        .fetch_all(&self.pool)
        .await;

        let snapshots: Vec<BlockSnapshot> = match blocks {
            Ok(rows) => rows
                .into_iter()
                .map(|b| BlockSnapshot {
                    id: b.id,
                    block_type: b.block_type,
                    data: b.data,
                    position: b.position,
                    parent_id: b.parent_id,
                })
                .collect(),
            Err(e) => {
                tracing::error!("Failed to fetch blocks for sync: {}", e);
                return;
            }
        };

        let msg = WsMessage::BlockSyncBatch {
            note_id,
            blocks: snapshots,
            timestamp: Utc::now(),
        };

        if let Ok(json) = serde_json::to_string(&msg) {
            let _ = ws_tx.send(Message::Text(json));
        }
    }

    async fn rebuild_note_content(&self, note_id: Uuid) {
        let blocks = sqlx::query_as::<_, crate::models::block::Block>(
            r#"
            SELECT id, note_id, block_type, data, position, parent_id, created_at, updated_at
            FROM note_blocks
            WHERE note_id = $1
            ORDER BY position ASC, created_at ASC
            "#,
        )
        .bind(note_id)
        .fetch_all(&self.pool)
        .await;

        let plain_text = match blocks {
            Ok(blocks) => crate::services::note_service::blocks_to_plain_text(&blocks),
            Err(e) => {
                tracing::error!("Failed to fetch blocks for content rebuild: {}", e);
                return;
            }
        };

        if let Err(e) = sqlx::query("UPDATE notes SET content = $1, updated_at = NOW() WHERE id = $2")
            .bind(&plain_text)
            .bind(note_id)
            .execute(&self.pool)
            .await
        {
            tracing::error!("Failed to rebuild note content: {}", e);
        }
    }

    /// Returns true if the user may apply write operations to the note
    /// (owner, or collaborator with 'write'/'admin'/'edit' permission).
    async fn verify_write_access(&self, note_id: Uuid, user_id: Uuid) -> bool {
        let is_owner: Option<bool> = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM notes WHERE id = $1 AND user_id = $2 AND is_deleted = false)",
        )
        .bind(note_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(None);

        if is_owner.unwrap_or(false) {
            return true;
        }

        let is_writer: Option<bool> = sqlx::query_scalar(
            "SELECT EXISTS(
                SELECT 1 FROM note_collaborators
                WHERE note_id = $1 AND user_id = $2
                  AND permission IN ('write', 'admin', 'edit')
            )",
        )
        .bind(note_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(None);

        is_writer.unwrap_or(false)
    }

    pub async fn handle_connection(self: Arc<Self>, ws: WebSocket, user: User, note_id: Uuid) {
        if !self.try_acquire_connection() {
            return;
        }

        // Per-user connection limit
        {
            let mut user_conns = self.user_connections.write().await;
            let count = user_conns.entry(user.id).or_insert(0);
            if *count >= self.max_connections_per_user {
                self.release_connection();
                tracing::warn!(
                    "Connection rejected for user {}: {} active connections exceeds limit of {}",
                    user.id,
                    *count,
                    self.max_connections_per_user
                );
                return;
            }
            *count += 1;
        }

        if let Err(e) = self.verify_note_access(note_id, user.id).await {
            self.release_connection();
            {
                let mut user_conns = self.user_connections.write().await;
                if let Some(count) = user_conns.get_mut(&user.id) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        user_conns.remove(&user.id);
                    }
                }
            }
            tracing::warn!(
                "Access denied for user {} to note {}: {}",
                user.id,
                note_id,
                e
            );
            return;
        }

        self.ensure_redis_subscriber(note_id).await;

        let (sender, mut receiver) = ws.split();
        let rx = self.tx.subscribe();

        if let Err(e) = self.create_or_update_session(note_id, user.id, 0, 0).await {
            tracing::error!("Failed to create session: {}", e);
            self.release_connection();
            return;
        }

        let join_msg = WsMessage::UserJoined {
            note_id,
            user_id: user.id,
            user_name: user.display_name.clone(),
            timestamp: Utc::now(),
        };
        self.publish(&join_msg);

        let user_id = user.id;
        let user_name = user.display_name.clone();
        let (ws_tx, ws_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

        let sender_task = Self::spawn_sender_task(sender, ws_rx);
        let broadcast_task = Self::spawn_broadcast_task(rx, ws_tx.clone(), note_id, user_id);

        while let Some(result) = receiver.next().await {
            match result {
                Ok(Message::Text(text)) => {
                    self.handle_incoming_message(note_id, user_id, &user_name, text, &ws_tx)
                        .await;
                }
                Ok(Message::Close(_)) => break,
                Ok(Message::Ping(data)) => {
                    let _ = ws_tx.send(Message::Pong(data));
                }
                Ok(Message::Binary(_)) => {
                    tracing::debug!("Ignoring unexpected binary frame from user {}", user_id);
                }
                Ok(Message::Pong(_)) => {
                    // unsolicited pong — ignore silently
                }
                Err(e) => {
                    tracing::warn!(
                        "WebSocket closed for user {} on note {} (reason: {})",
                        user_id, note_id, e
                    );
                    break;
                }
            }
        }

        self.cleanup_connection(note_id, user_id, sender_task, broadcast_task)
            .await;
    }

    async fn verify_note_access(&self, note_id: Uuid, user_id: Uuid) -> Result<()> {
        let is_owner = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM notes WHERE id = $1 AND user_id = $2 AND is_deleted = false)",
            note_id,
            user_id
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(false);

        if is_owner {
            return Ok(());
        }

        let is_collaborator: Option<bool> = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM note_collaborators WHERE note_id = $1 AND user_id = $2)",
        )
        .bind(note_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        if !is_collaborator.unwrap_or(false) {
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
