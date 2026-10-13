use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{AppState, auth::Auth, json::ApiJson};
use crate::{contacts, error::ApiError};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/tags", get(list).post(create))
        .route("/tags/{id}", patch(rename).delete(remove))
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct TagView {
    pub id: Uuid,
    pub name: String,
    pub contacts: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct TagList {
    pub data: Vec<TagView>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct TagRequest {
    pub name: String,
}

fn one_name(raw: &str) -> Result<String, ApiError> {
    let mut v = contacts::clean_tags(&[raw.to_string()]).map_err(ApiError::Validation)?;
    Ok(v.remove(0))
}

fn duplicate(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(d) if d.is_unique_violation() && d.constraint() == Some("tags_name"))
}

fn exists() -> ApiError {
    ApiError::Conflict("tag_exists", "a tag with this name already exists".into())
}

pub(crate) async fn list(State(state): State<AppState>, auth: Auth) -> Result<Json<TagList>, ApiError> {
    auth.require_full()?;
    let data = sqlx::query_as!(
        TagView,
        r#"SELECT t.id, t.name, t.created_at,
                  (SELECT count(*) FROM contact_tags ct WHERE ct.tag_id = t.id) AS "contacts!"
           FROM tags t WHERE t.account_id = $1 ORDER BY lower(t.name)"#,
        auth.account_id()
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(TagList { data }))
}

pub(crate) async fn create(
    State(state): State<AppState>,
    auth: Auth,
    ApiJson(body): ApiJson<TagRequest>,
) -> Result<(StatusCode, Json<TagView>), ApiError> {
    auth.require_full()?;
    let name = one_name(&body.name)?;
    let tag = sqlx::query_as!(
        TagView,
        r#"INSERT INTO tags (id, account_id, name) VALUES ($1, $2, $3)
           RETURNING id, name, created_at, 0::bigint AS "contacts!""#,
        Uuid::now_v7(),
        auth.account_id(),
        name
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| if duplicate(&e) { exists() } else { e.into() })?;
    Ok((StatusCode::CREATED, Json(tag)))
}

pub(crate) async fn rename(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
    ApiJson(body): ApiJson<TagRequest>,
) -> Result<Json<TagView>, ApiError> {
    auth.require_full()?;
    let name = one_name(&body.name)?;
    sqlx::query_as!(
        TagView,
        r#"UPDATE tags t SET name = $3 WHERE t.id = $1 AND t.account_id = $2
           RETURNING t.id, t.name, t.created_at,
                     (SELECT count(*) FROM contact_tags ct WHERE ct.tag_id = t.id) AS "contacts!""#,
        id,
        auth.account_id(),
        name
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| if duplicate(&e) { exists() } else { e.into() })?
    .map(Json)
    .ok_or(ApiError::NotFound)
}

pub(crate) async fn remove(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    auth.require_full()?;
    let r = sqlx::query!("DELETE FROM tags WHERE id = $1 AND account_id = $2", id, auth.account_id())
        .execute(&state.pool)
        .await?;
    if r.rows_affected() == 1 { Ok(StatusCode::NO_CONTENT) } else { Err(ApiError::NotFound) }
}
