use axum::{extract::FromRequestParts, http::{header::AUTHORIZATION, request::Parts}};
use uuid::Uuid;

use super::AppState;
use crate::{accounts::AccountStatus, error::ApiError, keys::{self, KeyAccess, KeyContext}};

/// Extractor: a valid, non-revoked API key. Also applies the per-key rate limit.
pub struct Auth(pub KeyContext);

impl FromRequestParts<AppState> for Auth {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(ApiError::Unauthorized)?;
        let token = header.strip_prefix("Bearer ").ok_or(ApiError::Unauthorized)?.trim();
        if !token.starts_with(keys::TOKEN_PREFIX) {
            return Err(ApiError::Unauthorized);
        }
        let ctx = keys::find_by_token(&state.pool, token).await?.ok_or(ApiError::Unauthorized)?;
        if state.key_limiter.check_key(&ctx.key_id).is_err() {
            return Err(ApiError::RateLimited);
        }
        Ok(Auth(ctx))
    }
}

impl Auth {
    pub fn account_id(&self) -> Uuid {
        self.0.account_id
    }

    pub fn require_full(&self) -> Result<(), ApiError> {
        match self.0.access {
            KeyAccess::Full => Ok(()),
            KeyAccess::Sending => Err(ApiError::Forbidden),
        }
    }

    pub fn require_active(&self) -> Result<(), ApiError> {
        match self.0.account_status {
            AccountStatus::Active => Ok(()),
            AccountStatus::Paused => Err(ApiError::AccountPaused(
                self.0.paused_reason.clone().unwrap_or_else(|| "no reason given".into()),
            )),
        }
    }
}
