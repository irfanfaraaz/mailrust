//! Routes for the public pages (/join, /confirm, /u). Only the Next.js app may call them.
use std::net::IpAddr;

use axum::{
    Json, Router,
    extract::{FromRequestParts, Path, State},
    http::{StatusCode, request::Parts},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    AppState,
    json::ApiJson,
    login::{ClientIp, TokenRequest, from_web},
};
use crate::{
    contacts,
    error::ApiError,
    keys::hash_token,
    newsletter::{self, JOIN_PER_IP_PER_HOUR},
    tokens, users,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/public/newsletter/{slug}", get(show_newsletter))
        .route("/public/confirm/{token}", get(confirm_info))
        .route("/public/unsubscribe/{token}", get(unsubscribe_info))
        .route("/public/unsubscribe", post(unsubscribe))
        .route("/public/resubscribe", post(resubscribe))
        .route("/public/join", post(join))
        .route("/public/confirm", post(confirm))
}

/// Extractor: the caller must be our Next.js app (else 404, so the routes look absent),
/// and each browser IP gets `PUBLIC_REQUESTS_PER_SECOND_PER_IP`.
pub struct WebCaller {
    pub ip: Option<IpAddr>,
}

impl FromRequestParts<AppState> for WebCaller {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        if !from_web(parts, state) {
            return Err(ApiError::NotFound);
        }
        if !parts.headers.contains_key("x-mailrust-client-ip") {
            static WARNED: std::sync::Once = std::sync::Once::new();
            WARNED.call_once(|| {
                tracing::warn!(
                    "a public request came without the reader's IP: every reader shares one rate limit; \
                     make the reverse proxy in front of web/ set X-Forwarded-For"
                );
            });
        }
        let ClientIp(ip) = ClientIp::from_request_parts(parts, state).await?;
        if let Some(ip) = ip
            && state.ip_limiter.check_key(&ip).is_err()
        {
            return Err(ApiError::RateLimited);
        }
        Ok(WebCaller { ip })
    }
}

/// Extractor: only the Next.js app (404 otherwise), with no per-IP limit. For image bytes:
/// mail providers fetch images through a few proxy IPs, which a per-IP limit would starve.
pub struct WebOnly;

impl FromRequestParts<AppState> for WebOnly {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        if from_web(parts, state) {
            Ok(WebOnly)
        } else {
            Err(ApiError::NotFound)
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct PublicNewsletter {
    pub name: String,
    /// False: show "This newsletter is not taking signups right now."
    pub open: bool,
    /// False: hide the first-name field.
    pub ask_name: bool,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct UnsubscribeView {
    /// None when the link is not valid any more; the page still says "You're unsubscribed".
    pub newsletter_name: Option<String>,
}

fn not_allowed() -> ApiError {
    ApiError::Conflict(
        "resubscribe_not_allowed",
        "this link cannot subscribe you again; use the signup form".into(),
    )
}

async fn name_of(state: &AppState, contact_id: Uuid) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar!(
        r#"SELECT COALESCE(a.newsletter_name, a.name) AS "name!" FROM contacts c JOIN accounts a ON a.id = c.account_id WHERE c.id = $1"#,
        contact_id
    )
    .fetch_optional(&state.pool)
    .await
}

pub(crate) async fn show_newsletter(
    State(state): State<AppState>,
    _: WebCaller,
    Path(slug): Path<String>,
) -> Result<Json<PublicNewsletter>, ApiError> {
    let slug = newsletter::normalize_slug(&slug)
        .ok()
        .flatten()
        .ok_or(ApiError::NotFound)?;
    let n = newsletter::by_slug(&state.pool, &slug)
        .await?
        .ok_or(ApiError::NotFound)?;
    let open = n.closed_reason(&state.pool).await?.is_none();
    Ok(Json(PublicNewsletter {
        name: n.display_name().to_string(),
        open,
        ask_name: n.signup_ask_name,
    }))
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ConfirmInfo {
    /// None when the link is unknown; a used or expired link still names the newsletter.
    pub newsletter_name: Option<String>,
    /// The signup form's slug, for a "sign up again" link.
    pub signup_slug: Option<String>,
}

/// Read-only: names the newsletter on the confirm page before the reader presses Confirm.
pub(crate) async fn confirm_info(
    State(state): State<AppState>,
    _: WebCaller,
    Path(token): Path<String>,
) -> Result<Json<ConfirmInfo>, ApiError> {
    let r = sqlx::query!(
        r#"SELECT COALESCE(a.newsletter_name, a.name) AS "name!", a.public_slug
           FROM contact_confirmations cc JOIN contacts c ON c.id = cc.contact_id JOIN accounts a ON a.id = c.account_id
           WHERE cc.token_hash = $1"#,
        hash_token(token.trim())
    )
    .fetch_optional(&state.pool)
    .await?;
    Ok(Json(ConfirmInfo {
        newsletter_name: r.as_ref().map(|r| r.name.clone()),
        signup_slug: r.and_then(|r| r.public_slug),
    }))
}

pub(crate) async fn unsubscribe_info(
    State(state): State<AppState>,
    _: WebCaller,
    Path(token): Path<String>,
) -> Result<Json<UnsubscribeView>, ApiError> {
    let newsletter_name = match tokens::verify_unsubscribe(&state.config.tracking_secret, &token) {
        Some(id) => name_of(&state, id).await?,
        None => None,
    };
    Ok(Json(UnsubscribeView { newsletter_name }))
}

/// Always 200: an invalid token or a deleted contact gets the same answer with no name.
pub(crate) async fn unsubscribe(
    State(state): State<AppState>,
    _: WebCaller,
    ApiJson(body): ApiJson<TokenRequest>,
) -> Result<Json<UnsubscribeView>, ApiError> {
    let Some((id, campaign)) =
        tokens::verify_unsubscribe_any(&state.config.tracking_secret, &body.token)
    else {
        return Ok(Json(UnsubscribeView {
            newsletter_name: None,
        }));
    };
    let r = sqlx::query!(
        "UPDATE contacts SET status = 'unsubscribed', unsubscribed_at = now(), unsubscribed_via_link = true, updated_at = now()
         WHERE id = $1 AND status <> 'unsubscribed'",
        id
    )
    .execute(&state.pool)
    .await?;
    if r.rows_affected() == 1 {
        tracing::info!(contact_id = %id, "unsubscribed by link");
        if let Some(campaign_id) = campaign {
            record_unsubscribe(&state, id, campaign_id).await?;
        }
    }
    Ok(Json(UnsubscribeView {
        newsletter_name: name_of(&state, id).await?,
    }))
}

/// The report counts it for the issue the link came from (only when the click changed the contact).
async fn record_unsubscribe(
    state: &AppState,
    contact_id: Uuid,
    campaign_id: Uuid,
) -> anyhow::Result<()> {
    let Some(e) = sqlx::query!(
        "SELECT id, account_id FROM emails WHERE campaign_id = $1 AND contact_id = $2",
        campaign_id,
        contact_id
    )
    .fetch_optional(&state.pool)
    .await?
    else {
        return Ok(());
    };
    let mut conn = state.pool.acquire().await?;
    crate::email_events::record(
        &mut conn,
        crate::email_events::NewEvent {
            account_id: e.account_id,
            email_id: e.id,
            kind: crate::email_events::EventKind::Unsubscribed,
            recipient: None,
            data: serde_json::json!({}),
            likely_bot: false,
        },
    )
    .await?;
    Ok(())
}

/// Works only for a contact that left through this same link and had confirmed once
/// (a pending signup that unsubscribes must still confirm by email).
pub(crate) async fn resubscribe(
    State(state): State<AppState>,
    _: WebCaller,
    ApiJson(body): ApiJson<TokenRequest>,
) -> Result<StatusCode, ApiError> {
    let id = tokens::verify_unsubscribe(&state.config.tracking_secret, &body.token)
        .ok_or_else(not_allowed)?;
    let r = sqlx::query!(
        "UPDATE contacts SET status = 'subscribed', subscribed_at = now(), unsubscribed_at = NULL,
                unsubscribed_via_link = false, updated_at = now()
         WHERE id = $1 AND status = 'unsubscribed' AND unsubscribed_via_link AND subscribed_at IS NOT NULL",
        id
    )
    .execute(&state.pool)
    .await?;
    if r.rows_affected() == 1 {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_allowed())
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct PublicJoinRequest {
    pub slug: String,
    pub email: String,
    pub first_name: Option<String>,
    /// Honeypot (the form's `hp_f3a1` input): people never fill it; a filled one gets the normal answer and nothing happens.
    #[serde(default)]
    pub website: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ConfirmView {
    pub newsletter_name: String,
}

/// Same answer (202, no body) whatever the address's state; the work happens after the response.
pub(crate) async fn join(
    State(state): State<AppState>,
    WebCaller { ip }: WebCaller,
    ApiJson(body): ApiJson<PublicJoinRequest>,
) -> Result<StatusCode, ApiError> {
    let email = users::normalize_email(&body.email).map_err(ApiError::Validation)?;
    let first_name =
        contacts::clean_name(body.first_name.as_deref()).map_err(ApiError::Validation)?;
    let slug = newsletter::normalize_slug(&body.slug)
        .ok()
        .flatten()
        .ok_or(ApiError::NotFound)?;
    let n = newsletter::by_slug(&state.pool, &slug)
        .await?
        .ok_or(ApiError::NotFound)?;
    if n.closed_reason(&state.pool).await?.is_some() {
        return Err(ApiError::Conflict(
            "signup_closed",
            "this newsletter is not taking signups right now".into(),
        ));
    }
    if !body.website.trim().is_empty() {
        tracing::info!(account_id = %n.account_id, "join honeypot filled");
        return Ok(StatusCode::ACCEPTED);
    }
    let ipnet = ip.map(sqlx::types::ipnetwork::IpNetwork::from);
    let by_ip: i64 = match ipnet {
        None => 0,
        Some(ip) => sqlx::query_scalar!(
            r#"SELECT count(*) AS "n!" FROM join_requests WHERE ip = $1 AND created_at > now() - interval '1 hour'"#,
            ip
        )
        .fetch_one(&state.pool)
        .await?,
    };
    sqlx::query!(
        "INSERT INTO join_requests (ip, email) VALUES ($1, $2)",
        ipnet,
        email
    )
    .execute(&state.pool)
    .await?;
    if by_ip >= JOIN_PER_IP_PER_HOUR {
        tracing::warn!(account_id = %n.account_id, "join limit reached");
        return Ok(StatusCode::ACCEPTED);
    }
    // After the response, so a known address takes no longer to answer than an unknown one.
    tokio::spawn(async move {
        if let Err(e) = newsletter::process_join(&state, &n, &email, first_name).await {
            tracing::error!(error = ?e, account_id = %n.account_id, "join failed");
        }
    });
    Ok(StatusCode::ACCEPTED)
}

pub(crate) async fn confirm(
    State(state): State<AppState>,
    _: WebCaller,
    ApiJson(body): ApiJson<TokenRequest>,
) -> Result<Json<ConfirmView>, ApiError> {
    let mut tx = state.pool.begin().await?;
    let contact_id = sqlx::query_scalar!(
        "UPDATE contact_confirmations SET used_at = now()
         WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now()
         RETURNING contact_id",
        hash_token(body.token.trim())
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::BadToken)?;
    // Only a pending contact: a link left over from before an unsubscribe must not undo it.
    let r = sqlx::query!(
        "UPDATE contacts SET status = 'subscribed', subscribed_at = now(), unsubscribed_at = NULL,
                unsubscribed_via_link = false, updated_at = now()
         WHERE id = $1 AND status = 'pending'",
        contact_id
    )
    .execute(&mut *tx)
    .await?;
    if r.rows_affected() == 0 {
        return Err(ApiError::BadToken); // the transaction drops: the token stays unused
    }
    tx.commit().await?;
    tracing::info!(%contact_id, "subscription confirmed");
    let newsletter_name = name_of(&state, contact_id)
        .await?
        .ok_or(ApiError::BadToken)?;
    Ok(Json(ConfirmView { newsletter_name }))
}
