use std::{net::SocketAddr, num::NonZeroU32, sync::Arc};

use axum::{Router, extract::DefaultBodyLimit};
use governor::{Quota, RateLimiter};
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::{config::Config, sender::Sender};

pub mod auth;
pub mod domains;
pub mod emails;
pub mod health;
pub mod json;
pub mod ses;
pub mod webhooks;

// shortcut: in-memory, per process; keys never expire from the map. Fine for one API process.
pub type KeyLimiter = governor::DefaultKeyedRateLimiter<Uuid>;

pub const REQUESTS_PER_SECOND_PER_KEY: u32 = 10;
pub const MAX_BODY_BYTES: usize = 50 * 1024 * 1024;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
    pub sender: Arc<dyn Sender>,
    pub key_limiter: Arc<KeyLimiter>,
    pub sns_certs: Arc<crate::events::sns::CertCache>,
}

impl AppState {
    pub fn new(pool: PgPool, config: Config, sender: Arc<dyn Sender>) -> Self {
        let per_second = NonZeroU32::new(REQUESTS_PER_SECOND_PER_KEY).unwrap_or(NonZeroU32::MIN);
        Self {
            pool,
            config: Arc::new(config),
            sender,
            key_limiter: Arc::new(RateLimiter::keyed(Quota::per_second(per_second))),
            sns_certs: Arc::new(crate::events::sns::CertCache::new()),
        }
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(health::routes())
        .merge(domains::routes())
        .merge(emails::routes())
        .merge(ses::routes())
        .merge(webhooks::routes())
        .merge(crate::tracking::routes())
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub async fn serve(state: AppState, shutdown: CancellationToken) -> anyhow::Result<()> {
    let addr = state.config.bind_addr;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "api listening");
    axum::serve(
        listener,
        router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown.cancelled_owned())
    .await?;
    Ok(())
}
