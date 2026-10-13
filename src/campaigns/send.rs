//! Starting a send, the wave loop, and taking queued emails back. All wave state lives in the
//! database, so an engine restart continues where it stopped. Time is always passed in as `now`.
use std::time::Duration as StdDuration;

use chrono::{DateTime, Duration, Utc};
use sqlx::PgConnection;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::{Campaign, CampaignStatus, checks, lock_skip, plan};
use crate::{api::AppState, background, emails::MAX_SCHEDULE_DAYS, error::ApiError};

pub const TICK_SECS: u64 = 15;

/// `POST /campaigns/{id}/send`: checks, then `scheduled` at `send_at` (now when absent or past);
/// a send for now starts at once unless another issue of the account is sending.
pub async fn request(
    state: &AppState,
    c: &Campaign,
    send_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Result<(), ApiError> {
    if !matches!(c.status, CampaignStatus::Draft | CampaignStatus::Scheduled) {
        return Err(ApiError::Conflict(
            "campaign_not_sendable",
            "only a draft or a scheduled issue can be sent".into(),
        ));
    }
    if send_at.is_some_and(|at| at > now + Duration::days(MAX_SCHEDULE_DAYS)) {
        return Err(ApiError::Validation(format!(
            "send_at can be at most {MAX_SCHEDULE_DAYS} days ahead"
        )));
    }
    let report = checks::run(state, c).await?;
    if !report.passed() {
        return Err(report.failure());
    }
    let at = send_at.filter(|t| *t > now).unwrap_or(now);
    sqlx::query!(
        "UPDATE campaigns SET status = 'scheduled', send_at = $2, paused_reason = NULL, updated_at = now()
         WHERE id = $1 AND status IN ('draft', 'scheduled')",
        c.id,
        at
    )
    .execute(&state.pool)
    .await?;
    if at <= now {
        start_due(state, now, Some(c.id)).await?;
    }
    tracing::info!(campaign_id = %c.id, "campaign send requested");
    Ok(())
}

/// Starts scheduled issues whose time has come, one per account at a time.
pub async fn start_due(
    state: &AppState,
    now: DateTime<Utc>,
    only: Option<Uuid>,
) -> anyhow::Result<()> {
    let due = sqlx::query_scalar!(
        "SELECT c.id FROM campaigns c
         WHERE c.status = 'scheduled' AND c.send_at <= $1 AND ($2::uuid IS NULL OR c.id = $2)
           AND NOT EXISTS (SELECT 1 FROM campaigns o WHERE o.account_id = c.account_id AND o.status = 'sending')
         ORDER BY c.send_at LIMIT 20",
        now,
        only
    )
    .fetch_all(&state.pool)
    .await?;
    for id in due {
        if let Err(e) = start_one(state, id, now).await {
            tracing::error!(campaign_id = %id, error = ?e, "campaign start failed");
        }
    }
    Ok(())
}

async fn start_one(state: &AppState, id: Uuid, now: DateTime<Utc>) -> anyhow::Result<()> {
    let mut tx = state.pool.begin().await?;
    let Some(c) = lock_skip(&mut tx, id).await? else {
        return Ok(()); // another engine process has it
    };
    if c.status != CampaignStatus::Scheduled {
        return Ok(());
    }
    let report = checks::run(state, &c).await?;
    if !report.passed() {
        // Back to draft (not paused): a draft can be fixed, deleted or sent again. The reason stays for the editor.
        let reason = format!("checks failed at start: {}", report.failed_messages());
        sqlx::query!(
            "UPDATE campaigns SET status = 'draft', send_at = NULL, paused_reason = $2, updated_at = now() WHERE id = $1",
            id,
            reason
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        tracing::warn!(campaign_id = %id, "scheduled campaign failed its checks at start; back to draft");
        return Ok(());
    }
    if start(&mut tx, &c, now).await? {
        tx.commit().await?;
        tracing::info!(campaign_id = %id, "campaign started");
    }
    Ok(())
}

/// In the caller's transaction: `sending`, the archive slug, and the reader snapshot with tiers and waves.
/// False when another issue of the account is already sending (the transaction is then unusable: drop it).
pub async fn start(
    conn: &mut PgConnection,
    c: &Campaign,
    now: DateTime<Utc>,
) -> anyhow::Result<bool> {
    let r = sqlx::query!(
        "UPDATE campaigns SET status = 'sending', started_at = $2, send_at = COALESCE(send_at, $2), wave = 0,
                next_wave_at = $2, paused_reason = NULL, updated_at = now()
         WHERE id = $1",
        c.id,
        now
    )
    .execute(&mut *conn)
    .await;
    match r {
        Err(sqlx::Error::Database(d)) if d.is_unique_violation() => return Ok(false),
        r => {
            r?;
        }
    }
    let base = plan::slug_from_subject(&c.subject);
    let taken = sqlx::query_scalar!(
        r#"SELECT archive_slug AS "s!" FROM campaigns
           WHERE account_id = $1 AND archive_slug IS NOT NULL AND (archive_slug = $2 OR archive_slug LIKE $2 || '-%')"#,
        c.account_id,
        base
    )
    .fetch_all(&mut *conn)
    .await?;
    sqlx::query!(
        "UPDATE campaigns SET archive_slug = $2 WHERE id = $1",
        c.id,
        plan::pick_slug(&base, &taken)
    )
    .execute(&mut *conn)
    .await?;

    let n = checks::audience(&mut *conn, c.account_id, &c.include_tags, &c.exclude_tags)
        .await?
        .readers;
    let ends = plan::wave_ends(&plan::wave_sizes(n as usize));
    // shortcut: tier 1 matches emails whose to_addresses holds the bare lower-case address (campaign rows
    // always do); an API email sent to "Name <addr>" does not count. Match on bare_address() if that matters.
    sqlx::query!(
        r#"WITH eng AS (
             SELECT a.addr, count(*) FILTER (WHERE ev.type = 'clicked') AS clicks,
                    count(*) FILTER (WHERE ev.type = 'opened') AS opens
             FROM email_events ev
             JOIN emails e ON e.id = ev.email_id
             CROSS JOIN LATERAL unnest(e.to_addresses) AS a(addr)
             WHERE ev.account_id = $1 AND ev.type IN ('opened', 'clicked') AND NOT ev.likely_bot
               AND ev.created_at > $5::timestamptz - interval '90 days'
             GROUP BY a.addr
           ), readers AS (
             SELECT c.id,
                    CASE WHEN eng.addr IS NOT NULL THEN 1
                         WHEN c.subscribed_at > $5::timestamptz - interval '30 days'
                              AND NOT EXISTS (SELECT 1 FROM emails e2 WHERE e2.contact_id = c.id) THEN 2
                         ELSE 3 END AS tier,
                    COALESCE(eng.clicks, 0) AS clicks, COALESCE(eng.opens, 0) AS opens
             FROM contacts c LEFT JOIN eng ON eng.addr = c.email
             WHERE c.account_id = $1 AND c.status = 'subscribed'
               AND (cardinality($2::uuid[]) = 0
                    OR EXISTS (SELECT 1 FROM contact_tags x WHERE x.contact_id = c.id AND x.tag_id = ANY($2)))
               AND NOT EXISTS (SELECT 1 FROM contact_tags x WHERE x.contact_id = c.id AND x.tag_id = ANY($3))
               AND NOT EXISTS (SELECT 1 FROM suppressions s WHERE s.account_id = c.account_id AND s.address = c.email)
           ), ordered AS (
             SELECT id, tier, (row_number() OVER (ORDER BY tier, clicks DESC, opens DESC, random()))::int AS pos
             FROM readers
           )
           INSERT INTO campaign_recipients (campaign_id, contact_id, tier, position, wave)
           SELECT $4, id, tier::smallint, pos, 1 + (SELECT count(*) FROM unnest($6::int[]) AS b(e) WHERE b.e < pos)::int
           FROM ordered"#,
        c.account_id,
        &c.include_tags,
        &c.exclude_tags,
        c.id,
        now,
        &ends
    )
    .execute(&mut *conn)
    .await?;
    Ok(true)
}

/// Deletes this issue's emails that are still queued and frees their readers for a later wave.
/// Emails a worker already claimed (`sending`) are left alone.
pub async fn take_back(conn: &mut PgConnection, campaign_id: Uuid) -> sqlx::Result<u64> {
    let ids = sqlx::query_scalar!(
        "DELETE FROM emails WHERE campaign_id = $1 AND status = 'queued' RETURNING id",
        campaign_id
    )
    .fetch_all(&mut *conn)
    .await?;
    sqlx::query!(
        "UPDATE campaign_recipients SET email_id = NULL WHERE campaign_id = $1 AND email_id = ANY($2)",
        campaign_id,
        &ids
    )
    .execute(&mut *conn)
    .await?;
    Ok(ids.len() as u64)
}

/// The worker claimed an email of an issue that is no longer sending (paused or cancelled after the
/// wave was queued): delete it and free the reader, so Resume queues them again.
pub async fn take_back_one(pool: &sqlx::PgPool, email_id: Uuid) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    let gone = sqlx::query_scalar!(
        "DELETE FROM emails WHERE id = $1 AND status = 'sending' AND campaign_id IS NOT NULL RETURNING campaign_id",
        email_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(Some(campaign_id)) = gone {
        sqlx::query!(
            "UPDATE campaign_recipients SET email_id = NULL WHERE campaign_id = $1 AND email_id = $2",
            campaign_id,
            email_id
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}

pub async fn pause_in_tx(conn: &mut PgConnection, id: Uuid, reason: &str) -> sqlx::Result<()> {
    take_back(conn, id).await?;
    sqlx::query!(
        "UPDATE campaigns SET status = 'paused', paused_reason = $2, next_wave_at = NULL, updated_at = now() WHERE id = $1",
        id,
        reason
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn pause(state: &AppState, c: &Campaign, reason: &str) -> Result<(), ApiError> {
    let mut tx = state.pool.begin().await?;
    let status = sqlx::query_scalar!(
        r#"SELECT status AS "s: CampaignStatus" FROM campaigns WHERE id = $1 FOR UPDATE"#,
        c.id
    )
    .fetch_one(&mut *tx)
    .await?;
    if status != CampaignStatus::Sending {
        return Err(ApiError::Conflict(
            "campaign_not_sending",
            "only a sending issue can be paused".into(),
        ));
    }
    pause_in_tx(&mut tx, c.id, reason).await?;
    tx.commit().await?;
    tracing::info!(campaign_id = %c.id, "campaign paused");
    Ok(())
}

/// Checks again, then `sending` with the next wave due now. An issue that never started gets the full start.
pub async fn resume(state: &AppState, c: &Campaign, now: DateTime<Utc>) -> Result<(), ApiError> {
    if c.status != CampaignStatus::Paused {
        return Err(ApiError::Conflict(
            "campaign_not_paused",
            "only a paused issue can be resumed".into(),
        ));
    }
    let report = checks::run(state, c).await?;
    if !report.passed() {
        return Err(report.failure());
    }
    let another = || {
        ApiError::Conflict(
            "another_sending",
            "another issue of this account is sending; resume this one when it ends".into(),
        )
    };
    let mut tx = state.pool.begin().await?;
    let Some(locked) = lock_skip(&mut tx, c.id).await? else {
        return Err(ApiError::Conflict(
            "campaign_busy",
            "this issue is being changed; try again".into(),
        ));
    };
    if locked.status != CampaignStatus::Paused {
        return Err(ApiError::Conflict(
            "campaign_not_paused",
            "only a paused issue can be resumed".into(),
        ));
    }
    if locked.started_at.is_none() {
        if !start(&mut tx, &locked, now).await? {
            return Err(another());
        }
    } else {
        let r = sqlx::query!(
            "UPDATE campaigns SET status = 'sending', paused_reason = NULL, next_wave_at = $2, updated_at = now() WHERE id = $1",
            c.id,
            now
        )
        .execute(&mut *tx)
        .await;
        match r {
            Err(sqlx::Error::Database(d)) if d.is_unique_violation() => return Err(another()),
            r => {
                r?;
            }
        }
    }
    tx.commit().await?;
    tracing::info!(campaign_id = %c.id, "campaign resumed");
    Ok(())
}

pub async fn cancel(state: &AppState, c: &Campaign, now: DateTime<Utc>) -> Result<(), ApiError> {
    let mut tx = state.pool.begin().await?;
    let status = sqlx::query_scalar!(
        r#"SELECT status AS "s: CampaignStatus" FROM campaigns WHERE id = $1 FOR UPDATE"#,
        c.id
    )
    .fetch_one(&mut *tx)
    .await?;
    if !matches!(
        status,
        CampaignStatus::Scheduled | CampaignStatus::Sending | CampaignStatus::Paused
    ) {
        return Err(ApiError::Conflict(
            "campaign_not_cancellable",
            "only a scheduled, sending or paused issue can be cancelled; delete a draft instead"
                .into(),
        ));
    }
    take_back(&mut tx, c.id).await?;
    sqlx::query!(
        "UPDATE campaigns SET status = 'cancelled', finished_at = $2, next_wave_at = NULL, updated_at = now() WHERE id = $1",
        c.id,
        now
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    tracing::info!(campaign_id = %c.id, "campaign cancelled");
    Ok(())
}

/// An account that is paused (auto-pause or by hand) stops its issue: no new waves, queued emails back.
async fn pause_for_paused_accounts(state: &AppState) -> anyhow::Result<()> {
    let rows = sqlx::query!(
        r#"SELECT c.id, COALESCE(a.paused_reason, 'no reason given') AS "reason!"
           FROM campaigns c JOIN accounts a ON a.id = c.account_id
           WHERE c.status = 'sending' AND a.status = 'paused'"#
    )
    .fetch_all(&state.pool)
    .await?;
    for r in rows {
        if let Err(e) = pause_one(state, r.id, &r.reason).await {
            tracing::error!(campaign_id = %r.id, error = ?e, "pausing a campaign with its account failed");
        }
    }
    Ok(())
}

async fn pause_one(state: &AppState, id: Uuid, account_reason: &str) -> sqlx::Result<()> {
    let mut tx = state.pool.begin().await?;
    if let Some(c) = lock_skip(&mut tx, id).await?
        && c.status == CampaignStatus::Sending
    {
        pause_in_tx(
            &mut tx,
            id,
            &format!("the account is paused: {account_reason}"),
        )
        .await?;
        tx.commit().await?;
        tracing::warn!(campaign_id = %id, "campaign paused with its account");
    }
    Ok(())
}

/// One round of the campaign loop. `now` is the test hook: tests move it instead of sleeping.
pub async fn tick(state: &AppState, now: DateTime<Utc>) -> anyhow::Result<()> {
    // Each part logs its own failure and the round goes on: one bad row must not stop all sending.
    if let Err(e) = pause_for_paused_accounts(state).await {
        tracing::error!(error = ?e, "pausing campaigns of paused accounts failed");
    }
    if let Err(e) = start_due(state, now, None).await {
        tracing::error!(error = ?e, "starting due campaigns failed");
    }
    let sending = sqlx::query_scalar!(
        "SELECT id FROM campaigns WHERE status = 'sending' ORDER BY next_wave_at NULLS FIRST LIMIT 100"
    )
    .fetch_all(&state.pool)
    .await?;
    for id in sending {
        if let Err(e) = step(state, id, now).await {
            tracing::error!(campaign_id = %id, error = ?e, "campaign step failed");
        }
    }
    Ok(())
}

async fn step(state: &AppState, id: Uuid, now: DateTime<Utc>) -> anyhow::Result<()> {
    let mut tx = state.pool.begin().await?;
    let Some(c) = lock_skip(&mut tx, id).await? else {
        return Ok(());
    };
    if c.status != CampaignStatus::Sending {
        return Ok(());
    }
    match c.next_wave_at {
        Some(at) if at <= now => {
            let Some(sender) = checks::sender(&state.pool, c.account_id).await? else {
                pause_in_tx(
                    &mut tx,
                    id,
                    "the from address is not on a verified domain any more",
                )
                .await?;
                tx.commit().await?;
                return Ok(());
            };
            // Readers of a started wave that were taken back (pause) go first; else the next wave starts.
            let mut wave = c.wave;
            let mut n = queue_readers(&mut tx, &c, &sender, wave).await?;
            if n == 0 {
                wave += 1;
                n = queue_readers(&mut tx, &c, &sender, wave).await?;
            }
            sqlx::query!(
                "UPDATE campaigns SET wave = $2, next_wave_at = NULL, updated_at = now() WHERE id = $1",
                id,
                wave
            )
            .execute(&mut *tx)
            .await?;
            tracing::info!(campaign_id = %id, wave, queued = n, "campaign wave queued");
        }
        Some(_) => {} // inside the gap
        None => {
            let open = sqlx::query_scalar!(
                r#"SELECT count(*) AS "n!" FROM emails WHERE campaign_id = $1 AND status IN ('queued', 'sending', 'unknown')"#,
                id
            )
            .fetch_one(&mut *tx)
            .await?;
            if open == 0 {
                let r = sqlx::query!(
                    r#"SELECT COALESCE(max(wave), 0) AS "last!", count(*) FILTER (WHERE email_id IS NULL) AS "left!"
                       FROM campaign_recipients WHERE campaign_id = $1"#,
                    id
                )
                .fetch_one(&mut *tx)
                .await?;
                if c.wave >= r.last && r.left == 0 {
                    sqlx::query!(
                        "UPDATE campaigns SET status = 'sent', finished_at = $2, next_wave_at = NULL, updated_at = now() WHERE id = $1",
                        id,
                        now
                    )
                    .execute(&mut *tx)
                    .await?;
                    tracing::info!(campaign_id = %id, "campaign sent");
                } else {
                    sqlx::query!(
                        "UPDATE campaigns SET next_wave_at = $2, updated_at = now() WHERE id = $1",
                        id,
                        now + Duration::minutes(plan::WAVE_GAP_MINUTES)
                    )
                    .execute(&mut *tx)
                    .await?;
                }
            }
        }
    }
    tx.commit().await?;
    Ok(())
}

/// Inserts one `emails` row per reader of waves up to `wave` that has none yet, and links it, in the
/// caller's transaction (so a crash can never leave a reader queued twice or not at all).
async fn queue_readers(
    conn: &mut PgConnection,
    c: &Campaign,
    sender: &checks::Sender,
    wave: i32,
) -> anyhow::Result<usize> {
    let rows = sqlx::query!(
        "SELECT r.contact_id, ct.email FROM campaign_recipients r JOIN contacts ct ON ct.id = r.contact_id
         WHERE r.campaign_id = $1 AND r.wave <= $2 AND r.email_id IS NULL ORDER BY r.position",
        c.id,
        wave
    )
    .fetch_all(&mut *conn)
    .await?;
    if rows.is_empty() {
        return Ok(0);
    }
    let ids: Vec<Uuid> = rows.iter().map(|_| Uuid::now_v7()).collect();
    let contacts: Vec<Uuid> = rows.iter().map(|r| r.contact_id).collect();
    let addrs: Vec<String> = rows.iter().map(|r| r.email.clone()).collect();
    sqlx::query!(
        "INSERT INTO emails (id, account_id, domain_id, from_address, to_addresses, subject, priority, campaign_id, contact_id)
         SELECT x.id, $1, $2, $3, ARRAY[x.email], $4, 'bulk', $5, x.contact_id
         FROM unnest($6::uuid[], $7::uuid[], $8::text[]) AS x(id, contact_id, email)",
        c.account_id,
        sender.domain_id,
        sender.from,
        c.subject,
        c.id,
        &ids,
        &contacts,
        &addrs
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "UPDATE campaign_recipients r SET email_id = x.id
         FROM unnest($2::uuid[], $3::uuid[]) AS x(id, contact_id)
         WHERE r.campaign_id = $1 AND r.contact_id = x.contact_id",
        c.id,
        &ids,
        &contacts
    )
    .execute(&mut *conn)
    .await?;
    Ok(rows.len())
}

pub async fn run_loop(state: AppState, shutdown: CancellationToken) -> anyhow::Result<()> {
    background::every(StdDuration::from_secs(TICK_SECS), &shutdown, || async {
        if let Err(e) = tick(&state, Utc::now()).await {
            tracing::error!(error = ?e, "campaign tick failed");
        }
    })
    .await;
    Ok(())
}
