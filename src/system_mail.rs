//! Login, invite and newsletter confirm emails, sent through Mailrust's own queue.
use std::collections::BTreeMap;

use chrono::Utc;
use uuid::Uuid;

use crate::{
    accounts::{self, AccountStatus},
    api::AppState,
    domains::{self, Domain, DomainStatus},
    emails::{self, Priority, SendEmailRequest, UrgentCap, ValidEmail},
};

/// Tag key on every system email. Customers cannot set it, and the API never shows the body of an
/// email that has it: login and invite emails carry one-time links.
pub const SYSTEM_TAG: &str = "mailrust_system";

const PROBE_TO: &str = "check@example.com";

/// Why system emails cannot be sent right now, if anything is wrong.
pub async fn setup_problem(state: &AppState) -> anyhow::Result<Option<String>> {
    Ok(check(state, PROBE_TO, "x", "x".into()).await?.err())
}

/// The system account, the email to queue and the domain it sends from, or why it cannot be sent.
async fn check(
    state: &AppState,
    to: &str,
    subject: &str,
    text: String,
) -> anyhow::Result<Result<(Uuid, ValidEmail, Domain), String>> {
    let c = &state.config;
    let (Some(account_id), Some(from)) = (c.system_account_id, c.system_from.as_deref()) else {
        return Ok(Err(
            "SYSTEM_ACCOUNT_ID and SYSTEM_FROM are not set; login and invite emails cannot be sent"
                .into(),
        ));
    };
    let mut email = match emails::validate(request(from, to, subject, text), Utc::now()) {
        Ok(e) => e,
        // Blame SYSTEM_FROM only if it fails on its own; a bad recipient is the caller's error.
        Err(e) => match emails::validate(request(from, PROBE_TO, "x", "x".into()), Utc::now()) {
            Err(from_err) => return Ok(Err(format!("SYSTEM_FROM is not usable: {from_err}"))),
            Ok(_) => anyhow::bail!("system email invalid: {e}"),
        },
    };
    email.tags.insert(SYSTEM_TAG.into(), "true".into());
    if let Some(a) = accounts::get(&state.pool, account_id).await?
        && a.status == AccountStatus::Paused
    {
        return Ok(Err(format!(
            "the system account is paused ({}); resume it to send login and invite emails",
            a.paused_reason.as_deref().unwrap_or("no reason given")
        )));
    }
    Ok(
        match domains::find_by_name(&state.pool, account_id, &email.from_domain).await? {
            None => Err(format!(
                "{} is not added to the system account",
                email.from_domain
            )),
            Some(d) if d.status != DomainStatus::Verified => {
                Err(format!("{} is not verified yet", email.from_domain))
            }
            Some(d) => Ok((account_id, email, d)),
        },
    )
}

/// Queues a text-only, urgent email from SYSTEM_FROM. Text only: the worker rewrites links
/// only in HTML, so a one-time link never goes through click tracking.
/// Returns None (and logs why) when system email is not set up.
pub async fn queue(
    state: &AppState,
    to: &str,
    subject: &str,
    text: String,
) -> anyhow::Result<Option<Uuid>> {
    let (account_id, email, domain) = match check(state, to, subject, text).await? {
        Ok(found) => found,
        Err(problem) => {
            tracing::error!(%problem, "system email not sent");
            return Ok(None);
        }
    };
    Ok(Some(insert_one(state, account_id, domain.id, &email).await?))
}

/// Like `queue`, but from `from` through `account_id` itself (newsletter confirm emails).
/// None (logged, without the address) when the account cannot send from that domain right now.
pub async fn queue_from(
    state: &AppState,
    account_id: Uuid,
    from: &str,
    to: &str,
    subject: &str,
    text: String,
) -> anyhow::Result<Option<Uuid>> {
    let mut email = emails::validate(request(from, to, subject, text), Utc::now())
        .map_err(|e| anyhow::anyhow!("account email invalid: {e}"))?;
    email.tags.insert(SYSTEM_TAG.into(), "true".into());
    match domains::find_by_name(&state.pool, account_id, &email.from_domain).await? {
        Some(d) if d.status == DomainStatus::Verified => {
            Ok(Some(insert_one(state, account_id, d.id, &email).await?))
        }
        _ => {
            tracing::error!(%account_id, domain = %email.from_domain, "from domain not verified; email not sent");
            Ok(None)
        }
    }
}

async fn insert_one(
    state: &AppState,
    account_id: Uuid,
    domain_id: Uuid,
    email: &ValidEmail,
) -> anyhow::Result<Uuid> {
    let mut tx = state.pool.begin().await?;
    let (id, _) = emails::insert(&mut tx, account_id, domain_id, email, None, &mut UrgentCap::default()).await?;
    tx.commit().await?;
    Ok(id)
}

fn request(from: &str, to: &str, subject: &str, text: String) -> SendEmailRequest {
    SendEmailRequest {
        from: from.to_string(),
        to: vec![to.to_string()],
        cc: vec![],
        bcc: vec![],
        reply_to: vec![],
        subject: subject.to_string(),
        html: None,
        text: Some(text),
        headers: BTreeMap::new(),
        tags: BTreeMap::from([("category".to_string(), "system".to_string())]),
        attachments: vec![],
        scheduled_at: None,
        priority: Priority::Urgent,
    }
}
