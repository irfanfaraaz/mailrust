use base64::Engine;
use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::accounts::AccountStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize, utoipa::ToSchema, clap::ValueEnum)]
#[sqlx(type_name = "key_access", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum KeyAccess {
    Full,
    Sending,
}

pub const TOKEN_PREFIX: &str = "mr_live_";

pub struct NewKey {
    pub id: Uuid,
    pub token: String,
    pub prefix: String,
    pub access: KeyAccess,
}

#[derive(Debug, Clone)]
pub struct KeyContext {
    pub key_id: Uuid,
    pub account_id: Uuid,
    pub access: KeyAccess,
    pub account_status: AccountStatus,
    pub paused_reason: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct KeySummary {
    pub id: Uuid,
    pub name: String,
    pub prefix: String,
    pub access: KeyAccess,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

pub fn generate_token() -> String {
    let mut bytes = [0u8; 24];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    format!(
        "{TOKEN_PREFIX}{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    )
}

/// We store only this hash. A stolen database does not reveal usable keys.
pub fn hash_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

pub fn display_prefix(token: &str) -> String {
    token.chars().take(TOKEN_PREFIX.len() + 6).collect()
}

pub async fn create(pool: &PgPool, account_id: Uuid, name: &str, access: KeyAccess) -> sqlx::Result<NewKey> {
    let id = Uuid::now_v7();
    let token = generate_token();
    let prefix = display_prefix(&token);
    sqlx::query!(
        "INSERT INTO api_keys (id, account_id, name, prefix, key_hash, access) VALUES ($1, $2, $3, $4, $5, $6)",
        id,
        account_id,
        name,
        prefix,
        hash_token(&token),
        access as KeyAccess
    )
    .execute(pool)
    .await?;
    Ok(NewKey { id, token, prefix, access })
}

pub async fn find_by_token(pool: &PgPool, token: &str) -> sqlx::Result<Option<KeyContext>> {
    let row = sqlx::query_as!(
        KeyContext,
        r#"SELECT k.id AS key_id, k.account_id, k.access AS "access: KeyAccess",
                  a.status AS "account_status: AccountStatus", a.paused_reason
           FROM api_keys k JOIN accounts a ON a.id = k.account_id
           WHERE k.key_hash = $1 AND k.revoked_at IS NULL"#,
        hash_token(token)
    )
    .fetch_optional(pool)
    .await?;
    if let Some(ctx) = &row {
        // Write at most once a minute per key, not on every request.
        sqlx::query!(
            "UPDATE api_keys SET last_used_at = now()
             WHERE id = $1 AND (last_used_at IS NULL OR last_used_at < now() - interval '1 minute')",
            ctx.key_id
        )
        .execute(pool)
        .await?;
    }
    Ok(row)
}

pub async fn list(pool: &PgPool, account_id: Uuid) -> sqlx::Result<Vec<KeySummary>> {
    sqlx::query_as!(
        KeySummary,
        r#"SELECT id, name, prefix, access AS "access: KeyAccess", last_used_at, created_at
           FROM api_keys WHERE account_id = $1 AND revoked_at IS NULL ORDER BY created_at"#,
        account_id
    )
    .fetch_all(pool)
    .await
}

pub async fn revoke(pool: &PgPool, account_id: Option<Uuid>, key_id: Uuid) -> sqlx::Result<bool> {
    let r = sqlx::query!(
        "UPDATE api_keys SET revoked_at = now()
         WHERE id = $1 AND revoked_at IS NULL AND ($2::uuid IS NULL OR account_id = $2)",
        key_id,
        account_id
    )
    .execute(pool)
    .await?;
    Ok(r.rows_affected() == 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_unique_and_prefixed() {
        let a = generate_token();
        let b = generate_token();
        assert_ne!(a, b);
        assert!(a.starts_with(TOKEN_PREFIX));
        assert_eq!(a.len(), TOKEN_PREFIX.len() + 32);
        assert_eq!(display_prefix(&a).len(), 14);
    }

    #[test]
    fn hash_is_stable() {
        assert_eq!(hash_token("x"), hash_token("x"));
        assert_ne!(hash_token("x"), hash_token("y"));
    }
}
