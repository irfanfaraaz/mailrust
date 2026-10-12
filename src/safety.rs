use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::render::bare_address;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize, utoipa::ToSchema)]
#[sqlx(type_name = "suppression_reason", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SuppressionReason {
    Bounce,
    Complaint,
    Manual,
}

/// Which of these addresses are on the account's block list? Returns bare lowercase addresses.
pub async fn suppressed_among(pool: &PgPool, account_id: Uuid, addresses: &[String]) -> sqlx::Result<HashSet<String>> {
    let bare: Vec<String> = addresses.iter().map(|a| bare_address(a)).collect();
    let rows = sqlx::query_scalar!(
        "SELECT address FROM suppressions WHERE account_id = $1 AND address = ANY($2)",
        account_id,
        &bare[..]
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

pub async fn add_suppression(
    conn: &mut PgConnection,
    account_id: Uuid,
    address: &str,
    reason: SuppressionReason,
    source_email_id: Option<Uuid>,
) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO suppressions (account_id, address, reason, source_email_id) VALUES ($1, $2, $3, $4)
         ON CONFLICT (account_id, address) DO NOTHING",
        account_id,
        bare_address(address),
        reason as SuppressionReason,
        source_email_id
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}
