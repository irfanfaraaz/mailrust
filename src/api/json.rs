// src/api/json.rs
use axum::{extract::{FromRequest, rejection::JsonRejection}, http::StatusCode};

use crate::error::ApiError;

/// Like axum::Json, but bad or too-large bodies become our normal error shape.
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(ApiError))]
pub struct ApiJson<T>(pub T);

impl From<JsonRejection> for ApiError {
    fn from(r: JsonRejection) -> Self {
        if r.status() == StatusCode::PAYLOAD_TOO_LARGE {
            ApiError::TooLarge
        } else {
            ApiError::Validation(r.body_text())
        }
    }
}
