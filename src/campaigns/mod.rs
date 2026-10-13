//! Newsletter issues: the campaign row and its states. Checks, sending, the worker side and the
//! report live in the submodules added by later tasks.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool, types::Json};
use uuid::Uuid;

pub const EMPTY_DOC: &str = r#"{"type":"doc","content":[]}"#;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize, utoipa::ToSchema,
)]
#[sqlx(type_name = "campaign_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CampaignStatus {
    Draft,
    Scheduled,
    Sending,
    Paused,
    Sent,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Campaign {
    pub id: Uuid,
    #[serde(skip)]
    pub account_id: Uuid,
    pub subject: String,
    pub preview_text: Option<String>,
    /// The editor document (Tiptap JSON, see `render::blocks`).
    #[schema(value_type = Object)]
    pub blocks: Json<serde_json::Value>,
    /// Tag ids: send only to contacts with any of these (empty = everyone subscribed).
    pub include_tags: Vec<Uuid>,
    /// Tag ids: skip contacts with any of these.
    pub exclude_tags: Vec<Uuid>,
    pub status: CampaignStatus,
    pub send_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub paused_reason: Option<String>,
    /// Set when the send starts: `/p/<newsletter slug>/<archive_slug>`.
    pub archive_slug: Option<String>,
    pub archive_hidden: bool,
    /// The last wave started (0 = none yet).
    pub wave: i32,
    pub next_wave_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn get(
    pool: &PgPool,
    account_id: Option<Uuid>,
    id: Uuid,
) -> sqlx::Result<Option<Campaign>> {
    sqlx::query_as!(
        Campaign,
        r#"SELECT id, account_id, subject, preview_text, blocks AS "blocks: Json<serde_json::Value>",
                  include_tags, exclude_tags, status AS "status: CampaignStatus", send_at, started_at,
                  finished_at, paused_reason, archive_slug, archive_hidden, wave, next_wave_at, created_at, updated_at
           FROM campaigns WHERE id = $1 AND ($2::uuid IS NULL OR account_id = $2)"#,
        id,
        account_id
    )
    .fetch_optional(pool)
    .await
}

/// The row, locked for this transaction; None if missing or another transaction holds it.
pub async fn lock_skip(conn: &mut PgConnection, id: Uuid) -> sqlx::Result<Option<Campaign>> {
    sqlx::query_as!(
        Campaign,
        r#"SELECT id, account_id, subject, preview_text, blocks AS "blocks: Json<serde_json::Value>",
                  include_tags, exclude_tags, status AS "status: CampaignStatus", send_at, started_at,
                  finished_at, paused_reason, archive_slug, archive_hidden, wave, next_wave_at, created_at, updated_at
           FROM campaigns WHERE id = $1 FOR UPDATE SKIP LOCKED"#,
        id
    )
    .fetch_optional(&mut *conn)
    .await
}

pub async fn create_draft(
    conn: &mut PgConnection,
    account_id: Uuid,
    created_by: Option<Uuid>,
) -> sqlx::Result<Campaign> {
    sqlx::query_as!(
        Campaign,
        r#"INSERT INTO campaigns (id, account_id, created_by) VALUES ($1, $2, $3)
           RETURNING id, account_id, subject, preview_text, blocks AS "blocks: Json<serde_json::Value>",
                     include_tags, exclude_tags, status AS "status: CampaignStatus", send_at, started_at,
                     finished_at, paused_reason, archive_slug, archive_hidden, wave, next_wave_at, created_at, updated_at"#,
        Uuid::now_v7(),
        account_id,
        created_by
    )
    .fetch_one(&mut *conn)
    .await
}

pub const MAX_SUBJECT: usize = 200;
pub const MAX_PREVIEW: usize = 200;
pub const MAX_BLOCKS_BYTES: usize = 512 * 1024;

pub fn clean_subject(raw: &str) -> Result<String, String> {
    let s = raw.trim();
    if s.contains(['\r', '\n']) {
        return Err("the subject is one line".into());
    }
    if s.chars().count() > MAX_SUBJECT {
        return Err(format!("the subject is at most {MAX_SUBJECT} characters"));
    }
    Ok(s.to_string())
}

/// `""` clears it.
pub fn clean_preview(raw: &str) -> Result<Option<String>, String> {
    let s = raw.trim();
    if s.contains(['\r', '\n']) {
        return Err("the preview text is one line".into());
    }
    if s.chars().count() > MAX_PREVIEW {
        return Err(format!(
            "the preview text is at most {MAX_PREVIEW} characters"
        ));
    }
    Ok((!s.is_empty()).then(|| s.to_string()))
}

/// Absent fields stay as they are.
#[derive(Debug, Default)]
pub struct Edit {
    pub subject: Option<String>,
    pub preview_text: Option<Option<String>>,
    pub blocks: Option<serde_json::Value>,
    pub include_tags: Option<Vec<Uuid>>,
    pub exclude_tags: Option<Vec<Uuid>>,
    pub archive_hidden: Option<bool>,
}

impl Edit {
    /// Anything but `archive_hidden`: only drafts and scheduled issues take these.
    pub fn touches_content(&self) -> bool {
        self.subject.is_some()
            || self.preview_text.is_some()
            || self.blocks.is_some()
            || self.include_tags.is_some()
            || self.exclude_tags.is_some()
    }
}

/// A content edit of a scheduled issue unschedules it: it is checked again when sent again.
pub async fn apply(conn: &mut PgConnection, id: Uuid, e: &Edit) -> sqlx::Result<()> {
    sqlx::query!(
        r#"UPDATE campaigns SET
             subject = COALESCE($2, subject),
             preview_text = CASE WHEN $3 THEN $4 ELSE preview_text END,
             blocks = COALESCE($5, blocks),
             include_tags = COALESCE($6, include_tags),
             exclude_tags = COALESCE($7, exclude_tags),
             archive_hidden = COALESCE($8, archive_hidden),
             status = CASE WHEN $9 AND status = 'scheduled' THEN 'draft'::campaign_status ELSE status END,
             send_at = CASE WHEN $9 AND status = 'scheduled' THEN NULL ELSE send_at END,
             updated_at = now()
           WHERE id = $1"#,
        id,
        e.subject,
        e.preview_text.is_some(),
        e.preview_text.clone().flatten(),
        e.blocks.clone().map(Json) as Option<Json<serde_json::Value>>,
        e.include_tags.as_deref(),
        e.exclude_tags.as_deref(),
        e.archive_hidden,
        e.touches_content()
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// True if any id is not a tag of this account.
pub async fn foreign_tags(pool: &PgPool, account_id: Uuid, ids: &[Uuid]) -> sqlx::Result<bool> {
    let mine: i64 = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM tags WHERE account_id = $1 AND id = ANY($2)"#,
        account_id,
        ids
    )
    .fetch_one(pool)
    .await?;
    Ok(mine != ids.len() as i64)
}
