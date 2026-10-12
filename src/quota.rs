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
/// Numbers older than this (3 missed syncs) are not trusted for the reserve: fail open, the real SES cap still pauses us.
pub const STALE_SECS: i64 = 900;

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

/// Saves fresh numbers. If they show room below the reserve line, a daily-cap pause ends early.
/// (Not just below the cap: SES's count lags, so a near-full answer would end the pause and hit the cap again.)
/// A max of 0 or less means SES reports no limit.
pub async fn save(pool: &PgPool, q: &SendQuota) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO sender_quota (id, max_send_rate, max_24h_send, sent_last_24h, synced_at)
         VALUES (true, $1, $2, $3, now())
         ON CONFLICT (id) DO UPDATE SET
             max_send_rate = EXCLUDED.max_send_rate, max_24h_send = EXCLUDED.max_24h_send,
             sent_last_24h = EXCLUDED.sent_last_24h, synced_at = EXCLUDED.synced_at,
             paused_until = CASE WHEN EXCLUDED.max_24h_send <= 0 OR EXCLUDED.sent_last_24h < $4 * EXCLUDED.max_24h_send
                                 THEN NULL ELSE sender_quota.paused_until END,
             pause_reason = CASE WHEN EXCLUDED.max_24h_send <= 0 OR EXCLUDED.sent_last_24h < $4 * EXCLUDED.max_24h_send
                                 THEN NULL ELSE sender_quota.pause_reason END",
        q.max_send_rate,
        q.max_24h_send,
        q.sent_last_24h,
        RESERVE_AT
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// SES said the 24-hour quota is used up: nobody sends until the pause ends or a sync shows room.
pub async fn pause(pool: &PgPool, reason: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO sender_quota (id, paused_until, pause_reason) VALUES (true, now() + make_interval(secs => $1), $2)
         ON CONFLICT (id) DO UPDATE SET paused_until = EXCLUDED.paused_until, pause_reason = EXCLUDED.pause_reason",
        DAILY_CAP_PAUSE_SECS as f64,
        reason
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

/// True while the daily-cap pause holds.
pub async fn paused(pool: &PgPool) -> sqlx::Result<bool> {
    sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM sender_quota WHERE paused_until > now()) AS "paused!""#
    )
    .fetch_one(pool)
    .await
}

/// What `queue::claim` may take this round, and the SES rate to pace it by (one read per round).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Gate {
    pub paused: bool,
    pub only_urgent: bool,
    pub max_send_rate: Option<f64>,
}

/// Before the first sync there is no row: no pause, no reserve.
/// The estimate adds recipients sent since the sync, because SES's count is up to 5 minutes old.
pub async fn gate(pool: &PgPool) -> sqlx::Result<Gate> {
    let row = sqlx::query!(
        r#"SELECT q.max_send_rate, COALESCE(q.paused_until > now(), false) AS "paused!",
                  COALESCE(q.max_24h_send > 0 AND q.synced_at > now() - make_interval(secs => $2) AND
                      q.sent_last_24h + (SELECT COALESCE(sum(cardinality(e.to_addresses) + cardinality(e.cc_addresses)
                                                             + cardinality(e.bcc_addresses)), 0)
                                         FROM emails e WHERE e.sent_at > q.synced_at)::float8
                      >= $1 * q.max_24h_send, false) AS "only_urgent!"
           FROM sender_quota q"#,
        RESERVE_AT,
        STALE_SECS as f64
    )
    .fetch_optional(pool)
    .await?;
    Ok(row
        .map(|r| Gate {
            paused: r.paused,
            only_urgent: r.only_urgent,
            max_send_rate: r.max_send_rate,
        })
        .unwrap_or_default())
}
