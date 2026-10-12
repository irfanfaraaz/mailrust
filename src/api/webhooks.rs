use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, post},
};
use serde::Deserialize;
use uuid::Uuid;

use super::{AppState, auth::Auth, json::ApiJson};
use crate::{
    error::ApiError,
    webhooks::{self, EVENT_TYPES, WebhookView, ssrf},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/webhooks", post(create).get(list))
        .route("/webhooks/{id}", delete(remove))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateWebhookRequest {
    pub url: String,
    pub events: Vec<String>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct WebhookList {
    pub data: Vec<WebhookView>,
}

pub(crate) async fn create(
    State(state): State<AppState>,
    auth: Auth,
    ApiJson(body): ApiJson<CreateWebhookRequest>,
) -> Result<(StatusCode, Json<WebhookView>), ApiError> {
    auth.require_full()?;
    let mut events = body.events;
    events.sort();
    events.dedup();
    if events.is_empty() {
        return Err(ApiError::Validation(
            "events must list at least one event type".into(),
        ));
    }
    if let Some(bad) = events.iter().find(|e| !EVENT_TYPES.contains(&e.as_str())) {
        return Err(ApiError::Validation(format!(
            "unknown event type {bad}; allowed: {}",
            EVENT_TYPES.join(", ")
        )));
    }
    if body.url.len() > 2048 {
        return Err(ApiError::Validation("url is too long".into()));
    }
    ssrf::check_url(&body.url, state.config.allow_private_webhooks)
        .await
        .map_err(ApiError::Validation)?;
    let view = webhooks::create(&state.pool, auth.account_id(), &body.url, &events).await?;
    Ok((StatusCode::CREATED, Json(view)))
}

pub(crate) async fn list(
    State(state): State<AppState>,
    auth: Auth,
) -> Result<Json<WebhookList>, ApiError> {
    auth.require_full()?;
    Ok(Json(WebhookList {
        data: webhooks::list(&state.pool, auth.account_id()).await?,
    }))
}

pub(crate) async fn remove(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    auth.require_full()?;
    if webhooks::delete(&state.pool, auth.account_id(), id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}
