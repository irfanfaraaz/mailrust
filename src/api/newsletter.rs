use axum::{Json, Router, extract::State, routing::get};
use serde::Deserialize;

use super::{AppState, auth::Auth, json::ApiJson};
use crate::{
    contacts,
    domains::{self, DomainStatus},
    error::ApiError,
    newsletter::{self, NewsletterSettings},
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/newsletter", get(show).patch(update))
}

/// Absent fields stay as they are; `""` clears a text field.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateNewsletterRequest {
    pub public_slug: Option<String>,
    pub newsletter_name: Option<String>,
    /// Must be on a verified domain of this account.
    pub newsletter_from: Option<String>,
    pub signup_enabled: Option<bool>,
    /// Shown at the bottom of every issue; sending needs it. `""` clears it.
    pub postal_address: Option<String>,
    /// Show the first-name field on the signup form.
    pub signup_ask_name: Option<bool>,
}

pub(crate) async fn show(
    State(state): State<AppState>,
    auth: Auth,
) -> Result<Json<NewsletterSettings>, ApiError> {
    auth.require_full()?;
    let n = newsletter::get(&state.pool, auth.account_id())
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(n.settings(&state.pool).await?))
}

pub(crate) async fn update(
    State(state): State<AppState>,
    auth: Auth,
    ApiJson(body): ApiJson<UpdateNewsletterRequest>,
) -> Result<Json<NewsletterSettings>, ApiError> {
    auth.require_full()?;
    let account_id = auth.account_id();
    let n = newsletter::get(&state.pool, account_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let v = ApiError::Validation;
    let slug = match body.public_slug.as_deref() {
        Some(s) => newsletter::normalize_slug(s).map_err(v)?,
        None => n.public_slug.clone(),
    };
    let name = match body.newsletter_name.as_deref() {
        Some(s) => contacts::clean_name(Some(s)).map_err(v)?,
        None => n.newsletter_name.clone(),
    };
    let from = match body.newsletter_from.as_deref() {
        Some(s) => {
            let from = newsletter::normalize_from(s).map_err(v)?;
            if let Some(f) = &from {
                let domain = newsletter::from_domain(f);
                let verified = domains::find_by_name(&state.pool, account_id, domain)
                    .await?
                    .is_some_and(|d| d.status == DomainStatus::Verified);
                if !verified {
                    return Err(ApiError::Unprocessable(
                        "from_not_verified",
                        format!("add and verify {domain} before sending from it"),
                    ));
                }
            }
            from
        }
        None => n.newsletter_from.clone(),
    };
    let enabled = body.signup_enabled.unwrap_or(n.signup_enabled);
    if enabled && slug.is_none() {
        return Err(ApiError::Validation(
            "choose a public address before turning signups on".into(),
        ));
    }
    let postal = match body.postal_address.as_deref() {
        Some(s) => newsletter::clean_postal(s).map_err(ApiError::Validation)?,
        None => n.postal_address.clone(),
    };
    let ask_name = body.signup_ask_name.unwrap_or(n.signup_ask_name);
    sqlx::query!(
        "UPDATE accounts SET public_slug = $2, newsletter_name = $3, newsletter_from = $4, signup_enabled = $5,
                postal_address = $6, signup_ask_name = $7 WHERE id = $1",
        account_id,
        slug,
        name,
        from,
        enabled,
        postal,
        ask_name
    )
    .execute(&state.pool)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => {
            ApiError::Conflict("slug_taken", "another newsletter already uses this public address".into())
        }
        _ => e.into(),
    })?;
    let n = newsletter::get(&state.pool, account_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(n.settings(&state.pool).await?))
}
