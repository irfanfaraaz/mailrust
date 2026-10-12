use std::{collections::BTreeMap, time::Duration as StdDuration};

use chrono::Duration;
use serde_json::json;
use sqlx::{PgPool, types::Json};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{api::AppState, background, email_events::{self, EventKind, NewEvent}};

pub const BATCH: i64 = 50;
pub const LEASE_SECS: i64 = 300;
pub const UNKNOWN_GRACE_SECS: i64 = 120;
pub const BACKOFF_SECS: [i64; 5] = [30, 120, 600, 3600, 21600];
pub const MAX_ATTEMPTS: i32 = 6; // 1 try + 5 retries

#[derive(Debug, Clone)]
pub struct ClaimedEmail {
    pub id: Uuid,
    pub account_id: Uuid,
    pub domain_id: Option<Uuid>,
    pub from_address: String,
    pub to_addresses: Vec<String>,
    pub cc_addresses: Vec<String>,
    pub bcc_addresses: Vec<String>,
    pub reply_to: Vec<String>,
    pub subject: String,
    pub html: Option<String>,
    pub text_body: Option<String>,
    pub headers: Json<BTreeMap<String, String>>,
    pub tags: Json<BTreeMap<String, String>>,
    pub attempts: i32,
}

/// Takes up to `limit` due emails. SKIP LOCKED: two workers never get the same row and never wait on each other.
pub async fn claim(pool: &PgPool, limit: i64) -> sqlx::Result<Vec<ClaimedEmail>> {
    sqlx::query_as!(
        ClaimedEmail,
        r#"UPDATE emails
           SET status = 'sending', lease_until = now() + make_interval(secs => $2), attempts = attempts + 1, updated_at = now()
           WHERE id IN (
               SELECT e.id FROM emails e JOIN accounts a ON a.id = e.account_id
               WHERE e.status = 'queued' AND e.send_at <= now() AND a.status = 'active'
               ORDER BY e.send_at
               LIMIT $1
               FOR UPDATE OF e SKIP LOCKED)
           RETURNING id, account_id, domain_id, from_address, to_addresses, cc_addresses, bcc_addresses, reply_to,
                     subject, html, text_body,
                     headers AS "headers: Json<BTreeMap<String, String>>",
                     tags AS "tags: Json<BTreeMap<String, String>>",
                     attempts"#,
        limit,
        LEASE_SECS as f64
    )
    .fetch_all(pool)
    .await
}

pub fn retry_delay(attempts: i32) -> Option<Duration> {
    if attempts < 1 || attempts >= MAX_ATTEMPTS {
        return None;
    }
    Some(Duration::seconds(BACKOFF_SECS[(attempts - 1) as usize]))
}

pub async fn save_links(pool: &PgPool, id: Uuid, links: &[String]) -> sqlx::Result<()> {
    sqlx::query!("UPDATE emails SET links = $2 WHERE id = $1", id, Json(links) as _)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn mark_sent(pool: &PgPool, email: &ClaimedEmail, provider_id: &str, recipients: &[String]) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    // An SES event may have arrived first (e.g. delivered). Keep that status, but always save the provider id.
    sqlx::query!(
        "UPDATE emails SET provider_message_id = $2, sent_at = COALESCE(sent_at, now()), lease_until = NULL,
                unknown_since = NULL, last_error = NULL, updated_at = now(),
                status = CASE WHEN status IN ('sending', 'unknown') THEN 'sent'::email_status ELSE status END
         WHERE id = $1",
        email.id,
        provider_id
    )
    .execute(&mut *tx)
    .await?;
    email_events::record(&mut tx, NewEvent {
        account_id: email.account_id,
        email_id: email.id,
        kind: EventKind::Sent,
        recipient: None,
        data: json!({ "recipients": recipients, "provider_message_id": provider_id }),
        likely_bot: false,
    })
    .await?;
    tx.commit().await
}

pub async fn mark_retry(pool: &PgPool, id: Uuid, error: &str, delay: Duration) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE emails SET status = 'queued', send_at = now() + make_interval(secs => $3), lease_until = NULL,
                last_error = $2, updated_at = now()
         WHERE id = $1 AND status = 'sending'",
        id,
        error,
        delay.num_seconds() as f64
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_failed(pool: &PgPool, email: &ClaimedEmail, error: &str) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query!(
        "UPDATE emails SET status = 'failed', last_error = $2, lease_until = NULL, updated_at = now()
         WHERE id = $1 AND status IN ('sending', 'queued')",
        email.id,
        error
    )
    .execute(&mut *tx)
    .await?;
    email_events::record(&mut tx, NewEvent {
        account_id: email.account_id,
        email_id: email.id,
        kind: EventKind::Failed,
        recipient: None,
        data: json!({ "error": error }),
        likely_bot: false,
    })
    .await?;
    tx.commit().await
}

pub async fn mark_suppressed(pool: &PgPool, email: &ClaimedEmail, removed: &[String]) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query!(
        "UPDATE emails SET status = 'suppressed', lease_until = NULL, updated_at = now() WHERE id = $1 AND status = 'sending'",
        email.id
    )
    .execute(&mut *tx)
    .await?;
    email_events::record(&mut tx, NewEvent {
        account_id: email.account_id,
        email_id: email.id,
        kind: EventKind::Suppressed,
        recipient: None,
        data: json!({ "recipients": removed }),
        likely_bot: false,
    })
    .await?;
    tx.commit().await
}

/// Back in line without using up a try (the account was paused, not the email's fault).
pub async fn requeue_paused(pool: &PgPool, id: Uuid) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE emails SET status = 'queued', attempts = GREATEST(attempts - 1, 0), lease_until = NULL, updated_at = now()
         WHERE id = $1 AND status = 'sending'",
        id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// A worker died while holding these. We do not know if SES got them, so: unknown.
pub async fn expire_leases(pool: &PgPool) -> sqlx::Result<u64> {
    Ok(sqlx::query!(
        "UPDATE emails SET status = 'unknown', unknown_since = now(), lease_until = NULL, updated_at = now()
         WHERE status = 'sending' AND lease_until < now()"
    )
    .execute(pool)
    .await?
    .rows_affected())
}

/// No SES "Send" event arrived in the grace period, so SES never got it: send again.
pub async fn requeue_unknown(pool: &PgPool) -> sqlx::Result<u64> {
    Ok(sqlx::query!(
        "UPDATE emails SET status = 'queued', send_at = now(), unknown_since = NULL, updated_at = now()
         WHERE status = 'unknown' AND unknown_since < now() - make_interval(secs => $1)",
        UNKNOWN_GRACE_SECS as f64
    )
    .execute(pool)
    .await?
    .rows_affected())
}

pub async fn reaper_loop(state: AppState, shutdown: CancellationToken) -> anyhow::Result<()> {
    background::every(StdDuration::from_secs(30), &shutdown, || async {
        match expire_leases(&state.pool).await {
            Ok(n) if n > 0 => tracing::warn!(count = n, "leases expired, emails marked unknown"),
            Ok(_) => {}
            Err(e) => tracing::error!(error = ?e, "expire_leases failed"),
        }
        match requeue_unknown(&state.pool).await {
            Ok(n) if n > 0 => tracing::warn!(count = n, "unknown emails requeued"),
            Ok(_) => {}
            Err(e) => tracing::error!(error = ?e, "requeue_unknown failed"),
        }
    })
    .await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_then_give_up() {
        let secs: Vec<i64> = (1..=5).map(|a| retry_delay(a).unwrap().num_seconds()).collect();
        assert_eq!(secs, vec![30, 120, 600, 3600, 21600]);
        assert!(retry_delay(6).is_none());
        assert!(retry_delay(0).is_none());
    }
}
