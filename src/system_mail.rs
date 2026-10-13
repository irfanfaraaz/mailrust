//! Login and invite emails, sent through Mailrust's own queue.
use std::collections::BTreeMap;

use chrono::Utc;
use uuid::Uuid;

use crate::{
    api::AppState,
    domains::{self, DomainStatus},
    emails::{self, Priority, SendEmailRequest, UrgentCap},
};

/// Why system emails cannot be sent right now, if anything is wrong.
pub async fn setup_problem(state: &AppState) -> anyhow::Result<Option<String>> {
    let c = &state.config;
    let (Some(account_id), Some(from)) = (c.system_account_id, c.system_from.as_deref()) else {
        return Ok(Some(
            "SYSTEM_ACCOUNT_ID and SYSTEM_FROM are not set; login and invite emails cannot be sent"
                .into(),
        ));
    };
    let email = match emails::validate(
        request(from, "check@example.com", "x", "x".into()),
        Utc::now(),
    ) {
        Ok(e) => e,
        Err(e) => return Ok(Some(format!("SYSTEM_FROM is not usable: {e}"))),
    };
    match domains::find_by_name(&state.pool, account_id, &email.from_domain).await? {
        None => Ok(Some(format!(
            "{} is not added to the system account",
            email.from_domain
        ))),
        Some(d) if d.status != DomainStatus::Verified => {
            Ok(Some(format!("{} is not verified yet", email.from_domain)))
        }
        Some(_) => Ok(None),
    }
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
    if let Some(problem) = setup_problem(state).await? {
        tracing::error!(%problem, "system email not sent");
        return Ok(None);
    }
    let c = &state.config;
    let (Some(account_id), Some(from)) = (c.system_account_id, c.system_from.as_deref()) else {
        return Ok(None);
    };
    let email = emails::validate(request(from, to, subject, text), Utc::now())
        .map_err(|e| anyhow::anyhow!("system email invalid: {e}"))?;
    let domain = domains::find_by_name(&state.pool, account_id, &email.from_domain)
        .await?
        .ok_or_else(|| anyhow::anyhow!("system domain vanished"))?;
    let mut tx = state.pool.begin().await?;
    let (id, _) = emails::insert(
        &mut tx,
        account_id,
        domain.id,
        &email,
        None,
        &mut UrgentCap::default(),
    )
    .await?;
    tx.commit().await?;
    Ok(Some(id))
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
