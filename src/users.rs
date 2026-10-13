use lettre::Address;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::accounts::AccountStatus;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    sqlx::Type,
    Serialize,
    Deserialize,
    utoipa::ToSchema,
    clap::ValueEnum,
)]
#[sqlx(type_name = "member_role", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MemberRole {
    // Declaration order is the rank: Owner > Member.
    Member,
    Owner,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct User {
    pub id: Uuid,
    pub email: String,
}

pub fn normalize_email(input: &str) -> Result<String, String> {
    let e = input.trim().to_ascii_lowercase();
    e.parse::<Address>()
        .map(|_| e.clone())
        .map_err(|_| format!("{} is not a valid email address", input.trim()))
}

pub async fn find_or_create(conn: &mut PgConnection, email: &str) -> sqlx::Result<User> {
    sqlx::query_as!(
        User,
        "INSERT INTO users (id, email) VALUES ($1, $2)
         ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email
         RETURNING id, email",
        Uuid::now_v7(),
        email
    )
    .fetch_one(conn)
    .await
}

pub async fn find_by_email(pool: &PgPool, email: &str) -> sqlx::Result<Option<User>> {
    sqlx::query_as!(User, "SELECT id, email FROM users WHERE email = $1", email)
        .fetch_optional(pool)
        .await
}

/// Adds the user to the account, or keeps the higher of the old and new role.
pub async fn add_member(
    conn: &mut PgConnection,
    account_id: Uuid,
    user_id: Uuid,
    role: MemberRole,
) -> sqlx::Result<MemberRole> {
    sqlx::query_scalar!(
        r#"INSERT INTO account_members (account_id, user_id, role) VALUES ($1, $2, $3)
           ON CONFLICT (account_id, user_id) DO UPDATE
             SET role = CASE WHEN account_members.role = 'owner' THEN 'owner'::member_role ELSE EXCLUDED.role END
           RETURNING role AS "role: MemberRole""#,
        account_id,
        user_id,
        role as MemberRole
    )
    .fetch_one(conn)
    .await
}

/// CLI path: creates the user and membership directly, no email. Safe to run again.
pub async fn invite_direct(
    pool: &PgPool,
    email: &str,
    account_id: Uuid,
    role: MemberRole,
) -> anyhow::Result<(User, MemberRole)> {
    let email = normalize_email(email).map_err(anyhow::Error::msg)?;
    let mut tx = pool.begin().await?;
    let user = find_or_create(&mut tx, &email).await?;
    let role = add_member(&mut tx, account_id, user.id, role).await?;
    tx.commit().await?;
    Ok((user, role))
}

pub struct Membership {
    pub role: MemberRole,
    pub status: AccountStatus,
    pub paused_reason: Option<String>,
}

pub async fn membership(
    pool: &PgPool,
    user_id: Uuid,
    account_id: Uuid,
) -> sqlx::Result<Option<Membership>> {
    sqlx::query_as!(
        Membership,
        r#"SELECT m.role AS "role: MemberRole", a.status AS "status: AccountStatus", a.paused_reason
           FROM account_members m JOIN accounts a ON a.id = m.account_id
           WHERE m.user_id = $1 AND m.account_id = $2"#,
        user_id,
        account_id
    )
    .fetch_optional(pool)
    .await
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MyAccount {
    pub id: Uuid,
    pub name: String,
    pub role: MemberRole,
    pub status: AccountStatus,
    pub paused_reason: Option<String>,
}

pub async fn accounts_of(pool: &PgPool, user_id: Uuid) -> sqlx::Result<Vec<MyAccount>> {
    sqlx::query_as!(
        MyAccount,
        r#"SELECT a.id, a.name, m.role AS "role: MemberRole", a.status AS "status: AccountStatus", a.paused_reason
           FROM account_members m JOIN accounts a ON a.id = m.account_id
           WHERE m.user_id = $1 ORDER BY a.name, a.id"#,
        user_id
    )
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emails_are_trimmed_and_lower_cased() {
        assert_eq!(
            normalize_email("  Ana@Example.COM ").unwrap(),
            "ana@example.com"
        );
        assert!(normalize_email("not an email").is_err());
        assert!(normalize_email("").is_err());
    }

    #[test]
    fn owner_outranks_member() {
        assert!(MemberRole::Owner > MemberRole::Member);
    }
}
