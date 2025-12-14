// src/handlers/websocket.rs
use axum::{
    extract::{ws::WebSocketUpgrade, Path, State},
    response::Response,
    Extension,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::models::user::User;
use crate::services::CollaborationService;

/// GET /api/v1/notes/:id/ws
pub async fn note_websocket_handler(
    ws: WebSocketUpgrade,
    State(collab_service): State<Arc<CollaborationService>>,
    Extension(user): Extension<User>,
    Path(note_id): Path<Uuid>,
) -> Response {
    tracing::info!(
        "WebSocket upgrade request for note {} from user {}",
        note_id,
        user.id
    );

    ws.on_upgrade(move |socket| collab_service.handle_connection(socket, user, note_id))
}
