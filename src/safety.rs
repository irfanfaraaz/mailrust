use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::{accounts, render::bare_address};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize, utoipa::ToSchema,
)]
#[sqlx(type_name = "suppression_reason", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SuppressionReason {
    Bounce,
    Complaint,
    Manual,
}

/// Which of these addresses are on the account's block list? Returns bare lowercase addresses.
pub async fn suppressed_among(
    pool: &PgPool,
    account_id: Uuid,
    addresses: &[String],
) -> sqlx::Result<HashSet<String>> {
    let bare: Vec<String> = addresses.iter().map(|a| bare_address(a)).collect();
    let rows = sqlx::query_scalar!(
        "SELECT address FROM suppressions WHERE account_id = $1 AND address = ANY($2)",
        account_id,
        &bare[..]
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

pub async fn add_suppression(
    conn: &mut PgConnection,
    account_id: Uuid,
    address: &str,
    reason: SuppressionReason,
    source_email_id: Option<Uuid>,
) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO suppressions (account_id, address, reason, source_email_id) VALUES ($1, $2, $3, $4)
         ON CONFLICT (account_id, address) DO NOTHING",
        account_id,
        bare_address(address),
        reason as SuppressionReason,
        source_email_id
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub const BOUNCE_LIMIT: f64 = 0.02;
pub const BOUNCE_MIN_SENT: i64 = 100;
pub const COMPLAINT_LIMIT: f64 = 0.0008;
// Below 1,250 sent, a single complaint is already over 0.08%; one unhappy reader should not pause an account.
pub const COMPLAINT_MIN_SENT: i64 = 1250;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rates {
    pub sent: i64,
    pub bounced: i64,
    pub complained: i64,
}

pub fn verdict(r: &Rates) -> Option<String> {
    if r.sent <= 0 {
        return None;
    }
    let sent = r.sent as f64;
    if r.sent >= BOUNCE_MIN_SENT && r.bounced as f64 / sent > BOUNCE_LIMIT {
        return Some(format!(
            "bounce rate {:.2}% is over 2% in the last 7 days ({} of {} recipients)",
            r.bounced as f64 * 100.0 / sent,
            r.bounced,
            r.sent
        ));
    }
    if r.sent >= COMPLAINT_MIN_SENT && r.complained as f64 / sent > COMPLAINT_LIMIT {
        return Some(format!(
            "complaint rate {:.3}% is over 0.08% in the last 7 days ({} of {} recipients)",
            r.complained as f64 * 100.0 / sent,
            r.complained,
            r.sent
        ));
    }
    None
}

/// Recipients sent to, hard bounces and complaints in the last 7 days.
pub async fn rates(pool: &PgPool, account_id: Uuid) -> sqlx::Result<Rates> {
    let r = sqlx::query!(
        r#"SELECT
             COALESCE(SUM(CASE WHEN type = 'sent' THEN jsonb_array_length(data->'recipients') ELSE 0 END), 0)::bigint AS "sent!",
             COUNT(*) FILTER (WHERE type = 'bounced') AS "bounced!",
             COUNT(*) FILTER (WHERE type = 'complained') AS "complained!"
           FROM email_events
           WHERE account_id = $1 AND created_at > now() - interval '7 days'
             AND type IN ('sent', 'bounced', 'complained')"#,
        account_id
    )
    .fetch_one(pool)
    .await?;
    Ok(Rates {
        sent: r.sent,
        bounced: r.bounced,
        complained: r.complained,
    })
}

/// Pauses the account if its rates are too high. Returns the reason if it was paused just now.
pub async fn check_account(pool: &PgPool, account_id: Uuid) -> anyhow::Result<Option<String>> {
    let r = rates(pool, account_id).await?;
    let Some(reason) = verdict(&r) else {
        return Ok(None);
    };
    if accounts::pause(pool, account_id, &reason).await? {
        tracing::warn!(%account_id, %reason, "account auto-paused");
        let mut conn = pool.acquire().await?;
        crate::webhooks::enqueue(
            &mut conn,
            account_id,
            "account.paused",
            serde_json::json!({ "account_id": account_id, "reason": reason }),
        )
        .await?;
        return Ok(Some(reason));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(sent: i64, bounced: i64, complained: i64) -> Rates {
        Rates {
            sent,
            bounced,
            complained,
        }
    }

    #[test]
    fn thresholds() {
        assert!(
            verdict(&r(99, 50, 0)).is_none(),
            "too few sent to judge bounces"
        );
        assert!(verdict(&r(100, 2, 0)).is_none(), "exactly 2% is allowed");
        assert!(verdict(&r(100, 3, 0)).is_some());
        assert!(
            verdict(&r(1249, 0, 5)).is_none(),
            "too few sent to judge complaints"
        );
        assert!(
            verdict(&r(1250, 0, 1)).is_none(),
            "exactly 0.08% is allowed"
        );
        assert!(verdict(&r(1250, 0, 2)).is_some());
        assert!(verdict(&r(0, 0, 0)).is_none());
    }
}
