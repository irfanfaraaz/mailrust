use axum::{
    Json, Router,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::StatusCode,
    routing::{delete, get},
};
use chrono::{DateTime, Utc};
use lettre::message::Mailbox;
use serde::{Deserialize, Serialize};

use super::{AppState, auth::Auth, json::ApiJson};
use crate::{
    error::ApiError,
    render::bare_address,
    safety::{self, SuppressionReason},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/suppressions", get(list).post(add))
        .route("/suppressions/{address}", delete(remove))
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SuppressionView {
    pub address: String,
    pub reason: SuppressionReason,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SuppressionList {
    pub data: Vec<SuppressionView>,
    pub next_cursor: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct AddSuppressionRequest {
    pub address: String,
}

#[derive(Deserialize, utoipa::IntoParams)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub after: Option<String>,
}

pub(crate) async fn list(
    State(state): State<AppState>,
    auth: Auth,
    q: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Json<SuppressionList>, ApiError> {
    auth.require_full()?;
    let Query(q) = q.map_err(|e| ApiError::Validation(e.body_text()))?;
    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let data = sqlx::query_as!(
        SuppressionView,
        r#"SELECT address, reason AS "reason: SuppressionReason", created_at FROM suppressions
           WHERE account_id = $1 AND ($2::text IS NULL OR address > $2)
           ORDER BY address LIMIT $3"#,
        auth.account_id(),
        q.after,
        limit
    )
    .fetch_all(&state.pool)
    .await?;
    let next_cursor = if data.len() as i64 == limit {
        data.last().map(|s| s.address.clone())
    } else {
        None
    };
    Ok(Json(SuppressionList { data, next_cursor }))
}

pub(crate) async fn add(
    State(state): State<AppState>,
    auth: Auth,
    ApiJson(body): ApiJson<AddSuppressionRequest>,
) -> Result<(StatusCode, Json<SuppressionView>), ApiError> {
    auth.require_full()?;
    body.address
        .parse::<Mailbox>()
        .map_err(|_| ApiError::Validation(format!("{} is not a valid address", body.address)))?;
    let mut conn = state.pool.acquire().await?;
    safety::add_suppression(
        &mut conn,
        auth.account_id(),
        &body.address,
        SuppressionReason::Manual,
        None,
    )
    .await?;
    let view = sqlx::query_as!(
        SuppressionView,
        r#"SELECT address, reason AS "reason: SuppressionReason", created_at FROM suppressions WHERE account_id = $1 AND address = $2"#,
        auth.account_id(),
        bare_address(&body.address)
    )
    .fetch_one(&mut *conn)
    .await?;
    Ok((StatusCode::CREATED, Json(view)))
}

/// Removes the address from Mailrust's list. It does not remove it from SES's own tenant list.
pub(crate) async fn remove(
    State(state): State<AppState>,
    auth: Auth,
    Path(address): Path<String>,
) -> Result<StatusCode, ApiError> {
    auth.require_full()?;
    let r = sqlx::query!(
        "DELETE FROM suppressions WHERE account_id = $1 AND address = $2",
        auth.account_id(),
        bare_address(&address)
    )
    .execute(&state.pool)
    .await?;
    if r.rows_affected() == 1 {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}
