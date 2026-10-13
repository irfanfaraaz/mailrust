use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::{error::ApiError, users::MemberRole};

pub const INVITE_DAYS: i64 = 7;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MemberView {
    pub user_id: Uuid,
    pub email: String,
    pub role: MemberRole,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct InviteView {
    pub id: Uuid,
    pub email: String,
    pub role: MemberRole,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TeamView {
    pub members: Vec<MemberView>,
    pub invites: Vec<InviteView>,
}

pub async fn view(pool: &PgPool, account_id: Uuid) -> sqlx::Result<TeamView> {
    let members = sqlx::query_as!(
        MemberView,
        r#"SELECT m.user_id, u.email, m.role AS "role: MemberRole", m.created_at AS joined_at
           FROM account_members m JOIN users u ON u.id = m.user_id
           WHERE m.account_id = $1 ORDER BY m.created_at"#,
        account_id
    )
    .fetch_all(pool)
    .await?;
    let invites = sqlx::query_as!(
        InviteView,
        r#"SELECT id, email, role AS "role: MemberRole", created_at, expires_at FROM invites
           WHERE account_id = $1 AND accepted_at IS NULL AND revoked_at IS NULL AND expires_at > now()
           ORDER BY created_at"#,
        account_id
    )
    .fetch_all(pool)
    .await?;
    Ok(TeamView { members, invites })
}

/// Locks the account's member rows, then refuses if `user_id` is its only owner.
/// The lock makes two owners stepping down at once safe: the second one waits and sees one owner.
pub async fn guard_last_owner(
    conn: &mut PgConnection,
    account_id: Uuid,
    user_id: Uuid,
) -> Result<(), ApiError> {
    let rows = sqlx::query!(
        r#"SELECT user_id, role AS "role: MemberRole" FROM account_members WHERE account_id = $1 FOR UPDATE"#,
        account_id
    )
    .fetch_all(&mut *conn)
    .await?;
    let owners: Vec<Uuid> = rows
        .iter()
        .filter(|r| r.role == MemberRole::Owner)
        .map(|r| r.user_id)
        .collect();
    if owners == [user_id] {
        return Err(ApiError::Conflict(
            "last_owner",
            "an account needs at least one owner".into(),
        ));
    }
    Ok(())
}
