use std::collections::HashMap;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{
        Path, Query, State,
        rejection::{BytesRejection, QueryRejection},
    },
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{AppState, auth::Auth};
use crate::{
    domains::{self, DomainStatus},
    emails::{
        self, EmailStatus, EmailSummary, EmailView, MAX_BATCH, Priority, SendEmailRequest,
        SendResponse, ValidEmail,
    },
    error::ApiError,
};

pub const IDEMPOTENCY_HOURS: i64 = 24;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/emails", post(send).get(list))
        .route("/emails/batch", post(batch))
        .route("/emails/{id}", get(get_one))
        .route("/emails/{id}/cancel", post(cancel))
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct BatchResponse {
    pub data: Vec<SendResponse>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct EmailList {
    pub data: Vec<EmailSummary>,
    pub next_cursor: Option<Uuid>,
}

#[derive(Deserialize, utoipa::IntoParams)]
pub struct ListQuery {
    pub status: Option<EmailStatus>,
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
}

fn body_error(r: BytesRejection) -> ApiError {
    if r.status() == StatusCode::PAYLOAD_TOO_LARGE {
        ApiError::TooLarge
    } else {
        ApiError::Validation(r.body_text())
    }
}

async fn verified_domain_id(
    state: &AppState,
    account_id: Uuid,
    domain: &str,
) -> Result<Uuid, ApiError> {
    let d = domains::find_by_name(&state.pool, account_id, domain)
        .await?
        .ok_or_else(|| {
            ApiError::Unprocessable(
                "domain_not_found",
                format!("add and verify {domain} before sending from it"),
            )
        })?;
    if d.status != DomainStatus::Verified {
        return Err(ApiError::Unprocessable(
            "domain_not_verified",
            format!("{domain} is not verified yet"),
        ));
    }
    Ok(d.id)
}

fn idempotency_key(headers: &HeaderMap) -> Result<Option<String>, ApiError> {
    let Some(v) = headers.get("idempotency-key") else {
        return Ok(None);
    };
    let s = v
        .to_str()
        .map_err(|_| ApiError::Validation("Idempotency-Key must be printable ASCII".into()))?;
    if s.is_empty() || s.len() > 256 {
        return Err(ApiError::Validation(
            "Idempotency-Key must be 1-256 characters".into(),
        ));
    }
    Ok(Some(s.to_string()))
}

fn is_idempotency_race(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(d) if d.is_unique_violation() && d.constraint() == Some("emails_idempotency"))
}

pub(crate) async fn send(
    State(state): State<AppState>,
    auth: Auth,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Result<Json<SendResponse>, ApiError> {
    auth.require_active()?;
    let body = body.map_err(body_error)?;
    let req: SendEmailRequest =
        serde_json::from_slice(&body).map_err(|e| ApiError::Validation(e.to_string()))?;
    let now = Utc::now();
    let email = emails::validate(req, now)?;
    let domain_id = verified_domain_id(&state, auth.account_id(), &email.from_domain).await?;
    let key = idempotency_key(&headers)?;
    let hash = Sha256::digest(&body).to_vec();

    let mut tx = state.pool.begin().await?;
    if let Some(key) = &key {
        let prior = sqlx::query!(
            r#"SELECT id, request_hash, created_at, priority AS "priority: Priority" FROM emails WHERE account_id = $1 AND idempotency_key = $2"#,
            auth.account_id(),
            key
        )
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(p) = prior {
            if p.created_at > now - Duration::hours(IDEMPOTENCY_HOURS) {
                return if p.request_hash.as_deref() == Some(hash.as_slice()) {
                    Ok(Json(SendResponse {
                        id: p.id,
                        priority: p.priority,
                    }))
                } else {
                    Err(ApiError::Conflict(
                        "idempotency_conflict",
                        "this Idempotency-Key was used with a different body".into(),
                    ))
                };
            }
            // Older than the window: free the key for reuse.
            sqlx::query!(
                "UPDATE emails SET idempotency_key = NULL WHERE id = $1",
                p.id
            )
            .execute(&mut *tx)
            .await?;
        }
    }
    let idem = key.as_deref().map(|k| (k, hash.as_slice()));
    let (id, priority) =
        match emails::insert(&mut tx, auth.account_id(), domain_id, &email, idem).await {
            Ok(done) => done,
            Err(e) if is_idempotency_race(&e) => {
                return Err(ApiError::Conflict(
                    "idempotency_conflict",
                    "a request with this Idempotency-Key is already being processed".into(),
                ));
            }
            Err(e) => return Err(e.into()),
        };
    tx.commit().await?;
    Ok(Json(SendResponse { id, priority }))
}

pub(crate) async fn batch(
    State(state): State<AppState>,
    auth: Auth,
    body: Result<Bytes, BytesRejection>,
) -> Result<Json<BatchResponse>, ApiError> {
    auth.require_active()?;
    let body = body.map_err(body_error)?;
    let reqs: Vec<SendEmailRequest> =
        serde_json::from_slice(&body).map_err(|e| ApiError::Validation(e.to_string()))?;
    if reqs.is_empty() || reqs.len() > MAX_BATCH {
        return Err(ApiError::Validation(format!(
            "a batch holds 1-{MAX_BATCH} emails"
        )));
    }
    let now = Utc::now();
    let mut valid: Vec<(ValidEmail, Uuid)> = Vec::with_capacity(reqs.len());
    let mut domain_ids: HashMap<String, Uuid> = HashMap::new();
    for (i, r) in reqs.into_iter().enumerate() {
        let prefix = |e: ApiError| match e {
            ApiError::Validation(m) => ApiError::Validation(format!("emails[{i}]: {m}")),
            ApiError::Unprocessable(c, m) => {
                ApiError::Unprocessable(c, format!("emails[{i}]: {m}"))
            }
            other => other,
        };
        let e = emails::validate(r, now).map_err(prefix)?;
        let id = match domain_ids.get(&e.from_domain) {
            Some(id) => *id,
            None => {
                let id = verified_domain_id(&state, auth.account_id(), &e.from_domain)
                    .await
                    .map_err(prefix)?;
                domain_ids.insert(e.from_domain.clone(), id);
                id
            }
        };
        valid.push((e, id));
    }
    let mut tx = state.pool.begin().await?;
    let mut data = Vec::with_capacity(valid.len());
    for (e, domain_id) in &valid {
        let (id, priority) =
            emails::insert(&mut tx, auth.account_id(), *domain_id, e, None).await?;
        data.push(SendResponse { id, priority });
    }
    tx.commit().await?;
    Ok(Json(BatchResponse { data }))
}

pub(crate) async fn get_one(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<Json<EmailView>, ApiError> {
    emails::view(&state.pool, auth.account_id(), id)
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

pub(crate) async fn list(
    State(state): State<AppState>,
    auth: Auth,
    q: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Json<EmailList>, ApiError> {
    auth.require_full()?;
    let Query(q) = q.map_err(|e| ApiError::Validation(e.body_text()))?;
    let limit = q.limit.unwrap_or(50).clamp(1, 100);
    let data = emails::list(&state.pool, auth.account_id(), q.status, q.after, limit).await?;
    let next_cursor = if data.len() as i64 == limit {
        data.last().map(|e| e.id)
    } else {
        None
    };
    Ok(Json(EmailList { data, next_cursor }))
}

pub(crate) async fn cancel(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<Json<SendResponse>, ApiError> {
    auth.require_full()?;
    let done = sqlx::query!(
        r#"UPDATE emails SET status = 'canceled', updated_at = now()
         WHERE id = $1 AND account_id = $2 AND status = 'queued' RETURNING id, priority AS "priority: Priority""#,
        id,
        auth.account_id()
    )
    .fetch_optional(&state.pool)
    .await?;
    match done {
        Some(r) => Ok(Json(SendResponse {
            id: r.id,
            priority: r.priority,
        })),
        None => match emails::view(&state.pool, auth.account_id(), id).await? {
            None => Err(ApiError::NotFound),
            Some(_) => Err(ApiError::Conflict(
                "not_cancelable",
                "only queued or scheduled emails can be canceled".into(),
            )),
        },
    }
}
