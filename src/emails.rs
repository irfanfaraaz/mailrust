use std::collections::BTreeMap;

use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::{DateTime, Duration, Utc};
use lettre::message::{Mailbox, header::ContentType};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::{PgConnection, PgPool, types::Json};
use uuid::Uuid;

use crate::{error::ApiError, render::AttachmentData};

/// SES: at most 50 recipients per message, To + CC + BCC together.
pub const MAX_RECIPIENTS: usize = 50;
pub const MAX_RECIPIENTS_PER_FIELD: usize = 50;
/// SES: 40 MB per message, measured after base64 encoding.
pub const MAX_TOTAL_BYTES: usize = 40 * 1024 * 1024;
/// Room for the MIME headers and part boundaries around the content.
const MIME_OVERHEAD_BYTES: usize = 64 * 1024;
pub const MAX_SCHEDULE_DAYS: i64 = 30;
pub const MAX_BATCH: usize = 100;
pub const MAX_ATTACHMENTS: usize = 100;
const RESERVED_HEADERS: [&str; 13] = [
    "from",
    "to",
    "cc",
    "bcc",
    "subject",
    "reply-to",
    "date",
    "message-id",
    "mime-version",
    "content-type",
    "content-transfer-encoding",
    "return-path",
    "sender",
];

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize, utoipa::ToSchema,
)]
#[sqlx(type_name = "email_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum EmailStatus {
    Queued,
    Sending,
    Unknown,
    Sent,
    Delivered,
    DeliveryDelayed,
    Bounced,
    Complained,
    Failed,
    Suppressed,
    Canceled,
}

impl EmailStatus {
    /// An event may move an email only to a strictly higher rank ("bad news wins").
    pub fn rank(self) -> u8 {
        match self {
            EmailStatus::Queued | EmailStatus::Sending | EmailStatus::Unknown => 0,
            EmailStatus::Sent => 1,
            EmailStatus::DeliveryDelayed => 2,
            EmailStatus::Delivered => 3,
            EmailStatus::Failed => 4,
            EmailStatus::Bounced => 5,
            EmailStatus::Complained => 6,
            EmailStatus::Suppressed | EmailStatus::Canceled => 7,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            EmailStatus::Queued => "queued",
            EmailStatus::Sending => "sending",
            EmailStatus::Unknown => "unknown",
            EmailStatus::Sent => "sent",
            EmailStatus::Delivered => "delivered",
            EmailStatus::DeliveryDelayed => "delivery_delayed",
            EmailStatus::Bounced => "bounced",
            EmailStatus::Complained => "complained",
            EmailStatus::Failed => "failed",
            EmailStatus::Suppressed => "suppressed",
            EmailStatus::Canceled => "canceled",
        }
    }
}

/// Claim order: urgent, then normal, then bulk. The caller picks it.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    sqlx::Type,
    Serialize,
    Deserialize,
    utoipa::ToSchema,
)]
#[sqlx(type_name = "email_priority", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Urgent,
    #[default]
    Normal,
    Bulk,
}

pub fn api_status(status: EmailStatus, send_at: DateTime<Utc>, now: DateTime<Utc>) -> &'static str {
    if status == EmailStatus::Queued && send_at > now {
        "scheduled"
    } else {
        status.as_str()
    }
}

fn one_or_many<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(String),
        Many(Vec<String>),
    }
    Ok(match OneOrMany::deserialize(d)? {
        OneOrMany::One(s) => vec![s],
        OneOrMany::Many(v) => v,
    })
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SendEmailRequest {
    pub from: String,
    #[serde(deserialize_with = "one_or_many")]
    pub to: Vec<String>,
    #[serde(default, deserialize_with = "one_or_many")]
    pub cc: Vec<String>,
    #[serde(default, deserialize_with = "one_or_many")]
    pub bcc: Vec<String>,
    #[serde(default, deserialize_with = "one_or_many")]
    pub reply_to: Vec<String>,
    pub subject: String,
    pub html: Option<String>,
    pub text: Option<String>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub tags: BTreeMap<String, String>,
    #[serde(default)]
    pub attachments: Vec<AttachmentInput>,
    pub scheduled_at: Option<DateTime<Utc>>,
    /// urgent (login codes, resets), normal (default) or bulk (newsletters).
    #[serde(default)]
    pub priority: Priority,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AttachmentInput {
    pub filename: String,
    /// Base64-encoded file content.
    pub content: String,
    pub content_type: Option<String>,
}

#[derive(Debug)]
pub struct ValidEmail {
    pub from: String,
    pub from_domain: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub reply_to: Vec<String>,
    pub subject: String,
    pub html: Option<String>,
    pub text: Option<String>,
    pub headers: BTreeMap<String, String>,
    pub tags: BTreeMap<String, String>,
    pub attachments: Vec<AttachmentData>,
    /// None = send now, stamped by the database clock (the same clock `queue::claim` compares against).
    pub send_at: Option<DateTime<Utc>>,
    pub priority: Priority,
}

fn bad(msg: impl Into<String>) -> ApiError {
    ApiError::Validation(msg.into())
}

fn check_list(field: &str, list: &[String], min: usize) -> Result<(), ApiError> {
    if list.len() < min {
        return Err(bad(format!("{field} needs at least {min} address")));
    }
    if list.len() > MAX_RECIPIENTS_PER_FIELD {
        return Err(bad(format!(
            "{field} has more than {MAX_RECIPIENTS_PER_FIELD} addresses"
        )));
    }
    for a in list {
        a.parse::<Mailbox>()
            .map_err(|_| bad(format!("{field}: {a} is not a valid address")))?;
    }
    Ok(())
}

/// Size of `n` bytes as MIME base64: 4 chars per 3 bytes, plus CRLF after every 76 chars.
fn base64_wire_len(n: usize) -> usize {
    let chars = n.div_ceil(3) * 4;
    chars + chars.div_ceil(76) * 2
}

fn is_tag_part(s: &str) -> bool {
    (1..=256).contains(&s.len())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub fn validate(req: SendEmailRequest, now: DateTime<Utc>) -> Result<ValidEmail, ApiError> {
    let from_box = req
        .from
        .parse::<Mailbox>()
        .map_err(|_| bad(format!("from: {} is not a valid address", req.from)))?;
    let from_domain = from_box.email.domain().to_ascii_lowercase();
    check_list("to", &req.to, 1)?;
    check_list("cc", &req.cc, 0)?;
    check_list("bcc", &req.bcc, 0)?;
    check_list("reply_to", &req.reply_to, 0)?;
    if req.to.len() + req.cc.len() + req.bcc.len() > MAX_RECIPIENTS {
        return Err(bad(format!(
            "to, cc and bcc together can have at most {MAX_RECIPIENTS} addresses"
        )));
    }

    if req.subject.trim().is_empty()
        || req.subject.chars().count() > 998
        || req.subject.contains(['\r', '\n'])
    {
        return Err(bad("subject must be 1-998 characters on one line"));
    }
    if req.html.is_none() && req.text.is_none() {
        return Err(bad("html or text is required"));
    }

    if req.headers.len() > 50 {
        return Err(bad("at most 50 headers"));
    }
    for (k, v) in &req.headers {
        let name_ok = (1..=100).contains(&k.len())
            && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
        if !name_ok {
            return Err(bad(format!("header name {k} is not allowed")));
        }
        let lower = k.to_ascii_lowercase();
        // SES obeys X-SES-* headers in raw mail (tags, configuration set), so customers must not set them.
        if lower.starts_with("x-ses-") {
            return Err(bad(format!("header {k} is reserved for the sender")));
        }
        if RESERVED_HEADERS.contains(&lower.as_str()) {
            return Err(bad(format!(
                "header {k} is set by Mailrust; use the matching field instead"
            )));
        }
        if v.len() > 998 || !v.chars().all(|c| (' '..='~').contains(&c)) {
            return Err(bad(format!(
                "header {k} must be printable ASCII on one line"
            )));
        }
    }

    if req.tags.len() > 20 {
        return Err(bad("at most 20 tags"));
    }
    for (k, v) in &req.tags {
        if !is_tag_part(k) || !is_tag_part(v) || k == crate::sender::EMAIL_ID_TAG {
            return Err(bad(format!(
                "tag {k} must use letters, digits, '_' or '-' (1-256 chars)"
            )));
        }
    }

    if req.attachments.len() > MAX_ATTACHMENTS {
        return Err(bad(format!("at most {MAX_ATTACHMENTS} attachments")));
    }
    // Worst case every part goes out as base64, as SES measures it.
    let mut total = MIME_OVERHEAD_BYTES
        + base64_wire_len(req.html.as_ref().map_or(0, String::len))
        + base64_wire_len(req.text.as_ref().map_or(0, String::len));
    let mut attachments = Vec::with_capacity(req.attachments.len());
    for a in req.attachments {
        if a.filename.is_empty()
            || a.filename.len() > 255
            || a.filename.contains(['/', '\\'])
            || a.filename.chars().any(char::is_control)
        {
            return Err(bad(
                "attachment filename must be 1-255 characters without slashes or control characters",
            ));
        }
        let content = STANDARD
            .decode(a.content.as_bytes())
            .map_err(|_| bad(format!("attachment {} is not valid base64", a.filename)))?;
        let content_type = a
            .content_type
            .unwrap_or_else(|| "application/octet-stream".into());
        ContentType::parse(&content_type)
            .map_err(|_| bad(format!("attachment {}: bad content_type", a.filename)))?;
        total += base64_wire_len(content.len());
        attachments.push(AttachmentData {
            filename: a.filename,
            content_type,
            content,
        });
    }
    if total > MAX_TOTAL_BYTES {
        return Err(ApiError::TooLarge);
    }

    let send_at = match req.scheduled_at {
        None => None,
        Some(t) if t > now + Duration::days(MAX_SCHEDULE_DAYS) => {
            return Err(bad(format!(
                "scheduled_at can be at most {MAX_SCHEDULE_DAYS} days ahead"
            )));
        }
        Some(t) => (t > now).then_some(t),
    };

    Ok(ValidEmail {
        from: req.from,
        from_domain,
        to: req.to,
        cc: req.cc,
        bcc: req.bcc,
        reply_to: req.reply_to,
        subject: req.subject,
        html: req.html,
        text: req.text,
        headers: req.headers,
        tags: req.tags,
        attachments,
        send_at,
        priority: req.priority,
    })
}

/// Every account may send this many urgent emails a day, or 10% of its mail if that is more.
pub const URGENT_FLOOR: i64 = 100;

/// Urgent use is capped so the urgent reserve keeps working. Over the cap, the email still goes, as normal.
/// One per request: the account's 24-hour counts are read once (on the first urgent email), then kept
/// up to date as this request inserts more, so a 100-email batch does not re-count the account 100 times.
#[derive(Debug, Default)]
pub struct UrgentCap {
    /// (urgent, total) in the last 24 h, including what this request already inserted.
    counts: Option<(i64, i64)>,
}

impl UrgentCap {
    async fn allowed(
        &mut self,
        conn: &mut PgConnection,
        account_id: Uuid,
        wanted: Priority,
    ) -> sqlx::Result<Priority> {
        if wanted != Priority::Urgent {
            return Ok(wanted);
        }
        let (urgent, total) = match self.counts {
            Some(c) => c,
            None => {
                // One urgent request per account at a time, so two cannot both slip past the cap.
                sqlx::query!(
                    "SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))",
                    account_id.to_string()
                )
                .execute(&mut *conn)
                .await?;
                let c = sqlx::query!(
                    r#"SELECT count(*) FILTER (WHERE priority = 'urgent') AS "urgent!", count(*) AS "total!"
                       FROM emails WHERE account_id = $1 AND created_at > now() - interval '24 hours'"#,
                    account_id
                )
                .fetch_one(&mut *conn)
                .await?;
                *self.counts.insert((c.urgent, c.total))
            }
        };
        Ok(if urgent < URGENT_FLOOR.max(total / 10) {
            Priority::Urgent
        } else {
            Priority::Normal
        })
    }

    /// Called after every insert, so later emails in the same request see it.
    fn record(&mut self, stored: Priority) {
        if let Some((u, t)) = &mut self.counts {
            *t += 1;
            if stored == Priority::Urgent {
                *u += 1;
            }
        }
    }
}

pub async fn insert(
    conn: &mut PgConnection,
    account_id: Uuid,
    domain_id: Uuid,
    e: &ValidEmail,
    idempotency: Option<(&str, &[u8])>,
    cap: &mut UrgentCap,
) -> sqlx::Result<(Uuid, Priority)> {
    let priority = cap.allowed(&mut *conn, account_id, e.priority).await?;
    let id = Uuid::now_v7();
    let (key, hash) = match idempotency {
        Some((k, h)) => (Some(k), Some(h)),
        None => (None, None),
    };
    sqlx::query!(
        "INSERT INTO emails (id, account_id, domain_id, from_address, to_addresses, cc_addresses, bcc_addresses,
                             reply_to, subject, html, text_body, headers, tags, send_at, idempotency_key, request_hash, priority)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, COALESCE($14, now()), $15, $16, $17)",
        id,
        account_id,
        domain_id,
        e.from,
        &e.to[..],
        &e.cc[..],
        &e.bcc[..],
        &e.reply_to[..],
        e.subject,
        e.html,
        e.text,
        Json(&e.headers) as _,
        Json(&e.tags) as _,
        e.send_at,
        key,
        hash,
        priority as Priority
    )
    .execute(&mut *conn)
    .await?;
    for a in &e.attachments {
        sqlx::query!(
            "INSERT INTO attachments (id, email_id, account_id, filename, content_type, content)
             VALUES ($1, $2, $3, $4, $5, $6)",
            Uuid::now_v7(),
            id,
            account_id,
            a.filename,
            a.content_type,
            a.content
        )
        .execute(&mut *conn)
        .await?;
    }
    cap.record(priority);
    Ok((id, priority))
}

pub async fn load_attachments(pool: &PgPool, email_id: Uuid) -> sqlx::Result<Vec<AttachmentData>> {
    sqlx::query_as!(
        AttachmentData,
        "SELECT filename, content_type, content FROM attachments WHERE email_id = $1 ORDER BY id",
        email_id
    )
    .fetch_all(pool)
    .await
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SendResponse {
    pub id: Uuid,
    pub priority: Priority,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct EventView {
    pub r#type: String,
    pub recipient: Option<String>,
    pub likely_bot: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct EmailView {
    pub id: Uuid,
    pub from: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub reply_to: Vec<String>,
    pub subject: String,
    pub html: Option<String>,
    pub text: Option<String>,
    pub status: String,
    pub priority: Priority,
    pub scheduled_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub tags: BTreeMap<String, String>,
    pub events: Vec<EventView>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct EmailSummary {
    pub id: Uuid,
    pub from: String,
    pub to: Vec<String>,
    pub subject: String,
    pub status: String,
    pub priority: Priority,
    pub scheduled_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

pub async fn view(pool: &PgPool, account_id: Uuid, id: Uuid) -> sqlx::Result<Option<EmailView>> {
    let Some(r) = sqlx::query!(
        r#"SELECT id, from_address, to_addresses, cc_addresses, bcc_addresses, reply_to, subject, html, text_body,
                  status AS "status: EmailStatus", priority AS "priority: Priority", send_at, created_at, sent_at, last_error,
                  tags AS "tags: Json<BTreeMap<String, String>>"
           FROM emails WHERE id = $1 AND account_id = $2"#,
        id,
        account_id
    )
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    let events = sqlx::query_as!(
        EventView,
        r#"SELECT type AS "type", recipient, likely_bot, created_at FROM email_events WHERE email_id = $1 ORDER BY created_at, id"#,
        id
    )
    .fetch_all(pool)
    .await?;
    Ok(Some(EmailView {
        id: r.id,
        from: r.from_address,
        to: r.to_addresses,
        cc: r.cc_addresses,
        bcc: r.bcc_addresses,
        reply_to: r.reply_to,
        subject: r.subject,
        html: r.html,
        text: r.text_body,
        status: api_status(r.status, r.send_at, Utc::now()).to_string(),
        priority: r.priority,
        scheduled_at: r.send_at,
        created_at: r.created_at,
        sent_at: r.sent_at,
        last_error: r.last_error,
        tags: r.tags.0,
        events,
    }))
}

pub async fn list(
    pool: &PgPool,
    account_id: Uuid,
    status: Option<EmailStatus>,
    after: Option<Uuid>,
    limit: i64,
) -> sqlx::Result<Vec<EmailSummary>> {
    let rows = sqlx::query!(
        r#"SELECT id, from_address, to_addresses, subject, status AS "status: EmailStatus", priority AS "priority: Priority", send_at, created_at
           FROM emails
           WHERE account_id = $1 AND ($2::email_status IS NULL OR status = $2) AND ($3::uuid IS NULL OR id < $3)
           ORDER BY id DESC LIMIT $4"#,
        account_id,
        status as Option<EmailStatus>,
        after,
        limit
    )
    .fetch_all(pool)
    .await?;
    let now = Utc::now();
    Ok(rows
        .into_iter()
        .map(|r| EmailSummary {
            id: r.id,
            from: r.from_address,
            to: r.to_addresses,
            subject: r.subject,
            status: api_status(r.status, r.send_at, now).to_string(),
            priority: r.priority,
            scheduled_at: r.send_at,
            created_at: r.created_at,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn req(v: serde_json::Value) -> SendEmailRequest {
        serde_json::from_value(v).unwrap()
    }

    fn base() -> serde_json::Value {
        json!({ "from": "Shop <hi@Shop.com>", "to": "a@x.com", "subject": "Hi", "text": "yo" })
    }

    #[test]
    fn priority_is_kept_and_lanes_sort_urgent_first() {
        let mut v = base();
        v["priority"] = json!("bulk");
        assert_eq!(
            validate(req(v), Utc::now()).unwrap().priority,
            Priority::Bulk
        );
        assert_eq!(
            validate(req(base()), Utc::now()).unwrap().priority,
            Priority::Normal
        );
        assert!(Priority::Urgent < Priority::Normal && Priority::Normal < Priority::Bulk);
    }

    #[test]
    fn accepts_a_string_or_a_list_and_lowercases_the_domain() {
        let v = validate(req(base()), Utc::now()).unwrap();
        assert_eq!(v.to, vec!["a@x.com"]);
        assert_eq!(v.from_domain, "shop.com");
    }

    #[test]
    fn rejects_bad_input() {
        let now = Utc::now();
        let cases = [
            json!({ "from": "nope", "to": "a@x.com", "subject": "Hi", "text": "x" }),
            json!({ "from": "a@b.com", "to": [], "subject": "Hi", "text": "x" }),
            json!({ "from": "a@b.com", "to": "a@x.com", "subject": "", "text": "x" }),
            json!({ "from": "a@b.com", "to": "a@x.com", "subject": "a\r\nBcc: evil@x.com", "text": "x" }),
            json!({ "from": "a@b.com", "to": "a@x.com", "subject": "Hi" }),
            json!({ "from": "a@b.com", "to": "a@x.com", "subject": "Hi", "text": "x", "headers": { "Bcc": "e@x.com" } }),
            json!({ "from": "a@b.com", "to": "a@x.com", "subject": "Hi", "text": "x", "headers": { "X-A": "line\nbreak" } }),
            json!({ "from": "a@b.com", "to": "a@x.com", "subject": "Hi", "text": "x", "tags": { "mailrust_email_id": "x" } }),
            json!({ "from": "a@b.com", "to": "a@x.com", "subject": "Hi", "text": "x", "attachments": [{ "filename": "a.txt", "content": "%%%" }] }),
            json!({ "from": "a@b.com", "to": "a@x.com", "subject": "Hi", "text": "x", "scheduled_at": (now + Duration::days(31)).to_rfc3339() }),
        ];
        for c in cases {
            assert!(
                matches!(validate(req(c.clone()), now), Err(ApiError::Validation(_))),
                "should reject {c}"
            );
        }
        let many: Vec<String> = (0..51).map(|i| format!("u{i}@x.com")).collect();
        let c = json!({ "from": "a@b.com", "to": many, "subject": "Hi", "text": "x" });
        assert!(matches!(
            validate(req(c), now),
            Err(ApiError::Validation(_))
        ));
    }

    #[test]
    fn limits_are_inclusive() {
        let now = Utc::now();
        let mut v = base();
        v["to"] = json!((0..50).map(|i| format!("u{i}@x.com")).collect::<Vec<_>>());
        v["subject"] = json!("s".repeat(998));
        v["headers"] = json!(
            (0..50)
                .map(|i| (format!("X-H{i}"), "v".to_string()))
                .collect::<BTreeMap<_, _>>()
        );
        v["tags"] = json!(
            (0..20)
                .map(|i| (format!("t_{i}"), "a-b".to_string()))
                .collect::<BTreeMap<_, _>>()
        );
        validate(req(v), now).expect("exactly at every limit is allowed");
    }

    #[test]
    fn rejects_bad_headers_and_tags() {
        let now = Utc::now();
        let with = |k: &str, val: serde_json::Value| {
            let mut v = base();
            v[k] = val;
            v
        };
        let cases = [
            with(
                "headers",
                json!(
                    (0..51)
                        .map(|i| (format!("X-H{i}"), "v".to_string()))
                        .collect::<BTreeMap<_, _>>()
                ),
            ),
            with("headers", json!({ "X Bad": "v" })),
            with("headers", json!({ "X-Ok": "caf\u{e9}" })),
            with(
                "tags",
                json!(
                    (0..21)
                        .map(|i| (format!("t{i}"), "v".to_string()))
                        .collect::<BTreeMap<_, _>>()
                ),
            ),
            with("tags", json!({ "a.b": "v" })),
            with("tags", json!({ "ok": "" })),
            with("tags", json!({ "ok": "has space" })),
        ];
        for c in cases {
            assert!(
                matches!(validate(req(c.clone()), now), Err(ApiError::Validation(_))),
                "should reject {c}"
            );
        }
    }

    #[test]
    fn attachments_over_40_mb_in_total_are_too_large() {
        assert_eq!(MAX_TOTAL_BYTES, 40 * 1024 * 1024);
        // The JSON body carries base64 (4/3 bigger), so the body limit must fit 40 MB of attachments.
        const { assert!(crate::api::MAX_BODY_BYTES > MAX_TOTAL_BYTES / 3 * 4 + 1024 * 1024) };
        let half = STANDARD.encode(vec![0u8; MAX_TOTAL_BYTES / 2 + 1]);
        let mut v = base();
        v["attachments"] = json!([
            { "filename": "a.bin", "content": half },
            { "filename": "b.bin", "content": half }
        ]);
        assert!(matches!(
            validate(req(v), Utc::now()),
            Err(ApiError::TooLarge)
        ));
    }

    #[test]
    fn at_most_50_recipients_across_to_cc_and_bcc() {
        let now = Utc::now();
        let list = |n: usize, p: &str| (0..n).map(|i| format!("{p}{i}@x.com")).collect::<Vec<_>>();
        let mut v = base();
        v["to"] = json!(list(20, "t"));
        v["cc"] = json!(list(20, "c"));
        v["bcc"] = json!(list(10, "b"));
        validate(req(v.clone()), now).expect("50 in total is allowed");
        v["bcc"] = json!(list(11, "b"));
        assert!(matches!(
            validate(req(v), now),
            Err(ApiError::Validation(_))
        ));
    }

    #[test]
    fn ses_control_headers_are_rejected() {
        for name in [
            "X-SES-MESSAGE-TAGS",
            "x-ses-configuration-set",
            "X-Ses-Source-Arn",
        ] {
            let mut v = base();
            v["headers"] = json!({ name: "mailrust_email_id=someone-else" });
            assert!(
                matches!(validate(req(v), Utc::now()), Err(ApiError::Validation(_))),
                "should reject {name}"
            );
        }
    }

    #[test]
    fn attachment_count_and_filenames_are_limited() {
        let now = Utc::now();
        let one = json!({ "filename": "a.txt", "content": "" });
        let mut v = base();
        v["attachments"] = json!(vec![one.clone(); MAX_ATTACHMENTS]);
        validate(req(v.clone()), now).expect("exactly the limit is allowed");
        v["attachments"] = json!(vec![one; MAX_ATTACHMENTS + 1]);
        assert!(matches!(
            validate(req(v), now),
            Err(ApiError::Validation(_))
        ));
        for bad_name in ["a\r\nX-Evil: 1.txt", "tab\there.txt", "nul\u{0}.txt"] {
            let mut v = base();
            v["attachments"] = json!([{ "filename": bad_name, "content": "" }]);
            assert!(
                matches!(validate(req(v), now), Err(ApiError::Validation(_))),
                "should reject {bad_name:?}"
            );
        }
    }

    #[test]
    fn size_is_measured_after_base64_like_ses_does() {
        let now = Utc::now();
        let file = |mb: usize| {
            let mut v = base();
            v["attachments"] = json!([{ "filename": "f.bin", "content": STANDARD.encode(vec![0u8; mb * 1024 * 1024]) }]);
            v
        };
        validate(req(file(28)), now).expect("28 MB of files is about 38 MB on the wire");
        assert!(matches!(
            validate(req(file(30)), now),
            Err(ApiError::TooLarge)
        ));
    }

    #[test]
    fn past_schedule_becomes_now_and_scheduled_status_shows() {
        let now = Utc::now();
        let mut v = base();
        v["scheduled_at"] = json!((now - Duration::hours(1)).to_rfc3339());
        assert_eq!(validate(req(v), now).unwrap().send_at, None);
        assert_eq!(
            api_status(EmailStatus::Queued, now + Duration::hours(1), now),
            "scheduled"
        );
        assert_eq!(api_status(EmailStatus::Queued, now, now), "queued");
    }

    #[test]
    fn bad_news_outranks_good_news() {
        assert!(EmailStatus::Bounced.rank() > EmailStatus::Delivered.rank());
        assert!(EmailStatus::Delivered.rank() > EmailStatus::DeliveryDelayed.rank());
        assert!(EmailStatus::Complained.rank() > EmailStatus::Bounced.rank());
    }
}
