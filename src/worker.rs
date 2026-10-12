use std::{collections::HashSet, sync::Arc, time::Duration};

use futures::StreamExt;
use serde_json::json;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    accounts::{self, AccountStatus},
    api::AppState,
    config::Config,
    domains::{self, Domain},
    email_events::{self, EventKind, NewEvent},
    emails,
    limiter::SendLimiter,
    queue::{self, ClaimedEmail},
    render::{self, RenderInput, Tracking},
    safety,
    sender::{OutgoingEmail, SenderError},
};

pub async fn run(state: AppState, shutdown: CancellationToken) -> anyhow::Result<()> {
    let limiter = Arc::new(SendLimiter::new(
        state.config.ses_max_send_rate,
        state.config.worker_processes,
    ));
    tracing::info!("worker started");
    while !shutdown.is_cancelled() {
        let idle = match tick(&state, &limiter).await {
            Ok(n) => n == 0,
            Err(e) => {
                tracing::error!(error = ?e, "worker tick failed");
                true
            }
        };
        if idle {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(1)) => {}
                _ = shutdown.cancelled() => {}
            }
        }
    }
    Ok(())
}

/// Claims one batch and processes it. Returns how many emails it claimed.
pub async fn tick(state: &AppState, limiter: &SendLimiter) -> anyhow::Result<usize> {
    let gate = crate::quota::gate(&state.pool).await?;
    let rate = crate::limiter::effective_rate(state.config.ses_max_send_rate, gate.max_send_rate);
    limiter.set_global_rate(rate);
    let limit = if gate.probing {
        1
    } else {
        claim_size(
            state.config.worker_batch,
            rate,
            state.config.worker_processes,
        )
    };
    let batch = queue::claim_through(&state.pool, limit, &gate).await?;
    let claimed = batch.len();
    let touched: HashSet<Uuid> = batch.iter().map(|e| e.account_id).collect();
    futures::stream::iter(batch)
        .for_each_concurrent(
            state.config.worker_concurrency as usize,
            |email| async move {
                let id = email.id;
                // On error the row stays 'sending'; the reaper turns it into 'unknown' and retries it.
                if let Err(e) = process(state, limiter, email).await {
                    tracing::error!(email_id = %id, error = ?e, "processing failed");
                }
            },
        )
        .await;
    if gate.probing && claimed > 0 {
        crate::quota::end_probe(&state.pool).await?;
    }
    after_batch(state, touched).await;
    Ok(claimed)
}

/// Emails per round: never more than the send rate can get through in half the lease, even if every email
/// has the most recipients. Otherwise emails at the back of a slow round lose their lease and burn tries unsent.
fn claim_size(batch: u32, rate: u32, processes: u32) -> i64 {
    let per_process = i64::from((rate / processes.max(1)).max(1));
    let budget = per_process * queue::LEASE_SECS / 2 / emails::MAX_RECIPIENTS as i64;
    i64::from(batch).min(budget.max(1))
}

async fn after_batch(state: &AppState, accounts: HashSet<Uuid>) {
    for id in accounts {
        if let Err(e) = safety::check_account(&state.pool, id).await {
            tracing::error!(account_id = %id, error = ?e, "auto-pause check failed");
        }
    }
}

fn tracking_for<'a>(config: &'a Config, domain: &Domain) -> Option<Tracking<'a>> {
    if !domain.open_tracking && !domain.click_tracking {
        return None;
    }
    let base_url = match (&domain.tracking_subdomain, domain.tracking_verified) {
        (Some(host), true) => format!("https://{host}"),
        _ => config.public_base(),
    };
    Some(Tracking {
        base_url,
        secret: &config.tracking_secret,
        opens: domain.open_tracking,
        clicks: domain.click_tracking,
    })
}

async fn process(
    state: &AppState,
    limiter: &SendLimiter,
    email: ClaimedEmail,
) -> anyhow::Result<()> {
    let pool = &state.pool;
    let Some(account) = accounts::get(pool, email.account_id).await? else {
        return Ok(());
    };
    if account.status == AccountStatus::Paused {
        queue::requeue_paused(pool, email.id).await?;
        return Ok(());
    }
    let domain = match email.domain_id {
        Some(id) => domains::get_by_id(pool, id).await?,
        None => None,
    };
    let Some(domain) = domain else {
        queue::mark_failed(pool, &email, "sending domain was removed").await?;
        return Ok(());
    };

    // Envelope: everyone, as bare lowercase addresses, each once, minus the block list.
    let mut seen = HashSet::new();
    let everyone: Vec<String> = email
        .to_addresses
        .iter()
        .chain(&email.cc_addresses)
        .chain(&email.bcc_addresses)
        .map(|a| render::bare_address(a))
        .filter(|a| seen.insert(a.clone()))
        .collect();
    let blocked = safety::suppressed_among(pool, account.id, &everyone).await?;
    let (removed, recipients): (Vec<String>, Vec<String>) =
        everyone.into_iter().partition(|a| blocked.contains(a));
    if recipients.is_empty() {
        queue::mark_suppressed(pool, &email, &removed).await?;
        return Ok(());
    }
    // Once per email: a requeue (daily cap, retry) comes back through here.
    let already = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM email_events WHERE email_id = $1 AND type = 'suppressed') AS "already!""#,
        email.id
    )
    .fetch_one(pool)
    .await?;
    if !removed.is_empty() && !already {
        let mut conn = pool.acquire().await?;
        email_events::record(
            &mut conn,
            NewEvent {
                account_id: account.id,
                email_id: email.id,
                kind: EventKind::Suppressed,
                recipient: None,
                data: json!({ "recipients": removed, "partial": true }),
                likely_bot: false,
            },
        )
        .await?;
    }

    let attachments = emails::load_attachments(pool, email.id).await?;
    let input = RenderInput {
        email_id: email.id,
        from: &email.from_address,
        to: &email.to_addresses,
        cc: &email.cc_addresses,
        reply_to: &email.reply_to,
        subject: &email.subject,
        html: email.html.as_deref(),
        text: email.text_body.as_deref(),
        headers: &email.headers.0,
        attachments: &attachments,
        tracking: tracking_for(&state.config, &domain),
    };
    let rendered = match render::render(&input) {
        Ok(r) => r,
        Err(e) => {
            queue::mark_failed(pool, &email, &e.to_string()).await?;
            return Ok(());
        }
    };
    if !rendered.links.is_empty() {
        queue::save_links(pool, email.id, &rendered.links).await?;
    }

    limiter
        .wait(
            account.id,
            account.send_rate_per_sec.max(1) as u32,
            recipients.len(),
        )
        .await;
    // The limiter wait can be longer than the lease; never send an email the reaper already gave to someone else.
    if !queue::renew_lease(pool, &email).await? {
        tracing::warn!(email_id = %email.id, "lease lost while waiting to send; skipped");
        return Ok(());
    }
    // Another email in this batch may have hit the daily cap while this one waited: do not knock on SES again.
    if crate::quota::paused(pool).await? {
        queue::requeue_paused(pool, email.id).await?;
        return Ok(());
    }
    let out = OutgoingEmail {
        email_id: email.id,
        tenant: account.tenant(),
        from: email.from_address.clone(),
        recipients,
        tags: email.tags.0.clone(),
        raw: rendered.raw,
    };
    match state.sender.send(&out).await {
        Ok(provider_id) => {
            queue::mark_sent(pool, &email, &provider_id, &out.recipients).await?;
            if state.config.sender == crate::config::SenderKind::Fake
                && let Err(e) =
                    crate::events::simulate(pool, email.id, &provider_id, &out.recipients).await
            {
                tracing::warn!(email_id = %email.id, error = ?e, "simulated events failed");
            }
        }
        Err(SenderError::Unknown(m)) => queue::mark_unknown(pool, email.id, &m).await?,
        Err(SenderError::Retryable(m)) => match queue::retry_delay(email.attempts) {
            Some(d) => queue::mark_retry(pool, email.id, &m, d).await?,
            None => queue::mark_failed(pool, &email, &m).await?,
        },
        Err(SenderError::Permanent(m)) | Err(SenderError::NotConfigured(m)) => {
            queue::mark_failed(pool, &email, &m).await?
        }
        Err(SenderError::DailyQuota(m)) => {
            tracing::warn!(email_id = %email.id, "daily sending quota used up; pausing all sending");
            // Pause first: if the requeue then fails, the reaper recovers the email; the other order could
            // leave it queued with no pause, and the next round would knock on SES again at once.
            crate::quota::pause(pool, &m).await?;
            queue::requeue_paused(pool, email.id).await?;
        }
        Err(SenderError::TenantPaused) => {
            accounts::pause(
                pool,
                account.id,
                "the sender paused this account (ses_tenant_paused)",
            )
            .await?;
            queue::requeue_paused(pool, email.id).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_round_never_claims_more_than_the_lease_can_send() {
        // 300 s lease, half of it as the budget, 50 recipients per email at worst.
        assert_eq!(claim_size(50, 14, 1), 42);
        assert_eq!(claim_size(50, 1, 1), 3);
        assert_eq!(claim_size(50, 1, 4), 3);
        assert_eq!(claim_size(50, 10_000, 1), 50);
        assert_eq!(claim_size(50, 28, 2), 42);
    }
}
