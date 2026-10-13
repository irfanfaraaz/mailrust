use axum::{
    Json, Router,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::StatusCode,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{AppState, auth::Auth, json::ApiJson};
use crate::{
    campaigns::{self, Campaign, CampaignStatus, Edit},
    error::ApiError,
    render::blocks,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/campaigns", get(list).post(create))
        .route("/campaigns/{id}", get(get_one).patch(update).delete(remove))
        .route("/campaigns/{id}/preview", post(preview))
}

/// Create and edit. Absent fields stay as they are; `preview_text: ""` clears it.
#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
pub struct CampaignRequest {
    pub subject: Option<String>,
    pub preview_text: Option<String>,
    /// The editor document (Tiptap JSON). Saved when it is a document; the full rules run at send.
    #[schema(value_type = Option<Object>)]
    pub blocks: Option<serde_json::Value>,
    pub include_tags: Option<Vec<Uuid>>,
    pub exclude_tags: Option<Vec<Uuid>>,
    /// Hide a sent issue from the public archive. Works in any status.
    pub archive_hidden: Option<bool>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct CampaignSummary {
    pub id: Uuid,
    pub subject: String,
    pub status: CampaignStatus,
    pub send_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Emails queued or sent for this issue.
    pub recipients: i64,
    /// Readers who opened (unique, bots left out).
    pub opened: i64,
    /// Readers who clicked (unique, bots left out).
    pub clicked: i64,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct CampaignList {
    pub data: Vec<CampaignSummary>,
    pub next_cursor: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub after: Option<Uuid>,
}

async fn to_edit(
    state: &AppState,
    account_id: Uuid,
    body: CampaignRequest,
) -> Result<Edit, ApiError> {
    let v = ApiError::Validation;
    if let Some(b) = &body.blocks {
        if serde_json::to_vec(b).map_or(usize::MAX, |x| x.len()) > campaigns::MAX_BLOCKS_BYTES {
            return Err(ApiError::Unprocessable(
                "blocks_too_large",
                "the issue is too long to save (512 KB)".into(),
            ));
        }
        blocks::parse(b).map_err(|e| v(e.to_string()))?;
    }
    for ids in [&body.include_tags, &body.exclude_tags]
        .into_iter()
        .flatten()
    {
        if campaigns::foreign_tags(&state.pool, account_id, ids).await? {
            return Err(v("a tag in the audience does not exist".into()));
        }
    }
    let dedupe = |ids: Option<Vec<Uuid>>| {
        ids.map(|mut x| {
            x.sort();
            x.dedup();
            x
        })
    };
    Ok(Edit {
        subject: body
            .subject
            .as_deref()
            .map(campaigns::clean_subject)
            .transpose()
            .map_err(v)?,
        preview_text: body
            .preview_text
            .as_deref()
            .map(campaigns::clean_preview)
            .transpose()
            .map_err(v)?,
        blocks: body.blocks,
        include_tags: dedupe(body.include_tags),
        exclude_tags: dedupe(body.exclude_tags),
        archive_hidden: body.archive_hidden,
    })
}

async fn load(state: &AppState, auth: &Auth, id: Uuid) -> Result<Campaign, ApiError> {
    campaigns::get(&state.pool, Some(auth.account_id()), id)
        .await?
        .ok_or(ApiError::NotFound)
}

pub(crate) async fn list(
    State(state): State<AppState>,
    auth: Auth,
    q: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Json<CampaignList>, ApiError> {
    auth.require_full()?;
    let Query(q) = q.map_err(|e| ApiError::Validation(e.body_text()))?;
    let limit = q.limit.unwrap_or(50).clamp(1, 100);
    let data = sqlx::query_as!(
        CampaignSummary,
        r#"SELECT c.id, c.subject, c.status AS "status: CampaignStatus", c.send_at, c.started_at, c.finished_at,
                  c.created_at, c.updated_at,
                  (SELECT count(*) FROM emails e WHERE e.campaign_id = c.id) AS "recipients!",
                  (SELECT count(DISTINCT v.email_id) FROM email_events v JOIN emails e ON e.id = v.email_id
                   WHERE e.campaign_id = c.id AND v.type = 'opened' AND NOT v.likely_bot) AS "opened!",
                  (SELECT count(DISTINCT v.email_id) FROM email_events v JOIN emails e ON e.id = v.email_id
                   WHERE e.campaign_id = c.id AND v.type = 'clicked' AND NOT v.likely_bot) AS "clicked!"
           FROM campaigns c
           WHERE c.account_id = $1 AND ($2::uuid IS NULL OR c.id < $2)
           ORDER BY c.id DESC LIMIT $3"#,
        auth.account_id(),
        q.after,
        limit
    )
    .fetch_all(&state.pool)
    .await?;
    let next_cursor = if data.len() as i64 == limit {
        data.last().map(|c| c.id)
    } else {
        None
    };
    Ok(Json(CampaignList { data, next_cursor }))
}

pub(crate) async fn create(
    State(state): State<AppState>,
    auth: Auth,
    ApiJson(body): ApiJson<CampaignRequest>,
) -> Result<(StatusCode, Json<Campaign>), ApiError> {
    auth.require_full()?;
    let edit = to_edit(&state, auth.account_id(), body).await?;
    let mut tx = state.pool.begin().await?;
    let c =
        campaigns::create_draft(&mut tx, auth.account_id(), auth.member.map(|m| m.user_id)).await?;
    campaigns::apply(&mut tx, c.id, &edit).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(load(&state, &auth, c.id).await?)))
}

pub(crate) async fn get_one(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<Json<Campaign>, ApiError> {
    auth.require_full()?;
    Ok(Json(load(&state, &auth, id).await?))
}

pub(crate) async fn update(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
    ApiJson(body): ApiJson<CampaignRequest>,
) -> Result<Json<Campaign>, ApiError> {
    auth.require_full()?;
    let edit = to_edit(&state, auth.account_id(), body).await?;
    let mut tx = state.pool.begin().await?;
    let status = sqlx::query_scalar!(
        r#"SELECT status AS "status: CampaignStatus" FROM campaigns WHERE id = $1 AND account_id = $2 FOR UPDATE"#,
        id,
        auth.account_id()
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound)?;
    if edit.touches_content()
        && !matches!(status, CampaignStatus::Draft | CampaignStatus::Scheduled)
    {
        return Err(ApiError::Conflict(
            "campaign_locked",
            "only a draft or a scheduled issue can be edited".into(),
        ));
    }
    campaigns::apply(&mut tx, id, &edit).await?;
    tx.commit().await?;
    Ok(Json(load(&state, &auth, id).await?))
}

pub(crate) async fn remove(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    auth.require_full()?;
    let c = load(&state, &auth, id).await?;
    if c.status != CampaignStatus::Draft {
        return Err(ApiError::Conflict(
            "campaign_not_draft",
            "only a draft can be deleted; cancel it instead".into(),
        ));
    }
    sqlx::query!(
        "DELETE FROM campaigns WHERE id = $1 AND status = 'draft'",
        id
    )
    .execute(&state.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct CampaignPreview {
    /// The email as a reader without merge values sees it (fallbacks), footer included.
    pub html: String,
    pub text: String,
    /// The largest copy any reader can get, in bytes (None until the body is a valid document).
    pub size_bytes: Option<usize>,
    pub checks: Vec<crate::campaigns::checks::Check>,
    pub warnings: Vec<String>,
    pub readers: i64,
    pub missing_first_name: i64,
    /// What readers without a first name see, when the issue greets by name.
    pub first_name_fallback: Option<String>,
    /// Readers per wave.
    pub waves: Vec<usize>,
}

pub(crate) async fn preview(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<Json<CampaignPreview>, ApiError> {
    auth.require_full()?;
    let c = load(&state, &auth, id).await?;
    let r = campaigns::checks::run(&state, &c).await?;
    let (html, text, fallback, size_bytes) = match &r.doc {
        Some(doc) => {
            let issue = crate::render::campaign::Issue {
                subject: &c.subject,
                preview_text: c.preview_text.as_deref(),
                newsletter_name: &r.newsletter_name,
                postal_address: &r.postal_address,
                doc,
            };
            let p = crate::render::campaign::email(
                &issue,
                &Default::default(),
                &state.config.web_base(),
                Some("#"),
            );
            let size = crate::render::campaign::email_size(
                &issue,
                &state.config.web_base(),
                &state.config.tracking_secret,
            )
            .ok();
            (p.html, p.text, blocks::first_name_fallback(doc), size)
        }
        None => (String::new(), String::new(), None, None),
    };
    Ok(Json(CampaignPreview {
        html,
        text,
        size_bytes,
        waves: campaigns::plan::wave_sizes(r.audience.readers as usize),
        readers: r.audience.readers,
        missing_first_name: r.audience.missing_first_name,
        first_name_fallback: fallback,
        checks: r.checks,
        warnings: r.warnings,
    }))
}
