use chrono::{DateTime, Duration, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::{auth_tokens, keys::hash_token};

pub const SESSION_DAYS: i64 = 30;

pub struct NewSession {
    pub id: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SessionUser {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub email: String,
}

pub async fn create(conn: &mut PgConnection, user_id: Uuid) -> sqlx::Result<NewSession> {
    let id = Uuid::now_v7();
    let token = auth_tokens::generate(auth_tokens::SESSION_PREFIX);
    let expires_at = Utc::now() + Duration::days(SESSION_DAYS);
    sqlx::query!(
        "INSERT INTO sessions (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
        id,
        user_id,
        hash_token(&token),
        expires_at
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "UPDATE users SET last_login_at = now() WHERE id = $1",
        user_id
    )
    .execute(&mut *conn)
    .await?;
    Ok(NewSession {
        id,
        token,
        expires_at,
    })
}

/// A live session, its expiry pushed out to 30 days (written at most once an hour).
pub async fn find(pool: &PgPool, token: &str) -> sqlx::Result<Option<SessionUser>> {
    let row = sqlx::query_as!(
        SessionUser,
        "SELECT s.id AS session_id, s.user_id, u.email
         FROM sessions s JOIN users u ON u.id = s.user_id
         WHERE s.token_hash = $1 AND s.expires_at > now()",
        hash_token(token)
    )
    .fetch_optional(pool)
    .await?;
    if let Some(s) = &row {
        sqlx::query!(
            "UPDATE sessions SET last_used_at = now(), expires_at = now() + make_interval(days => $2)
             WHERE id = $1 AND last_used_at < now() - interval '1 hour'",
            s.session_id,
            SESSION_DAYS as i32
        )
        .execute(pool)
        .await?;
    }
    Ok(row)
}

pub async fn delete(pool: &PgPool, session_id: Uuid) -> sqlx::Result<()> {
    sqlx::query!("DELETE FROM sessions WHERE id = $1", session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_all(pool: &PgPool, user_id: Uuid) -> sqlx::Result<()> {
    sqlx::query!("DELETE FROM sessions WHERE user_id = $1", user_id)
        .execute(pool)
        .await?;
    Ok(())
}
