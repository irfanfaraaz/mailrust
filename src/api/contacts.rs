use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sqlx::types::Json as SqlJson;
use uuid::Uuid;

use super::{AppState, auth::Auth, json::ApiJson};
use crate::{
    contacts::{self, ContactInput, ContactSource, ContactStatus, ContactView, Fields},
    error::ApiError,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/contacts", get(list).post(create))
        .route("/contacts/export", get(export))
        .route("/contacts/{id}", get(get_one).patch(update).delete(remove))
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ContactList {
    pub data: Vec<ContactView>,
    pub next_cursor: Option<Uuid>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateContactRequest {
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    #[serde(default)]
    pub fields: Fields,
    /// Tag names; missing tags are created.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Absent fields stay as they are. An empty name clears it. `fields` and `tags` replace the whole set.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateContactRequest {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub fields: Option<Fields>,
    pub tags: Option<Vec<String>>,
    /// `subscribed` or `unsubscribed`. An unsubscribed contact cannot be set back to `subscribed`.
    pub status: Option<ContactStatus>,
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub status: Option<ContactStatus>,
    pub tag: Option<Uuid>,
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
}

pub(crate) async fn list(
    State(state): State<AppState>,
    auth: Auth,
    q: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Json<ContactList>, ApiError> {
    auth.require_full()?;
    let Query(q) = q.map_err(|e| ApiError::Validation(e.body_text()))?;
    let limit = q.limit.unwrap_or(50).clamp(1, 100);
    let f = contacts::ListFilter { status: q.status, tag: q.tag, q: q.q };
    let data = contacts::list(&state.pool, auth.account_id(), &f, q.after, limit).await?;
    let next_cursor = if data.len() as i64 == limit { data.last().map(|c| c.id) } else { None };
    Ok(Json(ContactList { data, next_cursor }))
}

pub(crate) async fn create(
    State(state): State<AppState>,
    auth: Auth,
    ApiJson(body): ApiJson<CreateContactRequest>,
) -> Result<(StatusCode, Json<ContactView>), ApiError> {
    auth.require_full()?;
    let input = ContactInput {
        email: body.email,
        first_name: body.first_name,
        last_name: body.last_name,
        fields: body.fields,
        tags: body.tags,
    }
    .validate()
    .map_err(ApiError::Validation)?;
    let source = if auth.member.is_some() { ContactSource::Dashboard } else { ContactSource::Api };
    let mut tx = state.pool.begin().await?;
    let (id, new) = contacts::save(&mut tx, auth.account_id(), &input, source, None).await?;
    tx.commit().await?;
    let view = contacts::view(&state.pool, auth.account_id(), id).await?.ok_or(ApiError::NotFound)?;
    Ok((if new { StatusCode::CREATED } else { StatusCode::OK }, Json(view)))
}

pub(crate) async fn get_one(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<Json<ContactView>, ApiError> {
    auth.require_full()?;
    contacts::view(&state.pool, auth.account_id(), id).await?.map(Json).ok_or(ApiError::NotFound)
}

pub(crate) async fn update(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
    ApiJson(body): ApiJson<UpdateContactRequest>,
) -> Result<Json<ContactView>, ApiError> {
    auth.require_full()?;
    let account_id = auth.account_id();
    if body.status == Some(ContactStatus::Pending) {
        return Err(ApiError::Validation("status can be set to subscribed or unsubscribed only".into()));
    }
    let v = ApiError::Validation;
    let first = body.first_name.as_deref().map(|n| contacts::clean_name(Some(n))).transpose().map_err(v)?;
    let last = body.last_name.as_deref().map(|n| contacts::clean_name(Some(n))).transpose().map_err(v)?;
    let fields = body.fields.map(contacts::clean_fields).transpose().map_err(v)?;
    let tags = body.tags.as_deref().map(contacts::clean_tags).transpose().map_err(v)?;
    let (set_first, set_last) = (first.is_some(), last.is_some());

    let mut tx = state.pool.begin().await?;
    let current = sqlx::query_scalar!(
        r#"SELECT status AS "status: ContactStatus" FROM contacts WHERE id = $1 AND account_id = $2 FOR UPDATE"#,
        id,
        account_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound)?;
    if body.status == Some(ContactStatus::Subscribed) && current == ContactStatus::Unsubscribed {
        return Err(ApiError::Conflict(
            "resubscribe_not_allowed",
            "this contact unsubscribed; only they can subscribe again, through the signup form or their unsubscribe link".into(),
        ));
    }
    sqlx::query!(
        r#"UPDATE contacts SET
             first_name = CASE WHEN $3 THEN $4 ELSE first_name END,
             last_name = CASE WHEN $5 THEN $6 ELSE last_name END,
             fields = COALESCE($7, fields),
             status = COALESCE($8, status),
             subscribed_at = CASE WHEN $8 = 'subscribed' AND status <> 'subscribed' THEN now() ELSE subscribed_at END,
             unsubscribed_at = CASE WHEN $8 = 'unsubscribed' AND status <> 'unsubscribed' THEN now() ELSE unsubscribed_at END,
             unsubscribed_via_link = CASE WHEN $8 = 'unsubscribed' AND status <> 'unsubscribed' THEN false ELSE unsubscribed_via_link END,
             updated_at = now()
           WHERE id = $1 AND account_id = $2"#,
        id,
        account_id,
        set_first,
        first.flatten(),
        set_last,
        last.flatten(),
        fields.map(SqlJson) as Option<SqlJson<Fields>>,
        body.status as Option<ContactStatus>
    )
    .execute(&mut *tx)
    .await?;
    if let Some(names) = tags {
        let ids = contacts::ensure_tags(&mut tx, account_id, &names).await?;
        contacts::set_tags(&mut tx, id, &ids).await?;
    }
    tx.commit().await?;
    contacts::view(&state.pool, account_id, id).await?.map(Json).ok_or(ApiError::NotFound)
}

/// Hard delete (GDPR erase): tag links and confirmations go with it.
pub(crate) async fn remove(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    auth.require_full()?;
    let r = sqlx::query!("DELETE FROM contacts WHERE id = $1 AND account_id = $2", id, auth.account_id())
        .execute(&state.pool)
        .await?;
    if r.rows_affected() == 1 { Ok(StatusCode::NO_CONTENT) } else { Err(ApiError::NotFound) }
}

pub const EXPORT_PAGE: i64 = 1000;

/// Streams every contact as CSV, 1,000 rows per query, so a 100k list never sits in memory.
pub(crate) async fn export(State(state): State<AppState>, auth: Auth) -> Result<Response, ApiError> {
    auth.require_full()?;
    let account_id = auth.account_id();
    let keys = contacts::field_keys(&state.pool, account_id).await?;
    let first = contacts::export_csv(&keys, &[], true).map_err(anyhow::Error::from)?;
    let pool = state.pool.clone();
    let pages = futures::stream::unfold(Some((None::<Uuid>, keys)), move |cursor| {
        let pool = pool.clone();
        async move {
            let (after, keys) = cursor?;
            let page = match contacts::export_page(&pool, account_id, after, EXPORT_PAGE).await {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!(error = ?e, %account_id, "contact export failed");
                    return Some((Err(std::io::Error::other("export failed")), None));
                }
            };
            if page.is_empty() {
                return None;
            }
            let next = (page.len() as i64 == EXPORT_PAGE).then(|| (page.last().map(|r| r.id), keys.clone()));
            Some((contacts::export_csv(&keys, &page, false).map_err(std::io::Error::other), next))
        }
    });
    let body = Body::from_stream(futures::stream::once(async { Ok::<_, std::io::Error>(first) }).chain(pages));
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"contacts.csv\""),
        ],
        body,
    )
        .into_response())
}
