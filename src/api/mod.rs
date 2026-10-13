use std::{
    net::{IpAddr, SocketAddr},
    num::NonZeroU32,
    sync::Arc,
};

use axum::{Router, extract::DefaultBodyLimit};
use governor::{Quota, RateLimiter};
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::{config::Config, sender::Sender};

pub mod auth;
pub mod contacts;
pub mod domains;
pub mod emails;
pub mod health;
pub mod imports;
pub mod json;
pub mod keys;
pub mod login;
pub mod me;
pub mod newsletter;
pub mod public;
pub mod ses;
pub mod stats;
pub mod suppressions;
pub mod tags;
pub mod team;
pub mod webhooks;

// shortcut: in-memory, per process; keys never expire from the map. Fine for one API process.
pub type KeyLimiter = governor::DefaultKeyedRateLimiter<Uuid>;

pub const REQUESTS_PER_SECOND_PER_KEY: u32 = 10;

// shortcut: in-memory, per process, like KeyLimiter.
pub type IpLimiter = governor::DefaultKeyedRateLimiter<IpAddr>;
pub const PUBLIC_REQUESTS_PER_SECOND_PER_IP: u32 = 10;
pub const MAX_BODY_BYTES: usize = 64 * 1024 * 1024; // 40 MB of attachments is ~54 MB as base64 JSON

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
    pub sender: Arc<dyn Sender>,
    pub key_limiter: Arc<KeyLimiter>,
    pub ip_limiter: Arc<IpLimiter>,
    pub sns_certs: Arc<crate::events::sns::CertCache>,
}

impl AppState {
    pub fn new(pool: PgPool, config: Config, sender: Arc<dyn Sender>) -> Self {
        let per_second = NonZeroU32::new(REQUESTS_PER_SECOND_PER_KEY).unwrap_or(NonZeroU32::MIN);
        let per_ip = NonZeroU32::new(PUBLIC_REQUESTS_PER_SECOND_PER_IP).unwrap_or(NonZeroU32::MIN);
        Self {
            pool,
            config: Arc::new(config),
            sender,
            key_limiter: Arc::new(RateLimiter::keyed(Quota::per_second(per_second))),
            ip_limiter: Arc::new(RateLimiter::keyed(Quota::per_second(per_ip))),
            sns_certs: Arc::new(crate::events::sns::CertCache::new()),
        }
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(health::routes())
        .merge(contacts::routes())
        .merge(imports::routes())
        .merge(newsletter::routes())
        .merge(public::routes())
        .merge(domains::routes())
        .merge(emails::routes())
        .merge(ses::routes())
        .merge(webhooks::routes())
        .merge(keys::routes())
        .merge(login::routes())
        .merge(me::routes())
        .merge(stats::routes())
        .merge(suppressions::routes())
        .merge(tags::routes())
        .merge(team::routes())
        .merge(crate::tracking::routes())
        .merge(crate::openapi::routes())
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
