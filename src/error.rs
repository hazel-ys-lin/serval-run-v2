use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// Application error type that can be returned from handlers
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    // Authentication errors
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Token expired")]
    TokenExpired,

    #[error("Unauthorized")]
    Unauthorized,

    // Resource errors
    #[error("{0} not found")]
    NotFound(String),

    #[error("{0} already exists")]
    Conflict(String),

    // Validation errors
    #[error("Validation error: {0}")]
    Validation(String),

    // Database errors
    #[error("Database error: {0}")]
    Database(String),

    // Internal errors
    #[error("Internal server error")]
    Internal(String),

    // Queue errors
    #[error("Queue error: {0}")]
    Queue(String),
}

/// JSON error response body
#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message, details) = match &self {
            // 401 Unauthorized
            AppError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "Invalid credentials", None),
            AppError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid token", None),
            AppError::TokenExpired => (StatusCode::UNAUTHORIZED, "Token expired", None),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized", None),

            // 404 Not Found
            AppError::NotFound(resource) => {
                (StatusCode::NOT_FOUND, "Not found", Some(resource.clone()))
            }

            // 409 Conflict
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "Conflict", Some(msg.clone())),

            // 400 Bad Request
            AppError::Validation(msg) => (
                StatusCode::BAD_REQUEST,
                "Validation error",
                Some(msg.clone()),
            ),

            // 500 Internal Server Error
            AppError::Database(msg) => {
                tracing::error!("Database error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error", None)
            }
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error",
                    None,
                )
            }
            AppError::Queue(msg) => {
                tracing::error!("Queue error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Queue error", None)
            }
        };

        let body = Json(ErrorResponse {
            error: error_message.to_string(),
            details,
        });

        (status, body).into_response()
    }
}

// Convenient conversions from common error types

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound("Resource".to_string()),
            _ => AppError::Database(err.to_string()),
        }
    }
}

impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        match err {
            sea_orm::DbErr::RecordNotFound(_) => AppError::NotFound("Resource".to_string()),
            sea_orm::DbErr::RecordNotInserted => {
                AppError::Conflict("Record already exists".to_string())
            }
            sea_orm::DbErr::RecordNotUpdated => AppError::NotFound("Resource".to_string()),
            _ => AppError::Database(err.to_string()),
        }
    }
}

impl From<argon2::password_hash::Error> for AppError {
    fn from(_: argon2::password_hash::Error) -> Self {
        AppError::InvalidCredentials
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        match err.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AppError::TokenExpired,
            _ => AppError::InvalidToken,
        }
    }
}

/// Result type alias for handlers
pub type AppResult<T> = Result<T, AppError>;
