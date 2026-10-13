use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
};
use serde::Deserialize;
use uuid::Uuid;

use super::{
    AppState,
    auth::Auth,
    json::ApiJson,
    login::{SessionResponse, TokenRequest},
};
use crate::{
    accounts, auth_tokens,
    error::ApiError,
    keys::hash_token,
    sessions, system_mail,
    team::{self, INVITE_DAYS, InviteView, MemberView, TeamView},
    users::{self, MemberRole},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/team", get(list))
        .route("/team/invites", post(invite))
        .route("/team/invites/{id}", delete(cancel_invite))
        .route("/team/members/{user_id}", patch(set_role).delete(remove))
        .route("/invites/accept", post(accept))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct InviteRequest {
    pub email: String,
    pub role: MemberRole,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct RoleRequest {
    pub role: MemberRole,
}

pub(crate) async fn list(
    State(state): State<AppState>,
    auth: Auth,
) -> Result<Json<TeamView>, ApiError> {
    auth.require_session()?;
    Ok(Json(team::view(&state.pool, auth.account_id()).await?))
}

pub(crate) async fn invite(
    State(state): State<AppState>,
    auth: Auth,
    ApiJson(body): ApiJson<InviteRequest>,
) -> Result<(StatusCode, Json<InviteView>), ApiError> {
    let me = *auth.require_owner()?;
    let email = users::normalize_email(&body.email).map_err(ApiError::Validation)?;
    let account_id = auth.account_id();
    let already = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM account_members m JOIN users u ON u.id = m.user_id
                         WHERE m.account_id = $1 AND u.email = $2) AS "e!""#,
        account_id,
        email
    )
    .fetch_one(&state.pool)
    .await?;
    if already {
        return Err(ApiError::Conflict(
            "already_member",
            format!("{email} is already in this account"),
        ));
    }
    if let Some(problem) = system_mail::setup_problem(&state).await? {
        tracing::error!(%problem, "invite not created");
        return Err(ApiError::Conflict(
            "system_email_not_set_up",
            "Invite emails cannot be sent: system email is not set up on this server. Check the server log."
                .into(),
        ));
    }
    let account = accounts::get(&state.pool, account_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let token = auth_tokens::generate(auth_tokens::INVITE_PREFIX);
    let mut tx = state.pool.begin().await?;
    sqlx::query!(
        "UPDATE invites SET revoked_at = now()
         WHERE account_id = $1 AND email = $2 AND accepted_at IS NULL AND revoked_at IS NULL",
        account_id,
        email
    )
    .execute(&mut *tx)
    .await?;
    let view = sqlx::query_as!(
        InviteView,
        r#"INSERT INTO invites (id, account_id, email, role, token_hash, invited_by, expires_at)
           VALUES ($1, $2, $3, $4, $5, $6, now() + make_interval(days => $7))
           RETURNING id, email, role AS "role: MemberRole", created_at, expires_at"#,
        Uuid::now_v7(),
        account_id,
        email,
        body.role as MemberRole,
        hash_token(&token),
        me.user_id,
        INVITE_DAYS as i32
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    let link = format!("{}/invite?token={token}", state.config.web_base());
    if state.config.dev_log_links {
        tracing::info!(%link, "DEV_LOG_LINKS invite link");
    }
    let role = match body.role {
        MemberRole::Owner => "an owner",
        MemberRole::Member => "a member",
    };
    let text = format!(
        "You are invited to join {} on Mailrust as {role}.\n\nAccept the invite:\n\n{link}\n\nThe link works once and expires in {INVITE_DAYS} days.\n",
        account.name
    );
    system_mail::queue(
        &state,
        &email,
        &format!("Join {} on Mailrust", account.name),
        text,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(view)))
}

pub(crate) async fn cancel_invite(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    auth.require_owner()?;
    let r = sqlx::query!(
        "UPDATE invites SET revoked_at = now() WHERE id = $1 AND account_id = $2 AND accepted_at IS NULL AND revoked_at IS NULL",
        id,
        auth.account_id()
    )
    .execute(&state.pool)
    .await?;
    if r.rows_affected() == 1 {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}

pub(crate) async fn set_role(
    State(state): State<AppState>,
    auth: Auth,
    Path(user_id): Path<Uuid>,
    ApiJson(body): ApiJson<RoleRequest>,
) -> Result<Json<MemberView>, ApiError> {
    auth.require_owner()?;
    let mut tx = state.pool.begin().await?;
    if body.role == MemberRole::Member {
        team::guard_last_owner(&mut tx, auth.account_id(), user_id).await?;
    }
    let v = sqlx::query_as!(
        MemberView,
        r#"UPDATE account_members m SET role = $3 FROM users u
           WHERE m.account_id = $1 AND m.user_id = $2 AND u.id = m.user_id
           RETURNING m.user_id, u.email, m.role AS "role: MemberRole", m.created_at AS joined_at"#,
        auth.account_id(),
        user_id,
        body.role as MemberRole
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound)?;
    tx.commit().await?;
    Ok(Json(v))
}

/// Owners remove anyone; any member may remove themself (leave). Never the last owner.
pub(crate) async fn remove(
    State(state): State<AppState>,
    auth: Auth,
    Path(user_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let me = *auth.require_session()?;
    if me.user_id != user_id {
        auth.require_owner()?;
    }
    let mut tx = state.pool.begin().await?;
    team::guard_last_owner(&mut tx, auth.account_id(), user_id).await?;
    let r = sqlx::query!(
        "DELETE FROM account_members WHERE account_id = $1 AND user_id = $2",
        auth.account_id(),
        user_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    if r.rows_affected() == 1 {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}

pub(crate) async fn accept(
    State(state): State<AppState>,
    ApiJson(body): ApiJson<TokenRequest>,
) -> Result<Json<SessionResponse>, ApiError> {
    let mut tx = state.pool.begin().await?;
    let inv = sqlx::query!(
        r#"UPDATE invites SET accepted_at = now()
           WHERE token_hash = $1 AND accepted_at IS NULL AND revoked_at IS NULL AND expires_at > now()
           RETURNING account_id, email, role AS "role: MemberRole""#,
        hash_token(body.token.trim())
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::BadToken)?;
    let user = users::find_or_create(&mut tx, &inv.email).await?;
    users::add_member(&mut tx, inv.account_id, user.id, inv.role).await?;
    let s = sessions::create(&mut tx, user.id).await?;
    tx.commit().await?;
    Ok(Json(SessionResponse {
        session_token: s.token,
        expires_at: s.expires_at,
    }))
}
