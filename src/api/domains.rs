use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::Deserialize;
use uuid::Uuid;

use super::{AppState, auth::Auth, json::ApiJson};
use crate::{
    accounts,
    domains::{self, AddDomain, AddDomainError, Domain},
    error::ApiError,
    sender::SenderError,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/domains", post(create).get(list))
        .route("/domains/{id}", get(get_one).patch(update).delete(remove))
        .route("/domains/{id}/verify", post(verify))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateDomainRequest {
    pub name: String,
    pub tracking_subdomain: Option<String>,
    #[serde(default)]
    pub open_tracking: bool,
    #[serde(default)]
    pub click_tracking: bool,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateDomainRequest {
    pub open_tracking: Option<bool>,
    pub click_tracking: Option<bool>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct DomainList {
    pub data: Vec<Domain>,
}

pub(crate) async fn create(
    State(state): State<AppState>,
    auth: Auth,
    ApiJson(body): ApiJson<CreateDomainRequest>,
) -> Result<(StatusCode, Json<Domain>), ApiError> {
    auth.require_full()?;
    let account = accounts::get(&state.pool, auth.account_id()).await?.ok_or(ApiError::Unauthorized)?;
    let input = AddDomain {
        name: body.name,
        tracking_subdomain: body.tracking_subdomain,
        open_tracking: body.open_tracking,
        click_tracking: body.click_tracking,
    };
    match domains::add(&state, &account, input).await {
        Ok(d) => Ok((StatusCode::CREATED, Json(d))),
        Err(AddDomainError::Invalid(m)) => Err(ApiError::Validation(m)),
        Err(AddDomainError::Exists) => Err(ApiError::Conflict("domain_exists", "this domain is already added".into())),
        Err(AddDomainError::Sender(SenderError::Permanent(m))) => Err(ApiError::Unprocessable("sender_rejected", m)),
        Err(AddDomainError::Sender(e)) => Err(ApiError::Internal(e.into())),
        Err(AddDomainError::Db(e)) => Err(e.into()),
    }
}

pub(crate) async fn list(State(state): State<AppState>, auth: Auth) -> Result<Json<DomainList>, ApiError> {
    auth.require_full()?;
    Ok(Json(DomainList { data: domains::list(&state.pool, auth.account_id()).await? }))
}

async fn owned(state: &AppState, auth: &Auth, id: Uuid) -> Result<Domain, ApiError> {
    domains::get_for_account(&state.pool, auth.account_id(), id).await?.ok_or(ApiError::NotFound)
}

pub(crate) async fn get_one(State(state): State<AppState>, auth: Auth, Path(id): Path<Uuid>) -> Result<Json<Domain>, ApiError> {
    auth.require_full()?;
    Ok(Json(owned(&state, &auth, id).await?))
}

pub(crate) async fn update(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
    ApiJson(body): ApiJson<UpdateDomainRequest>,
) -> Result<Json<Domain>, ApiError> {
    auth.require_full()?;
    let d = owned(&state, &auth, id).await?;
    sqlx::query!(
        "UPDATE domains SET open_tracking = $2, click_tracking = $3 WHERE id = $1",
        d.id,
        body.open_tracking.unwrap_or(d.open_tracking),
        body.click_tracking.unwrap_or(d.click_tracking)
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(owned(&state, &auth, id).await?))
}

pub(crate) async fn remove(State(state): State<AppState>, auth: Auth, Path(id): Path<Uuid>) -> Result<StatusCode, ApiError> {
    auth.require_full()?;
    let d = owned(&state, &auth, id).await?;
    // shortcut: the identity stays at the sender (SES); delete it there by hand if needed.
    sqlx::query!("DELETE FROM domains WHERE id = $1", d.id).execute(&state.pool).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn verify(State(state): State<AppState>, auth: Auth, Path(id): Path<Uuid>) -> Result<Json<Domain>, ApiError> {
    auth.require_full()?;
    let d = owned(&state, &auth, id).await?;
    Ok(Json(domains::check(&state, &d).await?))
}
