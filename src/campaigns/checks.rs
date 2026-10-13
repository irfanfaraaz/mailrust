//! The hard checks before a send (spec §6.1), the reader count, and the from address.
use lettre::message::Mailbox;
use serde::Serialize;
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use super::Campaign;
use crate::{
    accounts::{self, AccountStatus},
    api::AppState,
    domains::{self, DomainStatus},
    error::ApiError,
    newsletter,
    render::{
        blocks::{self, Doc},
        campaign::{self, Issue, MAX_EMAIL_HTML},
    },
};

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Check {
    #[schema(value_type = String)]
    pub code: &'static str,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Audience {
    pub readers: i64,
    pub missing_first_name: i64,
}

/// Subscribed contacts with any include tag (or all when none), none of the skip tags, not suppressed.
pub async fn audience<'e>(
    ex: impl PgExecutor<'e>,
    account_id: Uuid,
    include: &[Uuid],
    exclude: &[Uuid],
) -> sqlx::Result<Audience> {
    let r = sqlx::query!(
        r#"SELECT count(*) AS "readers!",
                  count(*) FILTER (WHERE COALESCE(btrim(c.first_name), '') = '') AS "missing!"
           FROM contacts c
           WHERE c.account_id = $1 AND c.status = 'subscribed'
             AND (cardinality($2::uuid[]) = 0
                  OR EXISTS (SELECT 1 FROM contact_tags x WHERE x.contact_id = c.id AND x.tag_id = ANY($2)))
             AND NOT EXISTS (SELECT 1 FROM contact_tags x WHERE x.contact_id = c.id AND x.tag_id = ANY($3))
             AND NOT EXISTS (SELECT 1 FROM suppressions s WHERE s.account_id = c.account_id AND s.address = c.email)"#,
        account_id,
        include,
        exclude
    )
    .fetch_one(ex)
    .await?;
    Ok(Audience {
        readers: r.readers,
        missing_first_name: r.missing,
    })
}

#[derive(Debug, Clone)]
pub struct Sender {
    /// `Newsletter Name <news@domain>`.
    pub from: String,
    pub domain_id: Uuid,
}

/// The newsletter's from address when it is on a verified domain of the account.
pub async fn sender(pool: &PgPool, account_id: Uuid) -> anyhow::Result<Option<Sender>> {
    let Some(n) = newsletter::get(pool, account_id).await? else {
        return Ok(None);
    };
    let Some(addr) = n.newsletter_from.as_deref() else {
        return Ok(None);
    };
    Ok(
        match domains::find_by_name(pool, account_id, newsletter::from_domain(addr)).await? {
            Some(d) if d.status == DomainStatus::Verified => Some(Sender {
                from: Mailbox::new(Some(n.display_name().to_string()), addr.parse()?).to_string(),
                domain_id: d.id,
            }),
            _ => None,
        },
    )
}

pub struct Report {
    pub checks: Vec<Check>,
    /// HTML bytes of the largest copy (None until the body is a valid document).
    pub size: Option<usize>,
    pub warnings: Vec<String>,
    pub audience: Audience,
    /// The parsed document (None only when it is not a document at all).
    pub doc: Option<Doc>,
    pub sender: Option<Sender>,
    pub newsletter_name: String,
    pub postal_address: String,
}

impl Report {
    pub fn passed(&self) -> bool {
        self.checks.iter().all(|c| c.ok)
    }

    pub fn failed_messages(&self) -> String {
        self.checks
            .iter()
            .filter(|c| !c.ok)
            .map(|c| c.message.as_str())
            .collect::<Vec<_>>()
            .join("; ")
    }

    pub fn first_failed(&self, codes: &[&str]) -> Option<&Check> {
        self.checks
            .iter()
            .find(|c| !c.ok && codes.contains(&c.code))
    }

    /// 422 named by the first failed check, listing every failed one.
    pub fn failure(&self) -> ApiError {
        let code = self
            .checks
            .iter()
            .find(|c| !c.ok)
            .map_or("checks_failed", |c| c.code);
        ApiError::Unprocessable(code, self.failed_messages())
    }
}

fn check(code: &'static str, ok: bool, message: impl Into<String>) -> Check {
    Check {
        code,
        ok,
        message: message.into(),
    }
}

pub async fn run(state: &AppState, c: &Campaign) -> anyhow::Result<Report> {
    let pool = &state.pool;
    let n = newsletter::get(pool, c.account_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("account gone"))?;
    let sender = sender(pool, c.account_id).await?;
    let postal = n.postal_address.clone().unwrap_or_default();
    let doc = blocks::parse(&c.blocks.0).ok();

    let blocks_check = match &doc {
        None => check(
            "bad_blocks",
            false,
            "the issue body is not a valid document",
        ),
        Some(d) => match blocks::validate(d) {
            Err(e) => check(e.code(), false, e.to_string()),
            Ok(()) => {
                let ids = blocks::image_ids(d); // unique
                if state.images.owned(c.account_id, &ids).await?.len() != ids.len() {
                    check(
                        "image_not_found",
                        false,
                        "an image in the issue was not uploaded to this account; upload it again",
                    )
                } else {
                    check("bad_blocks", true, "Blocks are valid")
                }
            }
        },
    };
    let tracking_base = match &sender {
        Some(s) => domains::get_by_id(pool, s.domain_id)
            .await?
            .map(|d| crate::worker::tracking_base(&state.config, &d)),
        None => None,
    }
    .unwrap_or_else(|| state.config.public_base());
    let size = match &doc {
        Some(d) => {
            let issue = Issue {
                subject: &c.subject,
                preview_text: c.preview_text.as_deref(),
                newsletter_name: n.display_name(),
                postal_address: &postal,
                doc: d,
            };
            Some(campaign::email_size(
                &issue,
                &state.config.web_base(),
                &tracking_base,
                &state.config.tracking_secret,
            )?)
        }
        None => None,
    };
    let active = accounts::get(pool, c.account_id)
        .await?
        .is_some_and(|a| a.status == AccountStatus::Active);
    let audience = audience(pool, c.account_id, &c.include_tags, &c.exclude_tags).await?;

    let subject_set = !c.subject.trim().is_empty();
    let checks = vec![
        check(
            "from_address",
            sender.is_some(),
            match (&n.newsletter_from, &sender) {
                (None, _) => "Set the address issues come from (Signup form settings)".to_string(),
                (Some(f), None) => format!(
                    "{} is not a verified domain of this account",
                    newsletter::from_domain(f)
                ),
                (Some(f), Some(_)) => format!("Sends from {f}"),
            },
        ),
        check(
            "subject",
            subject_set,
            if subject_set {
                "Subject is set"
            } else {
                "Add a subject"
            },
        ),
        check(
            "postal_address",
            !postal.is_empty(),
            if postal.is_empty() {
                "Add your postal address (Signup form settings): the law asks for it in every issue"
            } else {
                "Postal address is set"
            },
        ),
        match size {
            Some(s) if s <= MAX_EMAIL_HTML => check(
                "size",
                true,
                format!("{} KB, under the 100 KB clip limit", s.div_ceil(1024)),
            ),
            Some(s) => check(
                "size",
                false,
                format!(
                    "The email is {} KB; Gmail clips over 100 KB. Shorten it",
                    s.div_ceil(1024)
                ),
            ),
            None => check("size", false, "Fix the issue body first"),
        },
        check(
            "account_active",
            active,
            if active {
                "Account is active"
            } else {
                "This account is paused; resume it first"
            },
        ),
        check(
            "readers",
            audience.readers > 0,
            if audience.readers > 0 {
                format!("{} readers", audience.readers)
            } else {
                "No subscribed readers match this audience".to_string()
            },
        ),
        blocks_check,
    ];
    let warnings = if c.preview_text.is_none() {
        vec!["Add preview text: inboxes show it next to the subject.".to_string()]
    } else {
        vec![]
    };
    Ok(Report {
        checks,
        size,
        warnings,
        audience,
        doc,
        sender,
        newsletter_name: n.display_name().to_string(),
        postal_address: postal,
    })
}
