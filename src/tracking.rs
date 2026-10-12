use std::net::SocketAddr;

use axum::{
    Router,
    extract::{ConnectInfo, Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::json;
use sqlx::types::Json;
use uuid::Uuid;

use crate::{
    api::AppState,
    email_events::{self, EventKind, NewEvent},
    tokens::{self, TokenKind},
    webhooks::ssrf::is_public_ip,
};

/// A 1x1 transparent GIF (43 bytes).
pub const PIXEL_GIF: &[u8] = &[
    0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x01, 0x00, 0x01, 0x00, 0x80, 0x00, 0x00, 0xff, 0xff, 0xff,
    0x00, 0x00, 0x00, 0x21, 0xf9, 0x04, 0x01, 0x00, 0x00, 0x00, 0x00, 0x2c, 0x00, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x01, 0x00, 0x00, 0x02, 0x02, 0x44, 0x01, 0x00, 0x3b,
];
/// Clicks this soon after sending are usually a mail server's link scanner, not a person.
pub const BOT_WINDOW_SECS: i64 = 10;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/o/{file}", get(open))
        .route("/c/{token}", get(click))
        .route("/internal/tls-allowed", get(tls_allowed))
}

async fn record(
    state: &AppState,
    email_id: Uuid,
    kind: EventKind,
    data: serde_json::Value,
    likely_bot: bool,
) -> anyhow::Result<()> {
    let Some(account_id) =
        sqlx::query_scalar!("SELECT account_id FROM emails WHERE id = $1", email_id)
            .fetch_optional(&state.pool)
            .await?
    else {
        return Ok(());
    };
    let mut conn = state.pool.acquire().await?;
    email_events::record(
        &mut conn,
        NewEvent {
            account_id,
            email_id,
            kind,
            recipient: None,
            data,
            likely_bot,
        },
    )
    .await?;
    Ok(())
}

pub(crate) async fn open(State(state): State<AppState>, Path(file): Path<String>) -> Response {
    let token = file.strip_suffix(".gif").unwrap_or(&file);
    if let Some((email_id, TokenKind::Open)) = tokens::verify(&state.config.tracking_secret, token)
        && let Err(e) = record(&state, email_id, EventKind::Opened, json!({}), false).await
    {
        tracing::warn!(error = ?e, "could not record open");
    }
    // Always the same answer, so the pixel says nothing about whether a token was valid.
    (
        [
            (header::CONTENT_TYPE, "image/gif"),
            (header::CACHE_CONTROL, "no-store, max-age=0"),
        ],
        PIXEL_GIF,
    )
        .into_response()
}

pub(crate) async fn click(State(state): State<AppState>, Path(token): Path<String>) -> Response {
    let Some((email_id, TokenKind::Click(idx))) =
        tokens::verify(&state.config.tracking_secret, &token)
    else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let row = sqlx::query!(
        r#"SELECT links AS "links: Json<Vec<String>>", sent_at FROM emails WHERE id = $1"#,
        email_id
    )
    .fetch_optional(&state.pool)
    .await;
    let Ok(Some(row)) = row else {
        return StatusCode::NOT_FOUND.into_response();
    };
    // Only ever redirect to a link we saved at send time, never to anything taken from the request.
    let Some(target) = row
        .links
        .0
        .get(idx as usize)
        .and_then(|l| url::Url::parse(l).ok())
    else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let likely_bot = row
        .sent_at
        .is_some_and(|t| Utc::now() - t < Duration::seconds(BOT_WINDOW_SECS));
    if let Err(e) = record(
        &state,
        email_id,
        EventKind::Clicked,
        json!({ "link": target.as_str() }),
        likely_bot,
    )
    .await
    {
        tracing::warn!(error = ?e, "could not record click");
    }
    (
        StatusCode::FOUND,
        [(header::LOCATION, target.as_str().to_string())],
    )
        .into_response()
}

#[derive(Deserialize)]
pub struct TlsQuery {
    pub domain: String,
}

pub fn internal_caller(peer: SocketAddr, headers: &HeaderMap) -> bool {
    !is_public_ip(peer.ip()) && !headers.contains_key("x-forwarded-for")
}

/// Caddy asks this before getting a certificate for a host. Yes only for our own host and verified tracking subdomains.
pub(crate) async fn tls_allowed(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<TlsQuery>,
) -> StatusCode {
    if !internal_caller(peer, &headers) {
        return StatusCode::NOT_FOUND;
    }
    let host = q.domain.trim_end_matches('.').to_ascii_lowercase();
    if host == state.config.public_host() {
        return StatusCode::OK;
    }
    match sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM domains WHERE tracking_subdomain = $1 AND tracking_verified) AS "e!""#,
        host
    )
    .fetch_one(&state.pool)
    .await
    {
        Ok(true) => StatusCode::OK,
        Ok(false) => StatusCode::NOT_FOUND,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}
