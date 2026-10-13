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
