use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    Validation(String),
    #[error("missing or invalid API key")]
    Unauthorized,
    #[error("this API key is not allowed to do that")]
    Forbidden,
    #[error("this account is paused: {0}")]
    AccountPaused(String),
    #[error("not found")]
    NotFound,
    #[error("{1}")]
    Conflict(&'static str, String),
    #[error("the request is too large")]
    TooLarge,
    #[error("{1}")]
    Unprocessable(&'static str, String),
    #[error("too many requests, slow down")]
    RateLimited,
    #[error("internal error")]
    Internal(#[from] anyhow::Error),
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        ApiError::Internal(e.into())
    }
}

impl ApiError {
    pub fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            ApiError::Validation(_) => (StatusCode::BAD_REQUEST, "validation_error"),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            ApiError::AccountPaused(_) => (StatusCode::FORBIDDEN, "account_paused"),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            ApiError::Conflict(code, _) => (StatusCode::CONFLICT, code),
            ApiError::TooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large"),
            ApiError::Unprocessable(code, _) => (StatusCode::UNPROCESSABLE_ENTITY, code),
            ApiError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "rate_limited"),
            ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if let ApiError::Internal(e) = &self {
            tracing::error!(error = ?e, "internal error");
        }
        let (status, code) = self.status_and_code();
        let body = json!({ "error": { "code": code, "message": self.to_string() } });
        (status, Json(body)).into_response()
    }
}
