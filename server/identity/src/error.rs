//! Error types for Identity Server

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

/// API Error response
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestions: Option<Vec<String>>,
}

/// Application errors
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Invalid username format: {0}")]
    InvalidUsername(String),

    #[error("Username already taken: {0}")]
    UsernameTaken(String),

    #[error("Username not found: {0}")]
    UsernameNotFound(String),

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidUsername(_) => StatusCode::BAD_REQUEST,
            Self::UsernameTaken(_) => StatusCode::CONFLICT,
            Self::UsernameNotFound(_) => StatusCode::NOT_FOUND,
            Self::InvalidSignature => StatusCode::BAD_REQUEST,
            Self::RateLimitExceeded => StatusCode::TOO_MANY_REQUESTS,
            Self::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Redis(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_code(&self) -> &str {
        match self {
            Self::InvalidUsername(_) => "INVALID_USERNAME",
            Self::UsernameTaken(_) => "USERNAME_TAKEN",
            Self::UsernameNotFound(_) => "USERNAME_NOT_FOUND",
            Self::InvalidSignature => "INVALID_SIGNATURE",
            Self::RateLimitExceeded => "RATE_LIMIT_EXCEEDED",
            Self::Database(_) => "INTERNAL_ERROR",
            Self::Redis(_) => "INTERNAL_ERROR",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }

    fn generate_username_suggestions(&self, username: &str) -> Vec<String> {
        vec![
            format!("{}_", username),
            format!("{}1", username),
            format!("{}_alt", username),
        ]
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let suggestions = match &self {
            Self::UsernameTaken(username) => Some(self.generate_username_suggestions(username)),
            _ => None,
        };

        let error_response = ErrorResponse {
            error: self.error_code().to_string(),
            message: self.to_string(),
            suggestions,
        };

        (self.status_code(), Json(error_response)).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
