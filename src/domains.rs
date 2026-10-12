use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, types::Json};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    accounts::{self, Account},
    api::AppState,
    background, dns,
    sender::{DnsRecord, DomainState, SenderError},
};

pub const VERIFY_WINDOW_HOURS: i64 = 72;
pub const VERIFY_EVERY: Duration = Duration::from_secs(600);

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, utoipa::ToSchema)]
#[sqlx(type_name = "domain_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DomainStatus {
    Pending,
    Verified,
    Failed,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Domain {
    pub id: Uuid,
    #[serde(skip)]
    pub account_id: Uuid,
    pub name: String,
    pub status: DomainStatus,
    #[schema(value_type = Vec<DnsRecord>)]
    pub dns_records: Json<Vec<DnsRecord>>,
    pub tracking_subdomain: Option<String>,
    pub tracking_verified: bool,
    pub open_tracking: bool,
    pub click_tracking: bool,
    #[serde(skip)]
    pub verify_until: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum AddDomainError {
    #[error("{0}")]
    Invalid(String),
    #[error("this domain is already added")]
    Exists,
    #[error(transparent)]
    Sender(#[from] SenderError),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

pub struct AddDomain {
    pub name: String,
    pub tracking_subdomain: Option<String>,
    pub open_tracking: bool,
    pub click_tracking: bool,
}

/// Lowercases, drops a trailing dot, and checks the domain looks real.
pub fn normalize_domain(input: &str) -> Result<String, String> {
    let d = input.trim().trim_end_matches('.').to_ascii_lowercase();
    if d.len() > 253 {
        return Err("domain is too long".into());
    }
    let labels: Vec<&str> = d.split('.').collect();
    if labels.len() < 2 {
        return Err(format!("{d} is not a full domain name"));
    }
    for l in &labels {
        let ok = !l.is_empty()
            && l.len() <= 63
            && !l.starts_with('-')
            && !l.ends_with('-')
            && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
        if !ok {
            return Err(format!("{d} is not a valid domain name"));
        }
    }
    Ok(d)
}

pub async fn get_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Domain>> {
    sqlx::query_as!(
        Domain,
        r#"SELECT id, account_id, name, status AS "status: DomainStatus",
                  dns_records AS "dns_records: Json<Vec<DnsRecord>>", tracking_subdomain, tracking_verified,
                  open_tracking, click_tracking, verify_until, created_at
           FROM domains WHERE id = $1"#,
        id
    )
    .fetch_optional(pool)
    .await
}

pub async fn get_for_account(
    pool: &PgPool,
    account_id: Uuid,
    id: Uuid,
) -> sqlx::Result<Option<Domain>> {
    Ok(get_by_id(pool, id)
        .await?
        .filter(|d| d.account_id == account_id))
}

pub async fn find_by_name(
    pool: &PgPool,
    account_id: Uuid,
    name: &str,
) -> sqlx::Result<Option<Domain>> {
    sqlx::query_as!(
        Domain,
        r#"SELECT id, account_id, name, status AS "status: DomainStatus",
                  dns_records AS "dns_records: Json<Vec<DnsRecord>>", tracking_subdomain, tracking_verified,
                  open_tracking, click_tracking, verify_until, created_at
           FROM domains WHERE account_id = $1 AND name = $2"#,
        account_id,
        name
    )
    .fetch_optional(pool)
    .await
}

pub async fn list(pool: &PgPool, account_id: Uuid) -> sqlx::Result<Vec<Domain>> {
    sqlx::query_as!(
        Domain,
        r#"SELECT id, account_id, name, status AS "status: DomainStatus",
                  dns_records AS "dns_records: Json<Vec<DnsRecord>>", tracking_subdomain, tracking_verified,
                  open_tracking, click_tracking, verify_until, created_at
           FROM domains WHERE account_id = $1 ORDER BY created_at"#,
        account_id
    )
    .fetch_all(pool)
    .await
}

pub async fn add(
    state: &AppState,
    account: &Account,
    input: AddDomain,
) -> Result<Domain, AddDomainError> {
    let name = normalize_domain(&input.name).map_err(AddDomainError::Invalid)?;
    let tracking = match input.tracking_subdomain.as_deref() {
        None => None,
        Some(t) => {
            let t = normalize_domain(t).map_err(AddDomainError::Invalid)?;
            let label = t.strip_suffix(&format!(".{name}")).unwrap_or_default();
            if label.is_empty() || label.contains('.') {
                return Err(AddDomainError::Invalid(format!(
                    "tracking subdomain must look like track.{name}"
                )));
            }
            Some(t)
        }
    };
    if find_by_name(&state.pool, account.id, &name)
        .await?
        .is_some()
    {
        return Err(AddDomainError::Exists);
    }

    let mut records = state.sender.create_domain(&account.tenant(), &name).await?;
    records.push(DnsRecord {
        record_type: "TXT".into(),
        name: format!("_dmarc.{name}"),
        value: "v=DMARC1; p=none;".into(),
        purpose: "DMARC (suggested; skip if you already have one)".into(),
    });
    if let Some(t) = &tracking {
        records.push(DnsRecord {
            record_type: "CNAME".into(),
            name: t.clone(),
            value: state.config.public_host(),
            purpose: "open and click tracking".into(),
        });
    }

    let id = Uuid::now_v7();
    let insert = sqlx::query!(
        "INSERT INTO domains (id, account_id, name, dns_records, tracking_subdomain, open_tracking, click_tracking, verify_until)
         VALUES ($1, $2, $3, $4, $5, $6, $7, now() + make_interval(hours => $8))",
        id,
        account.id,
        name,
        Json(&records) as _,
        tracking,
        input.open_tracking,
        input.click_tracking,
        VERIFY_WINDOW_HOURS as i32
    )
    .execute(&state.pool)
    .await;
    match insert {
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            // Either the same domain raced in, or another account already uses this tracking subdomain.
            return Err(AddDomainError::Exists);
        }
        other => other?,
    };

    let domain = get_by_id(&state.pool, id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?;
    // One check right away: the fake sender (and already-verified SES identities) verify instantly.
    match check(state, &domain).await {
        Ok(d) => Ok(d),
        Err(e) => {
            tracing::warn!(error = ?e, domain = %domain.name, "first domain check failed");
            Ok(domain)
        }
    }
}

pub async fn check(state: &AppState, domain: &Domain) -> anyhow::Result<Domain> {
    let account = accounts::get(&state.pool, domain.account_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("domain {} has no account", domain.id))?;
    let status = match state
        .sender
        .domain_state(&account.tenant(), &domain.name)
        .await?
    {
        DomainState::Verified => DomainStatus::Verified,
        DomainState::Failed => DomainStatus::Failed,
        DomainState::Pending if Utc::now() > domain.verify_until => DomainStatus::Failed,
        DomainState::Pending => DomainStatus::Pending,
    };
    let tracking_verified = match &domain.tracking_subdomain {
        Some(host) => dns::cname_points_to(host, &state.config.public_host()).await,
        None => false,
    };
    sqlx::query!(
        "UPDATE domains SET status = $2, tracking_verified = $3 WHERE id = $1",
        domain.id,
        status as DomainStatus,
        tracking_verified
    )
    .execute(&state.pool)
    .await?;
    Ok(get_by_id(&state.pool, domain.id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?)
}

pub async fn verify_loop(state: AppState, shutdown: CancellationToken) -> anyhow::Result<()> {
    background::every(VERIFY_EVERY, &shutdown, || async {
        let due = sqlx::query_scalar!(
            "SELECT id FROM domains
             WHERE status = 'pending'
                OR (tracking_subdomain IS NOT NULL AND NOT tracking_verified
                    AND created_at > now() - make_interval(hours => $1))",
            VERIFY_WINDOW_HOURS as i32
        )
        .fetch_all(&state.pool)
        .await;
        let ids = match due {
            Ok(ids) => ids,
            Err(e) => {
                tracing::error!(error = ?e, "domain verify query failed");
                return;
            }
        };
        for id in ids {
            if let Ok(Some(d)) = get_by_id(&state.pool, id).await {
                if let Err(e) = check(&state, &d).await {
                    tracing::warn!(error = ?e, domain = %d.name, "domain check failed");
                }
            }
        }
    })
    .await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::normalize_domain;

    #[test]
    fn normalizes_and_rejects() {
        assert_eq!(normalize_domain(" Shop.COM. ").unwrap(), "shop.com");
        assert!(normalize_domain("localhost").is_err());
        assert!(normalize_domain("-bad.com").is_err());
        assert!(normalize_domain("a..com").is_err());
        assert!(normalize_domain("sp ace.com").is_err());
        assert!(normalize_domain(&format!("{}.com", "a".repeat(64))).is_err());
    }
}
