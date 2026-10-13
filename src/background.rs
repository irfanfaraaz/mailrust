use std::{future::Future, time::Duration};

use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::api::AppState;

/// Starts every background loop. Later tasks add their loops here.
pub fn spawn_all(
    tasks: &mut JoinSet<anyhow::Result<()>>,
    state: AppState,
    shutdown: CancellationToken,
) {
    tasks.spawn(crate::domains::verify_loop(state.clone(), shutdown.clone()));
    tasks.spawn(crate::worker::run(state.clone(), shutdown.clone()));
    tasks.spawn(crate::queue::reaper_loop(state.clone(), shutdown.clone()));
    tasks.spawn(crate::webhooks::delivery_loop(
        state.clone(),
        shutdown.clone(),
    ));
    tasks.spawn(crate::quota::sync_loop(state.clone(), shutdown.clone()));
    tasks.spawn(crate::imports::worker_loop(state.clone(), shutdown.clone()));
    tasks.spawn(maintenance_loop(state, shutdown));
}

/// Hourly cleanup: attachments of finished emails after 7 days, SNS dedupe ids after 30 days,
/// dead login data (see `cleanup_auth`) and stale audience data (see `cleanup_audience`).
pub async fn maintenance_loop(state: AppState, shutdown: CancellationToken) -> anyhow::Result<()> {
    every(Duration::from_secs(3600), &shutdown, || async {
        let a = sqlx::query!(
            "DELETE FROM attachments a USING emails e
             WHERE a.email_id = e.id AND e.status NOT IN ('queued', 'sending', 'unknown')
               AND e.updated_at < now() - interval '7 days'"
        )
        .execute(&state.pool)
        .await;
        let s =
            sqlx::query!("DELETE FROM sns_messages WHERE received_at < now() - interval '30 days'")
                .execute(&state.pool)
                .await;
        if let Err(e) = a.and(s) {
            tracing::error!(error = ?e, "maintenance failed");
        }
        if let Err(e) = cleanup_auth(&state.pool).await {
            tracing::error!(error = ?e, "auth cleanup failed");
        }
        if let Err(e) = cleanup_audience(&state.pool).await {
            tracing::error!(error = ?e, "audience cleanup failed");
        }
    })
    .await;
    Ok(())
}

/// Expired sessions, old login tokens, login requests past the 1 h limit window, finished invites after 30 days.
pub async fn cleanup_auth(pool: &sqlx::PgPool) -> sqlx::Result<()> {
    sqlx::query!("DELETE FROM sessions WHERE expires_at < now()")
        .execute(pool)
        .await?;
    sqlx::query!("DELETE FROM login_tokens WHERE expires_at < now() - interval '1 day'")
        .execute(pool)
        .await?;
    sqlx::query!("DELETE FROM login_requests WHERE at < now() - interval '1 hour'")
        .execute(pool)
        .await?;
    sqlx::query!(
        "DELETE FROM invites WHERE created_at < now() - interval '30 days'
           AND (accepted_at IS NOT NULL OR revoked_at IS NOT NULL OR expires_at < now())"
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Pending signups nobody confirmed in 30 days, dead confirm links, join limits past their 1 h window.
/// A pending contact who had unsubscribed before goes back to unsubscribed instead of being deleted,
/// so a later import or API add can never re-subscribe them.
pub async fn cleanup_audience(pool: &sqlx::PgPool) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE contacts SET status = 'unsubscribed', updated_at = now()
         WHERE status = 'pending' AND unsubscribed_at IS NOT NULL AND updated_at < now() - interval '30 days'"
    )
    .execute(pool)
    .await?;
    sqlx::query!(
        "DELETE FROM contacts WHERE status = 'pending' AND unsubscribed_at IS NULL AND updated_at < now() - interval '30 days'"
    )
    .execute(pool)
    .await?;
    sqlx::query!("DELETE FROM contact_confirmations WHERE expires_at < now() - interval '1 day'")
        .execute(pool)
        .await?;
    sqlx::query!("DELETE FROM join_requests WHERE created_at < now() - interval '1 hour'")
        .execute(pool)
        .await?;
    Ok(())
}

/// Runs `f` now and then every `period` until shutdown.
pub async fn every<F, Fut>(period: Duration, shutdown: &CancellationToken, mut f: F)
where
    F: FnMut() -> Fut,
    Fut: Future<Output = ()>,
{
    loop {
        f().await;
        tokio::select! {
            _ = tokio::time::sleep(period) => {}
            _ = shutdown.cancelled() => break,
        }
    }
}
