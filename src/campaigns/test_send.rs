//! Test emails: one-off copies to members of the account. Stored like any email (HTML and text),
//! tagged `campaign_test`, with no List-Unsubscribe headers; they never count in the report.
use std::collections::BTreeMap;

use chrono::Utc;
use sqlx::types::Json;

use super::{Campaign, checks};
use crate::{
    api::AppState,
    contacts::Fields,
    emails::{self, Priority, SendEmailRequest, UrgentCap},
    error::ApiError,
    render::{blocks::MergeData, campaign},
    users,
};

pub const MAX_TEST_TO: usize = 5;
pub const TEST_PER_HOUR: i64 = 20;
pub const TEST_TAG: &str = "campaign_test";
/// A test needs these to pass; the audience and the postal address can wait for the real send.
const NEEDED: [&str; 5] = [
    "from_address",
    "subject",
    "bad_blocks",
    "merge_tag_fallback",
    "image_not_found",
];

pub async fn send(state: &AppState, c: &Campaign, to: &[String]) -> Result<usize, ApiError> {
    if to.is_empty() || to.len() > MAX_TEST_TO {
        return Err(ApiError::Validation(format!(
            "send a test to 1-{MAX_TEST_TO} addresses"
        )));
    }
    let mut addrs: Vec<String> = Vec::new();
    for a in to {
        let e = users::normalize_email(a).map_err(ApiError::Validation)?;
        if !addrs.contains(&e) {
            addrs.push(e);
        }
    }
    for a in &addrs {
        let member = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM users u JOIN account_members m ON m.user_id = u.id
                              WHERE u.email = $1 AND m.account_id = $2) AS "m!""#,
            a,
            c.account_id
        )
        .fetch_one(&state.pool)
        .await?;
        if !member {
            return Err(ApiError::Unprocessable(
                "not_a_member",
                "test emails go only to members of this account".into(),
            ));
        }
    }
    let report = checks::run(state, c).await?;
    if let Some(f) = report.first_failed(&NEEDED) {
        return Err(ApiError::Unprocessable(f.code, f.message.clone()));
    }
    let (Some(doc), Some(sender)) = (&report.doc, &report.sender) else {
        return Err(ApiError::Internal(anyhow::anyhow!(
            "checks passed without a document or sender"
        )));
    };

    let mut tx = state.pool.begin().await?;
    // One test request per account at a time, so two cannot both slip under the hourly limit.
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtextextended($1::text, 1))",
        format!("campaign_test:{}", c.account_id)
    )
    .execute(&mut *tx)
    .await?;
    let used = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM emails WHERE account_id = $1 AND tags ? 'campaign_test' AND created_at > now() - interval '1 hour'"#,
        c.account_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if used + addrs.len() as i64 > TEST_PER_HOUR {
        return Err(ApiError::RateLimited);
    }
    let issue = campaign::Issue {
        subject: &c.subject,
        preview_text: c.preview_text.as_deref(),
        newsletter_name: &report.newsletter_name,
        postal_address: &report.postal_address,
        doc,
    };
    for a in &addrs {
        let row = sqlx::query!(
            r#"SELECT first_name, last_name, fields AS "fields: Json<Fields>" FROM contacts WHERE account_id = $1 AND email = $2"#,
            c.account_id,
            a
        )
        .fetch_optional(&mut *tx)
        .await?;
        let data = row.map_or_else(MergeData::default, |r| MergeData {
            first_name: r.first_name,
            last_name: r.last_name,
            email: Some(a.clone()),
            fields: r.fields.0,
        });
        let parts = campaign::email(&issue, &data, &state.config.web_base(), None);
        let req = SendEmailRequest {
            from: sender.from.clone(),
            to: vec![a.clone()],
            cc: vec![],
            bcc: vec![],
            reply_to: vec![],
            subject: format!("[Test] {}", c.subject),
            html: Some(parts.html),
            text: Some(parts.text),
            headers: BTreeMap::new(),
            tags: BTreeMap::from([(TEST_TAG.to_string(), c.id.to_string())]),
            attachments: vec![],
            scheduled_at: None,
            priority: Priority::Normal,
        };
        let email = emails::validate(req, Utc::now())?;
        emails::insert(
            &mut tx,
            c.account_id,
            sender.domain_id,
            &email,
            None,
            &mut UrgentCap::default(),
        )
        .await?;
    }
    tx.commit().await?;
    tracing::info!(campaign_id = %c.id, count = addrs.len(), "test emails queued");
    Ok(addrs.len())
}
