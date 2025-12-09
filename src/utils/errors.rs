use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    DatabaseError(sqlx::Error),
    RedisError(redis::RedisError),
    AuthenticationError(String),
    ValidationError(String),
    NotFound(String),
    Forbidden(String),
    InternalError(String),
    BadRequest(String),
    Conflict(String),
    RateLimitExceeded,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::DatabaseError(e) => write!(f, "Database error: {}", e),
            AppError::RedisError(e) => write!(f, "Redis error: {}", e),
            AppError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            AppError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            AppError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            AppError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            AppError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            AppError::RateLimitExceeded => write!(f, "Rate limit exceeded"),
        }
    }
}

// Add this after the Display implementation:
impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::DatabaseError(e) => Some(e),
            AppError::RedisError(e) => Some(e),
            _ => None,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::DatabaseError(e) => {
                tracing::error!("Database error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error occurred".to_string())
            }
            AppError::RedisError(e) => {
                tracing::error!("Redis error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Cache error occurred".to_string())
            }
            AppError::AuthenticationError(msg) => (StatusCode::UNAUTHORIZED, msg),
            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            AppError::InternalError(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            AppError::RateLimitExceeded => {
                (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded".to_string())
            }
        };

        let body = Json(json!({
            "error": error_message,
            "status": status.as_u16()
        }));

        (status, body).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        AppError::DatabaseError(error)
    }
}

impl From<redis::RedisError> for AppError {
    fn from(error: redis::RedisError) -> Self {
        AppError::RedisError(error)
    }
}

pub type Result<T> = std::result::Result<T, AppError>;