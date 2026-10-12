use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::sender::{Sender, TenantRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, utoipa::ToSchema)]
#[sqlx(type_name = "account_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    Active,
    Paused,
}

#[derive(Debug, Clone, Serialize)]
pub struct Account {
    pub id: Uuid,
    pub name: String,
    pub status: AccountStatus,
    pub paused_reason: Option<String>,
    pub ses_tenant_name: String,
    pub ses_synced_at: Option<DateTime<Utc>>,
    pub send_rate_per_sec: i32,
    pub created_at: DateTime<Utc>,
}

impl Account {
    pub fn tenant(&self) -> TenantRef {
        TenantRef { account_id: self.id, tenant_name: self.ses_tenant_name.clone() }
    }
}

/// SES tenant names allow letters, digits, '-' and '_' up to 64 chars; this is 41.
pub fn tenant_name_for(id: Uuid) -> String {
    format!("mailrust-{}", id.simple())
}

pub async fn create(pool: &PgPool, name: &str) -> sqlx::Result<Account> {
    let id = Uuid::now_v7();
    sqlx::query_as!(
        Account,
        r#"INSERT INTO accounts (id, name, ses_tenant_name) VALUES ($1, $2, $3)
           RETURNING id, name, status AS "status: AccountStatus", paused_reason, ses_tenant_name,
                     ses_synced_at, send_rate_per_sec, created_at"#,
        id,
        name,
        tenant_name_for(id)
    )
    .fetch_one(pool)
    .await
}

pub async fn get(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Account>> {
    sqlx::query_as!(
        Account,
        r#"SELECT id, name, status AS "status: AccountStatus", paused_reason, ses_tenant_name,
                  ses_synced_at, send_rate_per_sec, created_at
           FROM accounts WHERE id = $1"#,
        id
    )
    .fetch_optional(pool)
    .await
}

pub async fn list(pool: &PgPool) -> sqlx::Result<Vec<Account>> {
    sqlx::query_as!(
        Account,
        r#"SELECT id, name, status AS "status: AccountStatus", paused_reason, ses_tenant_name,
                  ses_synced_at, send_rate_per_sec, created_at
           FROM accounts ORDER BY created_at"#
    )
    .fetch_all(pool)
    .await
}

pub async fn pause(pool: &PgPool, id: Uuid, reason: &str) -> sqlx::Result<bool> {
    let r = sqlx::query!(
        "UPDATE accounts SET status = 'paused', paused_reason = $2, paused_at = now()
         WHERE id = $1 AND status = 'active'",
        id,
        reason
    )
    .execute(pool)
    .await?;
    Ok(r.rows_affected() == 1)
}

pub async fn resume(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
    let r = sqlx::query!(
        "UPDATE accounts SET status = 'active', paused_reason = NULL, paused_at = NULL
         WHERE id = $1 AND status = 'paused'",
        id
    )
    .execute(pool)
    .await?;
    Ok(r.rows_affected() == 1)
}

pub async fn provision(pool: &PgPool, sender: &dyn Sender, account: &Account) -> anyhow::Result<()> {
    sender.provision_account(&account.tenant()).await?;
    sqlx::query!("UPDATE accounts SET ses_synced_at = now() WHERE id = $1", account.id)
        .execute(pool)
        .await?;
    Ok(())
}
