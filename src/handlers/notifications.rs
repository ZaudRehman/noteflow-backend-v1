use crate::models::user::User;
use crate::services::NotificationService;
use crate::utils::errors::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct PushSubscribeRequest {
    pub endpoint: String,
    pub p256dh_key: String,
    pub auth_key: String,
    pub user_agent: Option<String>,
}

/// POST /api/v1/notifications/push/subscribe
pub async fn push_subscribe(
    State(notification_service): State<Arc<NotificationService>>,
    Extension(user): Extension<User>,
    Json(req): Json<PushSubscribeRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let sub = notification_service
        .subscribe(user.id, &req.endpoint, &req.p256dh_key, &req.auth_key, req.user_agent)
        .await?;

    tracing::info!("User {} subscribed to push notifications", user.id);
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": sub.id, "message": "Subscribed" })),
    ))
}

/// DELETE /api/v1/notifications/push/subscribe/:id
pub async fn push_unsubscribe(
    State(notification_service): State<Arc<NotificationService>>,
    Extension(user): Extension<User>,
    Path(subscription_id): Path<Uuid>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    notification_service
        .unsubscribe(subscription_id, user.id)
        .await?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "message": "Unsubscribed" })),
    ))
}

/// GET /api/v1/notifications/push/subscriptions
pub async fn list_push_subscriptions(
    State(notification_service): State<Arc<NotificationService>>,
    Extension(user): Extension<User>,
) -> Result<Json<serde_json::Value>> {
    let subs = notification_service.list_subscriptions(user.id).await?;
    Ok(Json(serde_json::json!({ "subscriptions": subs })))
}
