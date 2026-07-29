use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::RedisManager;
use crate::models::user::User;
use crate::utils::errors::Result;

#[derive(Deserialize)]
pub struct CreateTicketRequest {
    pub note_id: Uuid,
}

#[derive(Serialize)]
pub struct CreateTicketResponse {
    pub ticket: String,
    pub expires_in: u32,
}

/// POST /api/v1/ws/ticket
pub async fn create_ws_ticket(
    Extension(user): Extension<User>,
    Extension(redis_manager): Extension<Arc<RedisManager>>,
    Json(body): Json<CreateTicketRequest>,
) -> Result<Json<CreateTicketResponse>> {
    let ticket = format!("{}{}", Uuid::new_v4().to_string().replace('-', ""), Uuid::new_v4().to_string().replace('-', ""));

    let value = format!("{}:{}", user.id, body.note_id);
    redis_manager.set_with_expiry(&format!("ws_ticket:{}", ticket), &value, 30).await?;

    Ok(Json(CreateTicketResponse {
        ticket,
        expires_in: 30,
    }))
}
