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
    tasks.spawn(maintenance_loop(state, shutdown));
}

/// Hourly cleanup: attachments of finished emails after 7 days, SNS dedupe ids after 30 days.
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
    })
    .await;
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
