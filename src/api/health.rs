// src/api/health.rs
use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde_json::{Value, json};

use super::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/health", get(health))
}

async fn health(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    match sqlx::query_scalar!("SELECT 1 AS \"one!\"").fetch_one(&state.pool).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "status": "ok" }))),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, Json(json!({ "status": "database unavailable" }))),
    }
}
