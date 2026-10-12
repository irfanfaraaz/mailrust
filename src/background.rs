use std::{future::Future, time::Duration};

use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::api::AppState;

/// Starts every background loop. Later tasks add their loops here.
pub fn spawn_all(tasks: &mut JoinSet<anyhow::Result<()>>, state: AppState, shutdown: CancellationToken) {
    let _ = (tasks, state, shutdown);
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
