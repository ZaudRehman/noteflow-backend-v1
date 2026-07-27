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
        | WsMessage::OpSyncBatch { note_id: nid, .. } => *nid == note_id,
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
                    | WsMessage::UserLeft { note_id, .. }
                    | WsMessage::OpInsert { note_id, .. }
                    | WsMessage::OpDelete { note_id, .. }
                    | WsMessage::OpSyncBatch { note_id, .. } => note_id,
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

    async fn store_op(&self, note_id: Uuid, client_id: &str, op_type: &str, position: usize, text: Option<&str>, length: Option<usize>) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            r#"
            INSERT INTO collab_operations (note_id, client_id, op_type, position, text_content, length)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id
            "#,
        )
        .bind(note_id)
        .bind(client_id)
        .bind(op_type)
        .bind(position as i32)
        .bind(text)
        .bind(length.map(|l| l as i32))
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    async fn handle_op_insert(
        self: &Arc<Self>,
        note_id: Uuid,
        client_id: &str,
        position: usize,
        text: &str,
    ) {
        if let Ok(op_id) = self.store_op(note_id, client_id, "insert", position, Some(text), None).await {
            tracing::trace!("Stored op:insert id={} note={} client={}", op_id, note_id, client_id);
        }
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
        if let Ok(op_id) = self.store_op(note_id, client_id, "delete", position, None, Some(length)).await {
            tracing::trace!("Stored op:delete id={} note={} client={}", op_id, note_id, client_id);
        }
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

    /// Periodically apply unapplied collab ops to notes.content and mark them applied.
    async fn apply_pending_ops(&self) {
        let unapplied = sqlx::query_as::<_, (Uuid, i64, String, i32, Option<String>, Option<i32>)>(
            r#"
            SELECT c.note_id, c.id, c.op_type, c.position, c.text_content, c.length
            FROM collab_operations c
            WHERE NOT c.applied
            ORDER BY c.note_id, c.id
            LIMIT 1000
            "#,
        )
        .fetch_all(&self.pool)
        .await;

        let unapplied = match unapplied {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!("Failed to fetch unapplied ops: {}", e);
                return;
            }
        };

        if unapplied.is_empty() {
            return;
        }

        use std::collections::HashMap;
        let mut by_note: HashMap<Uuid, Vec<(i64, String, i32, Option<String>, Option<i32>)>> = HashMap::new();
        for (note_id, id, op_type, position, text_content, length) in unapplied {
            by_note.entry(note_id).or_default().push((id, op_type, position, text_content, length));
        }

        for (note_id, ops) in &by_note {
            let current: Option<String> = sqlx::query_scalar(
                "SELECT content FROM notes WHERE id = $1",
            )
            .bind(note_id)
            .fetch_optional(&self.pool)
            .await
            .unwrap_or(None);

            let mut content = current.unwrap_or_default();

            for (_id, op_type, position, text_content, length) in ops {
                let pos = *position as usize;
                if pos > content.len() {
                    continue;
                }
                match op_type.as_str() {
                    "insert" => {
                        if let Some(text) = text_content {
                            content.insert_str(pos, text);
                        }
                    }
                    "delete" => {
                        let len = length.unwrap_or(0) as usize;
                        let end = (pos + len).min(content.len());
                        content.drain(pos..end);
                    }
                    _ => {}
                }
            }

            if let Err(e) = sqlx::query("UPDATE notes SET content = $1, updated_at = NOW() WHERE id = $2")
                .bind(&content)
                .bind(note_id)
                .execute(&self.pool)
                .await
            {
                tracing::error!("Failed to update note content for {}: {}", note_id, e);
                continue;
            }

            let ids: Vec<i64> = ops.iter().map(|(id, ..)| *id).collect();
            if let Err(e) = sqlx::query(
                "UPDATE collab_operations SET applied = true WHERE id = ANY($1)",
            )
            .bind(&ids)
            .execute(&self.pool)
            .await
            {
                tracing::error!("Failed to mark ops applied: {}", e);
            }
        }
    }

    pub fn spawn_background_tasks(self: &Arc<Self>) {
        let this = Arc::clone(self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            loop {
                interval.tick().await;
                this.apply_pending_ops().await;
            }
        });
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
                Err(e) => {
                    tracing::error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
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
