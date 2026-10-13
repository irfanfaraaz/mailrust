use std::net::{IpAddr, SocketAddr};

use axum::{
    Json, Router,
    extract::{ConnectInfo, FromRequestParts, State},
    http::{StatusCode, request::Parts},
    routing::post,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{AppState, json::ApiJson};
use crate::{auth_tokens, error::ApiError, keys::hash_token, sessions, system_mail, users};

pub const LOGIN_TOKEN_MINUTES: i64 = 15;
pub const PER_EMAIL_PER_HOUR: i64 = 5;
pub const PER_IP_PER_HOUR: i64 = 20;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/verify", post(verify))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct LoginRequest {
    pub email: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct TokenRequest {
    pub token: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SessionResponse {
    pub session_token: String,
    pub expires_at: DateTime<Utc>,
}

/// The browser's IP: from `X-Mailrust-Client-Ip` when the dashboard proves itself with
/// `X-Mailrust-Web`, else the TCP peer. None in tests without either.
pub struct ClientIp(pub Option<IpAddr>);

impl FromRequestParts<AppState> for ClientIp {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let h = &parts.headers;
        let trusted = match (&state.config.web_secret, h.get("x-mailrust-web")) {
            (Some(secret), Some(v)) => v.as_bytes() == secret.as_bytes(),
            _ => false,
        };
        if trusted {
            if let Some(ip) = h
                .get("x-mailrust-client-ip")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.trim().parse().ok())
            {
                return Ok(ClientIp(Some(ip)));
            }
        }
        let peer = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|c| c.0.ip());
        Ok(ClientIp(peer))
    }
}

// shortcut: a known email does a little more database work than an unknown one, so response
// time can differ by a few ms; move the token + email into a background task if that matters.
pub(crate) async fn login(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    ApiJson(body): ApiJson<LoginRequest>,
) -> Result<StatusCode, ApiError> {
    let email = users::normalize_email(&body.email).map_err(ApiError::Validation)?;
    let pool = &state.pool;
    let by_email: i64 = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM login_requests WHERE email = $1 AND at > now() - interval '1 hour'"#,
        email
    )
    .fetch_one(pool)
    .await?;
    let by_ip: i64 = match ip {
        None => 0,
        Some(ip) => sqlx::query_scalar!(
            r#"SELECT count(*) AS "n!" FROM login_requests WHERE ip = $1 AND at > now() - interval '1 hour'"#,
            sqlx::types::ipnetwork::IpNetwork::from(ip)
        )
        .fetch_one(pool)
        .await?,
    };
    sqlx::query!(
        "INSERT INTO login_requests (email, ip) VALUES ($1, $2)",
        email,
        ip.map(sqlx::types::ipnetwork::IpNetwork::from)
    )
    .execute(pool)
    .await?;
    if by_email >= PER_EMAIL_PER_HOUR || by_ip >= PER_IP_PER_HOUR {
        tracing::warn!("login link limit reached");
        return Ok(StatusCode::ACCEPTED);
    }
    let Some(user) = users::find_by_email(pool, &email).await? else {
        return Ok(StatusCode::ACCEPTED);
    };
    let token = auth_tokens::generate(auth_tokens::LOGIN_PREFIX);
    sqlx::query!(
        "INSERT INTO login_tokens (token_hash, user_id, expires_at) VALUES ($1, $2, now() + make_interval(mins => $3))",
        hash_token(&token),
        user.id,
        LOGIN_TOKEN_MINUTES as i32
    )
    .execute(pool)
    .await?;
    let link = format!("{}/login/confirm?token={token}", state.config.web_base());
    if state.config.dev_log_links {
        tracing::info!(%link, "DEV_LOG_LINKS login link");
    }
    let text = format!(
        "Log in to Mailrust:\n\n{link}\n\nThe link works once and expires in {LOGIN_TOKEN_MINUTES} minutes.\nIf you did not ask for it, ignore this email.\n"
    );
    system_mail::queue(&state, &email, "Your Mailrust login link", text).await?;
    Ok(StatusCode::ACCEPTED)
}

pub(crate) async fn verify(
    State(state): State<AppState>,
    ApiJson(body): ApiJson<TokenRequest>,
) -> Result<Json<SessionResponse>, ApiError> {
    let mut tx = state.pool.begin().await?;
    let user_id = sqlx::query_scalar!(
        "UPDATE login_tokens SET used_at = now()
         WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now()
         RETURNING user_id",
        hash_token(body.token.trim())
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::BadToken)?;
    let s = sessions::create(&mut tx, user_id).await?;
    tx.commit().await?;
    Ok(Json(SessionResponse {
        session_token: s.token,
        expires_at: s.expires_at,
    }))
}
