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

pub const CONCURRENCY: usize = 10;

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
    let batch = queue::claim(&state.pool, queue::BATCH).await?;
    let claimed = batch.len();
    let touched: HashSet<Uuid> = batch.iter().map(|e| e.account_id).collect();
    futures::stream::iter(batch)
        .for_each_concurrent(CONCURRENCY, |email| async move {
            let id = email.id;
            // On error the row stays 'sending'; the reaper turns it into 'unknown' and retries it.
            if let Err(e) = process(state, limiter, email).await {
                tracing::error!(email_id = %id, error = ?e, "processing failed");
            }
        })
        .await;
    after_batch(state, touched).await;
    Ok(claimed)
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
    if !removed.is_empty() {
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
    if !queue::renew_lease(pool, email.id).await? {
        tracing::warn!(email_id = %email.id, "lease lost while waiting to send; skipped");
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
            if state.config.sender == crate::config::SenderKind::Fake {
                if let Err(e) =
                    crate::events::simulate(pool, email.id, &provider_id, &out.recipients).await
                {
                    tracing::warn!(email_id = %email.id, error = ?e, "simulated events failed");
                }
            }
        }
        Err(SenderError::Retryable(m)) => match queue::retry_delay(email.attempts) {
            Some(d) => queue::mark_retry(pool, email.id, &m, d).await?,
            None => queue::mark_failed(pool, &email, &m).await?,
        },
        Err(SenderError::Permanent(m)) | Err(SenderError::NotConfigured(m)) => {
            queue::mark_failed(pool, &email, &m).await?
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
