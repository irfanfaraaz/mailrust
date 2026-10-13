use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    accounts::{self, AccountStatus},
    safety,
};

#[derive(Debug, Default, Serialize, utoipa::ToSchema)]
pub struct Counts {
    pub sent: i64,
    pub delivered: i64,
    pub bounced: i64,
    pub complained: i64,
    pub opened: i64,
    pub clicked: i64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct Overview {
    pub last_24h: Counts,
    pub last_7d: Counts,
    /// Over the last 7 days, the same window auto-pause uses.
    pub bounce_rate: f64,
    pub complaint_rate: f64,
    pub bounce_limit: f64,
    pub complaint_limit: f64,
    pub status: AccountStatus,
    pub paused_reason: Option<String>,
}

pub async fn overview(pool: &PgPool, account_id: Uuid) -> anyhow::Result<Option<Overview>> {
    let Some(acc) = accounts::get(pool, account_id).await? else {
        return Ok(None);
    };
    let r = sqlx::query!(
        r#"SELECT
            COALESCE(SUM(jsonb_array_length(data->'recipients')) FILTER (WHERE type = 'sent' AND created_at > now() - interval '1 day'), 0)::bigint AS "s1!",
            COUNT(*) FILTER (WHERE type = 'delivered'  AND created_at > now() - interval '1 day') AS "d1!",
            COUNT(*) FILTER (WHERE type = 'bounced'    AND created_at > now() - interval '1 day') AS "b1!",
            COUNT(*) FILTER (WHERE type = 'complained' AND created_at > now() - interval '1 day') AS "c1!",
            COUNT(*) FILTER (WHERE type = 'opened'     AND created_at > now() - interval '1 day') AS "o1!",
            COUNT(*) FILTER (WHERE type = 'clicked'    AND created_at > now() - interval '1 day') AS "k1!",
            COALESCE(SUM(jsonb_array_length(data->'recipients')) FILTER (WHERE type = 'sent'), 0)::bigint AS "s7!",
            COUNT(*) FILTER (WHERE type = 'delivered')  AS "d7!",
            COUNT(*) FILTER (WHERE type = 'bounced')    AS "b7!",
            COUNT(*) FILTER (WHERE type = 'complained') AS "c7!",
            COUNT(*) FILTER (WHERE type = 'opened')     AS "o7!",
            COUNT(*) FILTER (WHERE type = 'clicked')    AS "k7!"
           FROM email_events WHERE account_id = $1 AND created_at > now() - interval '7 days'"#,
        account_id
    )
    .fetch_one(pool)
    .await?;
    let rate = |n: i64| if r.s7 > 0 { n as f64 / r.s7 as f64 } else { 0.0 };
    Ok(Some(Overview {
        last_24h: Counts { sent: r.s1, delivered: r.d1, bounced: r.b1, complained: r.c1, opened: r.o1, clicked: r.k1 },
        last_7d: Counts { sent: r.s7, delivered: r.d7, bounced: r.b7, complained: r.c7, opened: r.o7, clicked: r.k7 },
        bounce_rate: rate(r.b7),
        complaint_rate: rate(r.c7),
        bounce_limit: safety::BOUNCE_LIMIT,
        complaint_limit: safety::COMPLAINT_LIMIT,
        status: acc.status,
        paused_reason: acc.paused_reason,
    }))
}
