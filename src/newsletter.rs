//! The account's newsletter: public signup settings, whether the form is open, and (Task 7) the join flow.
use lettre::message::Mailbox;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    accounts::AccountStatus,
    api::AppState,
    auth_tokens,
    contacts::ContactStatus,
    domains::{self, DomainStatus},
    keys::hash_token,
    system_mail,
};

pub const CONFIRM_DAYS: i32 = 7;
pub const CONFIRM_EVERY_MINUTES: i32 = 10;
pub const JOIN_PER_IP_PER_HOUR: i64 = 5;

#[derive(Debug, Clone)]
pub struct Newsletter {
    pub account_id: Uuid,
    pub account_name: String,
    pub status: AccountStatus,
    pub public_slug: Option<String>,
    pub newsletter_name: Option<String>,
    pub newsletter_from: Option<String>,
    pub signup_enabled: bool,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct NewsletterSettings {
    pub public_slug: Option<String>,
    pub newsletter_name: Option<String>,
    pub newsletter_from: Option<String>,
    pub signup_enabled: bool,
    /// True when `/join/<slug>` takes signups right now.
    pub signup_open: bool,
    /// Why the form is closed, when it is.
    pub closed_reason: Option<String>,
}

impl Newsletter {
    pub fn display_name(&self) -> &str {
        self.newsletter_name
            .as_deref()
            .unwrap_or(&self.account_name)
    }

    /// None when the public form is open.
    pub async fn closed_reason(&self, pool: &PgPool) -> sqlx::Result<Option<String>> {
        if self.public_slug.is_none() {
            return Ok(Some("choose a public address for the form".into()));
        }
        if !self.signup_enabled {
            return Ok(Some("signups are turned off".into()));
        }
        let Some(from) = self.newsletter_from.as_deref() else {
            return Ok(Some("choose a from address for the confirm email".into()));
        };
        if self.status == AccountStatus::Paused {
            return Ok(Some("the account is paused".into()));
        }
        let domain = from_domain(from);
        Ok(
            match domains::find_by_name(pool, self.account_id, domain).await? {
                Some(d) if d.status == DomainStatus::Verified => None,
                _ => Some(format!("{domain} is not a verified domain of this account")),
            },
        )
    }

    pub async fn settings(&self, pool: &PgPool) -> sqlx::Result<NewsletterSettings> {
        let closed_reason = self.closed_reason(pool).await?;
        Ok(NewsletterSettings {
            public_slug: self.public_slug.clone(),
            newsletter_name: self.newsletter_name.clone(),
            newsletter_from: self.newsletter_from.clone(),
            signup_enabled: self.signup_enabled,
            signup_open: closed_reason.is_none(),
            closed_reason,
        })
    }
}

pub async fn get(pool: &PgPool, account_id: Uuid) -> sqlx::Result<Option<Newsletter>> {
    sqlx::query_as!(
        Newsletter,
        r#"SELECT id AS account_id, name AS account_name, status AS "status: AccountStatus",
                  public_slug, newsletter_name, newsletter_from, signup_enabled
           FROM accounts WHERE id = $1"#,
        account_id
    )
    .fetch_optional(pool)
    .await
}

pub async fn by_slug(pool: &PgPool, slug: &str) -> sqlx::Result<Option<Newsletter>> {
    sqlx::query_as!(
        Newsletter,
        r#"SELECT id AS account_id, name AS account_name, status AS "status: AccountStatus",
                  public_slug, newsletter_name, newsletter_from, signup_enabled
           FROM accounts WHERE public_slug = $1"#,
        slug
    )
    .fetch_optional(pool)
    .await
}

pub fn normalize_slug(raw: &str) -> Result<Option<String>, String> {
    let s = raw.trim().to_ascii_lowercase();
    if s.is_empty() {
        return Ok(None);
    }
    let ok = (3..=40).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if ok {
        Ok(Some(s))
    } else {
        Err("the public address is 3-40 characters of a-z, 0-9 and -".into())
    }
}

pub fn normalize_from(raw: &str) -> Result<Option<String>, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Ok(None);
    }
    let m: Mailbox = s
        .parse()
        .map_err(|_| format!("{s} is not a valid address"))?;
    Ok(Some(m.email.to_string().to_ascii_lowercase()))
}

pub fn from_domain(from: &str) -> &str {
    from.rsplit_once('@').map_or("", |(_, d)| d)
}

/// Runs after `POST /public/join` has answered: new or unsubscribed → `pending` + confirm email;
/// pending → a new email unless one went out in the last 10 minutes; subscribed → nothing.
pub async fn process_join(
    state: &AppState,
    n: &Newsletter,
    email: &str,
    first_name: Option<String>,
) -> anyhow::Result<()> {
    let Some(from) = n.newsletter_from.as_deref() else {
        return Ok(());
    };
    let mut tx = state.pool.begin().await?;
    sqlx::query!(
        "INSERT INTO contacts (id, account_id, email, first_name, status, source, pending_since) VALUES ($1, $2, $3, $4, 'pending', 'form', now())
         ON CONFLICT (account_id, email) DO NOTHING",
        Uuid::now_v7(),
        n.account_id,
        email,
        first_name
    )
    .execute(&mut *tx)
    .await?;
    let c = sqlx::query!(
        r#"SELECT id, status AS "status: ContactStatus",
                  EXISTS (SELECT 1 FROM contact_confirmations cc WHERE cc.contact_id = contacts.id
                          AND cc.created_at > now() - make_interval(mins => $3)) AS "recent!"
           FROM contacts WHERE account_id = $1 AND email = $2 FOR UPDATE"#,
        n.account_id,
        email,
        CONFIRM_EVERY_MINUTES
    )
    .fetch_one(&mut *tx)
    .await?;
    if c.status == ContactStatus::Subscribed || c.recent {
        return Ok(());
    }
    sqlx::query!(
        "UPDATE contacts SET status = 'pending', pending_since = now(), first_name = COALESCE($2, first_name), updated_at = now() WHERE id = $1",
        c.id,
        first_name
    )
    .execute(&mut *tx)
    .await?;
    // Only the newest link works: an older one left in an inbox cannot confirm later.
    sqlx::query!(
        "UPDATE contact_confirmations SET used_at = now() WHERE contact_id = $1 AND used_at IS NULL",
        c.id
    )
    .execute(&mut *tx)
    .await?;
    let token = auth_tokens::generate(auth_tokens::CONFIRM_PREFIX);
    sqlx::query!(
        "INSERT INTO contact_confirmations (token_hash, contact_id, expires_at) VALUES ($1, $2, now() + make_interval(days => $3))",
        hash_token(&token),
        c.id,
        CONFIRM_DAYS
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let name = n.display_name();
    let link = format!("{}/confirm/{token}", state.config.web_base());
    let text = format!(
        "Please confirm that you want to get {name}:\n\n{link}\n\nThe link works for {CONFIRM_DAYS} days.\nIf you did not ask for this, ignore this email; you will not be added.\n"
    );
    let from = Mailbox::new(Some(name.to_string()), from.parse()?).to_string();
    let subject = format!("Confirm your subscription to {name}");
    system_mail::queue_from(state, n.account_id, &from, email, &subject, text).await?;
    tracing::info!(contact_id = %c.id, account_id = %n.account_id, "confirm email queued");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_trimmed_lower_cased_and_checked() {
        assert_eq!(
            normalize_slug("  Weekly ").unwrap().as_deref(),
            Some("weekly")
        );
        assert_eq!(
            normalize_slug("shop-news-2").unwrap().as_deref(),
            Some("shop-news-2")
        );
        assert_eq!(normalize_slug("   ").unwrap(), None);
        for bad in ["ab", "a_b", "a b", "ünï", &"x".repeat(41)] {
            assert!(normalize_slug(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn from_is_a_bare_lower_case_address() {
        assert_eq!(
            normalize_from(" Hello@News.Shop.COM ").unwrap().as_deref(),
            Some("hello@news.shop.com")
        );
        assert_eq!(
            normalize_from("Shop <hi@shop.com>").unwrap().as_deref(),
            Some("hi@shop.com")
        );
        assert_eq!(normalize_from("").unwrap(), None);
        assert!(normalize_from("nope").is_err());
        assert_eq!(from_domain("hi@news.shop.com"), "news.shop.com");
    }
}
