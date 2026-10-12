use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, post},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{AppState, auth::Auth, json::ApiJson};
use crate::{
    error::ApiError,
    keys::{self, KeyAccess, KeySummary},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api-keys", post(create).get(list))
        .route("/api-keys/{id}", delete(remove))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateKeyRequest {
    pub name: String,
    pub access: Option<KeyAccess>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct CreatedKey {
    pub id: Uuid,
    pub name: String,
    /// The full key. Shown only in this response.
    pub token: String,
    pub prefix: String,
    pub access: KeyAccess,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct KeyList {
    pub data: Vec<KeySummary>,
}

pub(crate) async fn create(
    State(state): State<AppState>,
    auth: Auth,
    ApiJson(body): ApiJson<CreateKeyRequest>,
) -> Result<(StatusCode, Json<CreatedKey>), ApiError> {
    auth.require_full()?;
    let name = body.name.trim().to_string();
    if name.is_empty() || name.len() > 100 {
        return Err(ApiError::Validation("name must be 1-100 characters".into()));
    }
    let k = keys::create(
        &state.pool,
        auth.account_id(),
        &name,
        body.access.unwrap_or(KeyAccess::Full),
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(CreatedKey {
            id: k.id,
            name,
            token: k.token,
            prefix: k.prefix,
            access: k.access,
        }),
    ))
}

pub(crate) async fn list(
    State(state): State<AppState>,
    auth: Auth,
) -> Result<Json<KeyList>, ApiError> {
    auth.require_full()?;
    Ok(Json(KeyList {
        data: keys::list(&state.pool, auth.account_id()).await?,
    }))
}

pub(crate) async fn remove(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    auth.require_full()?;
    if keys::revoke(&state.pool, Some(auth.account_id()), id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}
