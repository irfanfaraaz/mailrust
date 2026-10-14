//! The per-issue report: totals, unique open/click rates (bots left out), clicks per link,
//! a split per mailbox provider and wave progress.
use std::collections::BTreeMap;

use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Default, Clone, Serialize, utoipa::ToSchema)]
pub struct ReportTotals {
    pub recipients: i64,
    /// Handed to the sender (everything but queued, failed, skipped and cancelled).
    pub sent: i64,
    pub delivered: i64,
    pub bounced: i64,
    pub complained: i64,
    pub failed: i64,
    /// Readers who left or were blocked before their copy went out.
    pub skipped: i64,
    pub opened: i64,
    pub clicked: i64,
    pub unsubscribed: i64,
}

/// Each over `sent`, 0..1.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ReportRates {
    pub open_rate: f64,
    pub click_rate: f64,
    pub bounce_rate: f64,
    pub complaint_rate: f64,
    pub unsubscribe_rate: f64,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct LinkStat {
    pub url: String,
    pub clicks: i64,
    pub unique_clicks: i64,
}

#[derive(Debug, Clone, Default, Serialize, utoipa::ToSchema)]
pub struct ProviderStat {
    /// Gmail, Outlook, Yahoo, iCloud, Proton or Other.
    pub provider: String,
    pub sent: i64,
    pub delivered: i64,
    pub opened: i64,
    pub clicked: i64,
    pub bounced: i64,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct WaveStat {
    pub wave: i32,
    pub readers: i64,
    /// Readers whose copy reached a final state.
    pub done: i64,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct CampaignReport {
    pub totals: ReportTotals,
    pub rates: ReportRates,
    pub links: Vec<LinkStat>,
    pub providers: Vec<ProviderStat>,
    pub waves: Vec<WaveStat>,
}

pub fn provider_of(domain: &str) -> &'static str {
    let d = domain.trim().to_ascii_lowercase();
    match d.as_str() {
        "gmail.com" | "googlemail.com" => "Gmail",
        "outlook.com" | "hotmail.com" | "live.com" | "msn.com" => "Outlook",
        "yahoo.com" | "ymail.com" | "aol.com" => "Yahoo",
        "icloud.com" | "me.com" | "mac.com" => "iCloud",
        "proton.me" | "protonmail.com" | "pm.me" => "Proton",
        d if d.starts_with("hotmail.") || d.starts_with("outlook.") || d.starts_with("live.") => {
            "Outlook"
        }
        d if d.starts_with("yahoo.") => "Yahoo",
        _ => "Other",
    }
}

fn rate(n: i64, of: i64) -> f64 {
    if of <= 0 { 0.0 } else { n as f64 / of as f64 }
}

pub async fn build(pool: &PgPool, campaign_id: Uuid) -> sqlx::Result<CampaignReport> {
    let rows = sqlx::query!(
        r#"SELECT lower(split_part(e.to_addresses[1], '@', 2)) AS "domain!",
                  count(*) AS "recipients!",
                  count(*) FILTER (WHERE e.status NOT IN ('queued', 'sending', 'unknown', 'failed', 'suppressed', 'canceled')) AS "sent!",
                  count(*) FILTER (WHERE e.status = 'failed') AS "failed!",
                  count(*) FILTER (WHERE e.status = 'suppressed') AS "skipped!",
                  count(*) FILTER (WHERE ev.delivered) AS "delivered!",
                  count(*) FILTER (WHERE ev.bounced) AS "bounced!",
                  count(*) FILTER (WHERE ev.complained) AS "complained!",
                  count(*) FILTER (WHERE ev.opened) AS "opened!",
                  count(*) FILTER (WHERE ev.clicked) AS "clicked!",
                  count(*) FILTER (WHERE ev.unsubscribed) AS "unsubscribed!"
           FROM emails e
           LEFT JOIN LATERAL (
             SELECT bool_or(v.type = 'delivered') AS delivered, bool_or(v.type = 'bounced') AS bounced,
                    bool_or(v.type = 'complained') AS complained,
                    bool_or(v.type = 'opened' AND NOT v.likely_bot) AS opened,
                    bool_or(v.type = 'clicked' AND NOT v.likely_bot) AS clicked,
                    bool_or(v.type = 'unsubscribed') AS unsubscribed
             FROM email_events v WHERE v.email_id = e.id) ev ON true
           WHERE e.campaign_id = $1
           GROUP BY 1"#,
        campaign_id
    )
    .fetch_all(pool)
    .await?;
    let mut totals = ReportTotals::default();
    let mut providers: BTreeMap<&'static str, ProviderStat> = BTreeMap::new();
    for r in rows {
        totals.recipients += r.recipients;
        totals.sent += r.sent;
        totals.delivered += r.delivered;
        totals.bounced += r.bounced;
        totals.complained += r.complained;
        totals.failed += r.failed;
        totals.skipped += r.skipped;
        totals.opened += r.opened;
        totals.clicked += r.clicked;
        totals.unsubscribed += r.unsubscribed;
        let name = provider_of(&r.domain);
        let p = providers.entry(name).or_insert_with(|| ProviderStat {
            provider: name.to_string(),
            ..Default::default()
        });
        p.sent += r.sent;
        p.delivered += r.delivered;
        p.opened += r.opened;
        p.clicked += r.clicked;
        p.bounced += r.bounced;
    }
    let mut providers: Vec<ProviderStat> = providers.into_values().collect();
    // Biggest first; "Other" always last.
    providers.sort_by_key(|p| (p.provider == "Other", -p.sent));

    let links = sqlx::query_as!(
        LinkStat,
        r#"SELECT v.data->>'link' AS "url!", count(*) AS "clicks!", count(DISTINCT v.email_id) AS "unique_clicks!"
           FROM email_events v JOIN emails e ON e.id = v.email_id
           WHERE e.campaign_id = $1 AND v.type = 'clicked' AND NOT v.likely_bot AND v.data ? 'link'
           GROUP BY 1 ORDER BY 2 DESC, 1"#,
        campaign_id
    )
    .fetch_all(pool)
    .await?;
    let waves = sqlx::query_as!(
        WaveStat,
        r#"SELECT r.wave, count(*) AS "readers!",
                  count(*) FILTER (WHERE e.status IS NOT NULL AND e.status NOT IN ('queued', 'sending', 'unknown')) AS "done!"
           FROM campaign_recipients r LEFT JOIN emails e ON e.id = r.email_id
           WHERE r.campaign_id = $1 GROUP BY r.wave ORDER BY r.wave"#,
        campaign_id
    )
    .fetch_all(pool)
    .await?;
    let rates = ReportRates {
        open_rate: rate(totals.opened, totals.sent),
        click_rate: rate(totals.clicked, totals.sent),
        bounce_rate: rate(totals.bounced, totals.sent),
        complaint_rate: rate(totals.complained, totals.sent),
        unsubscribe_rate: rate(totals.unsubscribed, totals.sent),
    };
    Ok(CampaignReport {
        totals,
        rates,
        links,
        providers,
        waves,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn providers_by_domain() {
        assert_eq!(provider_of("gmail.com"), "Gmail");
        assert_eq!(provider_of("GoogleMail.com"), "Gmail");
        assert_eq!(provider_of("hotmail.co.uk"), "Outlook");
        assert_eq!(provider_of("outlook.com"), "Outlook");
        assert_eq!(provider_of("yahoo.fr"), "Yahoo");
        assert_eq!(provider_of("aol.com"), "Yahoo");
        assert_eq!(provider_of("me.com"), "iCloud");
        assert_eq!(provider_of("proton.me"), "Proton");
        assert_eq!(provider_of("shop.com"), "Other");
        assert_eq!(provider_of(""), "Other");
    }

    #[test]
    fn rates_never_divide_by_zero() {
        assert_eq!(rate(0, 0), 0.0);
        assert_eq!(rate(1, 4), 0.25);
    }
}
