use std::time::Duration as StdDuration;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;

use crate::{api::AppState, background, sender::SendQuota};

pub const SYNC_SECS: u64 = 300;
/// After SES says the 24-hour quota is used up, nobody sends for this long (or until a sync shows room).
pub const DAILY_CAP_PAUSE_SECS: i64 = 900;
/// Non-urgent mail stops at this share of the 24-hour quota; the rest is kept for urgent mail.
pub const RESERVE_AT: f64 = 0.9;

#[derive(Debug, Clone, Default)]
pub struct QuotaRow {
    pub max_send_rate: Option<f64>,
    pub max_24h_send: Option<f64>,
    pub sent_last_24h: Option<f64>,
    pub synced_at: Option<DateTime<Utc>>,
    pub paused_until: Option<DateTime<Utc>>,
    pub pause_reason: Option<String>,
}

pub async fn load(pool: &PgPool) -> sqlx::Result<Option<QuotaRow>> {
    sqlx::query_as!(
        QuotaRow,
        "SELECT max_send_rate, max_24h_send, sent_last_24h, synced_at, paused_until, pause_reason FROM sender_quota"
    )
    .fetch_optional(pool)
    .await
}

/// Saves fresh numbers. If they show room, a daily-cap pause ends early.
pub async fn save(pool: &PgPool, q: &SendQuota) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO sender_quota (id, max_send_rate, max_24h_send, sent_last_24h, synced_at)
         VALUES (true, $1, $2, $3, now())
         ON CONFLICT (id) DO UPDATE SET
             max_send_rate = EXCLUDED.max_send_rate, max_24h_send = EXCLUDED.max_24h_send,
             sent_last_24h = EXCLUDED.sent_last_24h, synced_at = EXCLUDED.synced_at,
             paused_until = CASE WHEN EXCLUDED.sent_last_24h < EXCLUDED.max_24h_send THEN NULL ELSE sender_quota.paused_until END,
             pause_reason = CASE WHEN EXCLUDED.sent_last_24h < EXCLUDED.max_24h_send THEN NULL ELSE sender_quota.pause_reason END",
        q.max_send_rate,
        q.max_24h_send,
        q.sent_last_24h
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn sync_once(state: &AppState) -> anyhow::Result<()> {
    if let Some(q) = state.sender.quota().await? {
        save(&state.pool, &q).await?;
    }
    Ok(())
}

pub async fn sync_loop(state: AppState, shutdown: CancellationToken) -> anyhow::Result<()> {
    background::every(StdDuration::from_secs(SYNC_SECS), &shutdown, || async {
        if let Err(e) = sync_once(&state).await {
            tracing::warn!(error = ?e, "quota sync failed; keeping the last numbers");
        }
    })
    .await;
    Ok(())
}
