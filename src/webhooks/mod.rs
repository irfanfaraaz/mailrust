use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::json;
use sqlx::{PgConnection, PgPool};
use tokio_util::sync::CancellationToken;
use url::Host;
use uuid::Uuid;

use crate::{api::AppState, background};

pub mod sign;
pub mod ssrf;

pub const EVENT_TYPES: [&str; 10] = [
    "email.sent",
    "email.delivered",
    "email.delivery_delayed",
    "email.bounced",
    "email.complained",
    "email.opened",
    "email.clicked",
    "email.failed",
    "email.suppressed",
    "account.paused",
];
pub const RETRY_SECS: [i64; 7] = [5, 300, 1800, 7200, 18000, 36000, 50400];
pub const PER_TICK: i64 = 20;
pub const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WebhookView {
    pub id: Uuid,
    pub url: String,
    pub events: Vec<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    /// Only returned once, when the webhook is created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
}

pub async fn create(
    pool: &PgPool,
    account_id: Uuid,
    url: &str,
    events: &[String],
) -> sqlx::Result<WebhookView> {
    let id = Uuid::now_v7();
    let secret = sign::new_secret();
    let created_at = sqlx::query_scalar!(
        "INSERT INTO webhooks (id, account_id, url, secret, event_types) VALUES ($1, $2, $3, $4, $5) RETURNING created_at",
        id,
        account_id,
        url,
        secret,
        events
    )
    .fetch_one(pool)
    .await?;
    Ok(WebhookView {
        id,
        url: url.into(),
        events: events.to_vec(),
        enabled: true,
        created_at,
        secret: Some(secret),
    })
}

pub async fn list(pool: &PgPool, account_id: Uuid) -> sqlx::Result<Vec<WebhookView>> {
    let rows = sqlx::query!(
        "SELECT id, url, event_types, enabled, created_at FROM webhooks WHERE account_id = $1 ORDER BY created_at",
        account_id
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| WebhookView {
            id: r.id,
            url: r.url,
            events: r.event_types,
            enabled: r.enabled,
            created_at: r.created_at,
            secret: None,
        })
        .collect())
}

pub async fn delete(pool: &PgPool, account_id: Uuid, id: Uuid) -> sqlx::Result<bool> {
    Ok(sqlx::query!(
        "DELETE FROM webhooks WHERE id = $1 AND account_id = $2",
        id,
        account_id
    )
    .execute(pool)
    .await?
    .rows_affected()
        == 1)
}

/// Queues one delivery per enabled webhook of the account that subscribed to `event_type`.
pub async fn enqueue(
    conn: &mut PgConnection,
    account_id: Uuid,
    event_type: &str,
    data: serde_json::Value,
) -> sqlx::Result<()> {
    let payload = json!({ "type": event_type, "created_at": Utc::now(), "data": data });
    sqlx::query!(
        "INSERT INTO webhook_deliveries (id, account_id, webhook_id, event_type, payload)
         SELECT gen_random_uuid(), $1, w.id, $2, $3 FROM webhooks w
         WHERE w.account_id = $1 AND w.enabled AND $2 = ANY(w.event_types)",
        account_id,
        event_type,
        payload
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

struct Due {
    id: Uuid,
    payload: serde_json::Value,
    attempts: i32,
    url: String,
    secret: String,
    enabled: bool,
}

async fn claim_due(pool: &PgPool, limit: i64) -> sqlx::Result<Vec<Due>> {
    sqlx::query_as!(
        Due,
        r#"WITH due AS (
               SELECT id FROM webhook_deliveries
               WHERE status = 'pending' AND next_attempt_at <= now() AND (lease_until IS NULL OR lease_until < now())
               ORDER BY next_attempt_at LIMIT $1
               FOR UPDATE SKIP LOCKED),
           upd AS (
               UPDATE webhook_deliveries d SET lease_until = now() + interval '2 minutes', attempts = d.attempts + 1
               FROM due WHERE d.id = due.id
               RETURNING d.id, d.webhook_id, d.payload, d.attempts)
           SELECT upd.id AS "id!", upd.payload AS "payload!", upd.attempts AS "attempts!",
                  w.url AS "url!", w.secret AS "secret!", w.enabled AS "enabled!"
           FROM upd JOIN webhooks w ON w.id = upd.webhook_id"#,
        limit
    )
    .fetch_all(pool)
    .await
}

/// One HTTP attempt. Ok(status code) if the server answered at all.
async fn post(d: &Due, allow_private: bool) -> Result<u16, String> {
    // Checked at every delivery, not only at creation: DNS can change after the webhook was saved.
    let (url, addrs) = ssrf::check_url(&d.url, allow_private).await?;
    let mut builder = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .redirect(reqwest::redirect::Policy::none());
    if let Some(Host::Domain(host)) = url.host() {
        builder = builder.resolve_to_addrs(host, &addrs); // pin to the addresses we just checked
    }
    let client = builder.build().map_err(|e| e.to_string())?;
    let body = serde_json::to_vec(&d.payload).map_err(|e| e.to_string())?;
    let msg_id = format!("msg_{}", d.id.simple());
    let ts = Utc::now().timestamp();
    let sig = sign::signature(&d.secret, &msg_id, ts, &body)?;
    let res = client
        .post(url)
        .header("content-type", "application/json")
        .header("user-agent", "Mailrust-Webhooks/1")
        .header("webhook-id", &msg_id)
        .header("webhook-timestamp", ts.to_string())
        .header("webhook-signature", sig)
        .body(body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(res.status().as_u16())
}

async fn finish(pool: &PgPool, d: &Due, outcome: Result<u16, String>) -> sqlx::Result<()> {
    let (code, error) = match &outcome {
        Ok(c) if (200..300).contains(c) => {
            sqlx::query!(
                "UPDATE webhook_deliveries SET status = 'delivered', last_response_code = $2, last_error = NULL, lease_until = NULL WHERE id = $1",
                d.id,
                *c as i32
            )
            .execute(pool)
            .await?;
            return Ok(());
        }
        Ok(c) => (Some(*c as i32), format!("server answered {c}")),
        Err(e) => (None, e.clone()),
    };
    match RETRY_SECS
        .get((d.attempts - 1).max(0) as usize)
        .filter(|_| d.enabled)
    {
        Some(secs) => {
            sqlx::query!(
                "UPDATE webhook_deliveries SET next_attempt_at = now() + make_interval(secs => $2), lease_until = NULL,
                        last_response_code = $3, last_error = $4
                 WHERE id = $1",
                d.id,
                *secs as f64,
                code,
                error
            )
            .execute(pool)
            .await?;
        }
        None => {
            sqlx::query!(
                "UPDATE webhook_deliveries SET status = 'failed', lease_until = NULL, last_response_code = $2, last_error = $3 WHERE id = $1",
                d.id,
                code,
                error
            )
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

/// Delivers one batch of due webhooks. Returns how many it tried.
pub async fn deliver_due(state: &AppState) -> anyhow::Result<usize> {
    let due = claim_due(&state.pool, PER_TICK).await?;
    for d in &due {
        let outcome = if d.enabled {
            post(d, state.config.allow_private_webhooks).await
        } else {
            Err("webhook is disabled".into())
        };
        finish(&state.pool, d, outcome).await?;
    }
    Ok(due.len())
}

pub async fn delivery_loop(state: AppState, shutdown: CancellationToken) -> anyhow::Result<()> {
    background::every(Duration::from_secs(1), &shutdown, || async {
        if let Err(e) = deliver_due(&state).await {
            tracing::error!(error = ?e, "webhook delivery failed");
        }
    })
    .await;
    Ok(())
}
