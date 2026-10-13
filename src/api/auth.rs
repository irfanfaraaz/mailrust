use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use uuid::Uuid;

use super::AppState;
use crate::{
    accounts::AccountStatus,
    auth_tokens::SESSION_PREFIX,
    error::ApiError,
    keys::{self, KeyAccess, KeyContext},
    sessions::{self, SessionUser},
    users::{self, MemberRole},
};

pub const ACCOUNT_HEADER: &str = "x-mailrust-account";

#[derive(Debug, Clone, Copy)]
pub struct Member {
    pub user_id: Uuid,
    pub role: MemberRole,
}

/// Extractor: an API key, or a dashboard session plus `X-Mailrust-Account`.
/// For a session, `ctx.key_id` holds the session id (it keys the rate limiter).
pub struct Auth {
    pub ctx: KeyContext,
    pub member: Option<Member>,
}

fn bearer(parts: &Parts) -> Result<&str, ApiError> {
    parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
        .ok_or(ApiError::Unauthorized)
}

impl FromRequestParts<AppState> for Auth {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let token = bearer(parts)?;
        let auth = if token.starts_with(keys::TOKEN_PREFIX) {
            let ctx = keys::find_by_token(&state.pool, token)
                .await?
                .ok_or(ApiError::Unauthorized)?;
            Auth { ctx, member: None }
        } else if token.starts_with(SESSION_PREFIX) {
            let s = sessions::find(&state.pool, token)
                .await?
                .ok_or(ApiError::Unauthorized)?;
            let raw = parts
                .headers
                .get(ACCOUNT_HEADER)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| {
                    ApiError::Validation("X-Mailrust-Account is required with a session".into())
                })?;
            let account_id: Uuid = raw.trim().parse().map_err(|_| {
                ApiError::Validation("X-Mailrust-Account must be an account id".into())
            })?;
            // Not a member → 404, so account ids cannot be probed.
            let m = users::membership(&state.pool, s.user_id, account_id)
                .await?
                .ok_or(ApiError::NotFound)?;
            Auth {
                ctx: KeyContext {
                    key_id: s.session_id,
                    account_id,
                    access: KeyAccess::Full,
                    account_status: m.status,
                    paused_reason: m.paused_reason,
                },
                member: Some(Member {
                    user_id: s.user_id,
                    role: m.role,
                }),
            }
        } else {
            return Err(ApiError::Unauthorized);
        };
        if state.key_limiter.check_key(&auth.ctx.key_id).is_err() {
            return Err(ApiError::RateLimited);
        }
        Ok(auth)
    }
}

impl Auth {
    pub fn account_id(&self) -> Uuid {
        self.ctx.account_id
    }

    pub fn require_full(&self) -> Result<(), ApiError> {
        match self.ctx.access {
            KeyAccess::Full => Ok(()),
            KeyAccess::Sending => Err(ApiError::Forbidden),
        }
    }

    pub fn require_active(&self) -> Result<(), ApiError> {
        match self.ctx.account_status {
            AccountStatus::Active => Ok(()),
            AccountStatus::Paused => Err(ApiError::AccountPaused(
                self.ctx
                    .paused_reason
                    .clone()
                    .unwrap_or_else(|| "no reason given".into()),
            )),
        }
    }

    /// Dashboard-only routes: API keys get 403.
    pub fn require_session(&self) -> Result<&Member, ApiError> {
        self.member.as_ref().ok_or(ApiError::Forbidden)
    }

    pub fn require_owner(&self) -> Result<&Member, ApiError> {
        let m = self.require_session()?;
        if m.role == MemberRole::Owner {
            Ok(m)
        } else {
            Err(ApiError::Forbidden)
        }
    }
}

/// Extractor for routes that need a logged-in user but no account (`/me`, logout).
pub struct SessionAuth(pub SessionUser);

impl FromRequestParts<AppState> for SessionAuth {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let token = bearer(parts)?;
        if !token.starts_with(SESSION_PREFIX) {
            return Err(ApiError::Unauthorized);
        }
        sessions::find(&state.pool, token)
            .await?
            .map(SessionAuth)
            .ok_or(ApiError::Unauthorized)
    }
}
