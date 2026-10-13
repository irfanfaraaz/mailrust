//! Contacts (one audience per account) and their tags.
use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool, types::Json};
use uuid::Uuid;

use crate::users::normalize_email;

pub const MAX_FIELDS: usize = 50;
pub const MAX_FIELD_KEY: usize = 40;
pub const MAX_FIELD_VALUE: usize = 500;
pub const MAX_NAME: usize = 100;
pub const MAX_TAG: usize = 50;

pub type Fields = BTreeMap<String, String>;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize, utoipa::ToSchema,
)]
#[sqlx(type_name = "contact_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ContactStatus {
    Pending,
    Subscribed,
    Unsubscribed,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize, utoipa::ToSchema,
)]
#[sqlx(type_name = "contact_source", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ContactSource {
    Form,
    Import,
    Api,
    Dashboard,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TagRef {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ContactView {
    pub id: Uuid,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    #[schema(value_type = BTreeMap<String, String>)]
    pub fields: Json<Fields>,
    pub status: ContactStatus,
    pub source: ContactSource,
    #[schema(value_type = Vec<TagRef>)]
    pub tags: Json<Vec<TagRef>>,
    /// The address is on the account's suppression list: nothing will be sent to it.
    pub blocked: bool,
    pub subscribed_at: Option<DateTime<Utc>>,
    pub unsubscribed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Fixed export columns: a custom field may not use these names.
pub const RESERVED_KEYS: [&str; 6] = [
    "email",
    "first_name",
    "last_name",
    "status",
    "tags",
    "created_at",
];

/// Postgres cannot store NUL and the rest breaks headers and CSVs. Field values may keep line breaks and tabs.
fn no_control(s: &str, allow_breaks: bool) -> Result<(), String> {
    let bad = |c: char| c.is_control() && !(allow_breaks && matches!(c, '\n' | '\r' | '\t'));
    if s.chars().any(bad) {
        return Err("text cannot hold control characters".into());
    }
    Ok(())
}

pub fn valid_field_key(k: &str) -> bool {
    (1..=MAX_FIELD_KEY).contains(&k.len())
        && k.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// Drops empty values, then checks the count, the keys and the value lengths.
pub fn clean_fields(fields: Fields) -> Result<Fields, String> {
    let fields: Fields = fields
        .into_iter()
        .map(|(k, v)| (k, v.trim().to_string()))
        .filter(|(_, v)| !v.is_empty())
        .collect();
    if fields.len() > MAX_FIELDS {
        return Err(format!("a contact has at most {MAX_FIELDS} fields"));
    }
    for (k, v) in &fields {
        if !valid_field_key(k) {
            return Err(format!(
                "field name {k:?} must be 1-{MAX_FIELD_KEY} characters of a-z, 0-9 and _"
            ));
        }
        if RESERVED_KEYS.contains(&k.as_str()) {
            return Err(format!("field name {k} is taken by a fixed column"));
        }
        no_control(v, true).map_err(|e| format!("field {k}: {e}"))?;
        if v.chars().count() > MAX_FIELD_VALUE {
            return Err(format!(
                "field {k} is longer than {MAX_FIELD_VALUE} characters"
            ));
        }
    }
    Ok(fields)
}

/// Trimmed; empty becomes None. No control characters, line breaks included.
pub fn clean_name(name: Option<&str>) -> Result<Option<String>, String> {
    match name.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(n) if no_control(n, false).is_err() => {
            Err("names cannot hold line breaks or control characters".into())
        }
        Some(n) if n.chars().count() > MAX_NAME => {
            Err(format!("names are at most {MAX_NAME} characters"))
        }
        Some(n) => Ok(Some(n.to_string())),
    }
}

/// Trims, checks 1-50 chars, drops repeats that differ only in case (first spelling wins).
pub fn clean_tags(names: &[String]) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for n in names {
        let t = n.trim();
        if t.is_empty() || t.chars().count() > MAX_TAG {
            return Err(format!("tag names are 1-{MAX_TAG} characters"));
        }
        no_control(t, false).map_err(|e| format!("tag names: {e}"))?;
        if !out.iter().any(|x| x.to_lowercase() == t.to_lowercase()) {
            out.push(t.to_string());
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Default)]
pub struct ContactInput {
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub fields: Fields,
    pub tags: Vec<String>,
}

impl ContactInput {
    pub fn validate(self) -> Result<Self, String> {
        Ok(Self {
            email: normalize_email(&self.email)?,
            first_name: clean_name(self.first_name.as_deref())?,
            last_name: clean_name(self.last_name.as_deref())?,
            fields: clean_fields(self.fields)?,
            tags: clean_tags(&self.tags)?,
        })
    }
}

/// Finds or creates each tag by name (ignoring case). Ids come back in the order of `names`.
/// Inserts in sorted order with DO NOTHING: existing tags are never locked, so concurrent writers cannot deadlock.
pub async fn ensure_tags(
    conn: &mut PgConnection,
    account_id: Uuid,
    names: &[String],
) -> sqlx::Result<Vec<Uuid>> {
    let mut sorted: Vec<&String> = names.iter().collect();
    sorted.sort_by_key(|n| n.to_lowercase());
    sorted.dedup_by_key(|n| n.to_lowercase());
    for name in &sorted {
        sqlx::query!(
            "INSERT INTO tags (id, account_id, name) VALUES ($1, $2, $3)
             ON CONFLICT (account_id, lower(name)) DO NOTHING",
            Uuid::now_v7(),
            account_id,
            name.as_str()
        )
        .execute(&mut *conn)
        .await?;
    }
    // Matched in Postgres (its lower(), not Rust's) and returned in input order.
    let ids = sqlx::query_scalar!(
        r#"SELECT t.id FROM unnest($2::text[]) WITH ORDINALITY AS n(name, ord)
           JOIN tags t ON t.account_id = $1 AND lower(t.name) = lower(n.name)
           ORDER BY n.ord"#,
        account_id,
        names
    )
    .fetch_all(&mut *conn)
    .await?;
    if ids.len() != names.len() {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(ids)
}

/// Adds tags to a contact. Ids that are gone or belong to another account are skipped.
pub async fn add_tags(
    conn: &mut PgConnection,
    contact_id: Uuid,
    tag_ids: &[Uuid],
) -> sqlx::Result<()> {
    if tag_ids.is_empty() {
        return Ok(());
    }
    sqlx::query!(
        "INSERT INTO contact_tags (contact_id, tag_id)
         SELECT c.id, t.id FROM contacts c JOIN tags t ON t.account_id = c.account_id
         WHERE c.id = $1 AND t.id = ANY($2)
         ON CONFLICT DO NOTHING",
        contact_id,
        tag_ids
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Makes `tag_ids` the contact's whole tag set.
pub async fn set_tags(
    conn: &mut PgConnection,
    contact_id: Uuid,
    tag_ids: &[Uuid],
) -> sqlx::Result<()> {
    sqlx::query!(
        "DELETE FROM contact_tags WHERE contact_id = $1 AND tag_id <> ALL($2)",
        contact_id,
        tag_ids
    )
    .execute(&mut *conn)
    .await?;
    add_tags(conn, contact_id, tag_ids).await
}

/// New address → `subscribed` from `source`. Existing → given names replace, fields merge; status and source never change.
// shortcut: merged fields can pass 50 keys (each write is checked alone); cap the merge if it matters.
pub async fn upsert(
    conn: &mut PgConnection,
    account_id: Uuid,
    c: &ContactInput,
    source: ContactSource,
) -> sqlx::Result<(Uuid, bool)> {
    let r = sqlx::query!(
        r#"INSERT INTO contacts (id, account_id, email, first_name, last_name, fields, status, source, subscribed_at)
           VALUES ($1, $2, $3, $4, $5, $6, 'subscribed', $7, now())
           ON CONFLICT (account_id, email) DO UPDATE SET
             first_name = COALESCE(EXCLUDED.first_name, contacts.first_name),
             last_name = COALESCE(EXCLUDED.last_name, contacts.last_name),
             fields = contacts.fields || EXCLUDED.fields,
             updated_at = now()
           RETURNING id, (xmax = 0) AS "inserted!""#,
        Uuid::now_v7(),
        account_id,
        c.email,
        c.first_name,
        c.last_name,
        Json(&c.fields) as _,
        source as ContactSource
    )
    .fetch_one(&mut *conn)
    .await?;
    Ok((r.id, r.inserted))
}

/// `upsert`, then add the input's tags (created if new) and `extra_tag`.
pub async fn save(
    conn: &mut PgConnection,
    account_id: Uuid,
    c: &ContactInput,
    source: ContactSource,
    extra_tag: Option<Uuid>,
) -> sqlx::Result<(Uuid, bool)> {
    let (id, new) = upsert(&mut *conn, account_id, c, source).await?;
    let mut tag_ids = ensure_tags(&mut *conn, account_id, &c.tags).await?;
    tag_ids.extend(extra_tag);
    add_tags(&mut *conn, id, &tag_ids).await?;
    Ok((id, new))
}

pub async fn view(pool: &PgPool, account_id: Uuid, id: Uuid) -> sqlx::Result<Option<ContactView>> {
    sqlx::query_as!(
        ContactView,
        r#"SELECT c.id, c.email, c.first_name, c.last_name,
                  c.fields AS "fields!: Json<Fields>",
                  c.status AS "status!: ContactStatus", c.source AS "source!: ContactSource",
                  COALESCE((SELECT json_agg(json_build_object('id', t.id, 'name', t.name) ORDER BY lower(t.name))
                            FROM contact_tags ct JOIN tags t ON t.id = ct.tag_id
                            WHERE ct.contact_id = c.id), '[]'::json) AS "tags!: Json<Vec<TagRef>>",
                  EXISTS (SELECT 1 FROM suppressions s WHERE s.account_id = c.account_id AND s.address = c.email) AS "blocked!",
                  c.subscribed_at, c.unsubscribed_at, c.created_at, c.updated_at
           FROM contacts c WHERE c.account_id = $1 AND c.id = $2"#,
        account_id,
        id
    )
    .fetch_optional(pool)
    .await
}

#[derive(Debug, Default)]
pub struct ListFilter {
    pub status: Option<ContactStatus>,
    pub tag: Option<Uuid>,
    /// Matches email, first name, last name or "first last", ignoring case; `%` and `_` are literal.
    pub q: Option<String>,
}

pub async fn list(
    pool: &PgPool,
    account_id: Uuid,
    f: &ListFilter,
    after: Option<Uuid>,
    limit: i64,
) -> sqlx::Result<Vec<ContactView>> {
    let pattern =
        f.q.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(crate::emails::like_pattern);
    sqlx::query_as!(
        ContactView,
        r#"SELECT c.id, c.email, c.first_name, c.last_name,
                  c.fields AS "fields!: Json<Fields>",
                  c.status AS "status!: ContactStatus", c.source AS "source!: ContactSource",
                  COALESCE((SELECT json_agg(json_build_object('id', t.id, 'name', t.name) ORDER BY lower(t.name))
                            FROM contact_tags ct JOIN tags t ON t.id = ct.tag_id
                            WHERE ct.contact_id = c.id), '[]'::json) AS "tags!: Json<Vec<TagRef>>",
                  EXISTS (SELECT 1 FROM suppressions s WHERE s.account_id = c.account_id AND s.address = c.email) AS "blocked!",
                  c.subscribed_at, c.unsubscribed_at, c.created_at, c.updated_at
           FROM contacts c
           WHERE c.account_id = $1
             AND ($2::contact_status IS NULL OR c.status = $2)
             AND ($3::uuid IS NULL OR EXISTS (SELECT 1 FROM contact_tags x WHERE x.contact_id = c.id AND x.tag_id = $3))
             AND ($4::text IS NULL OR c.email ILIKE $4 OR c.first_name ILIKE $4 OR c.last_name ILIKE $4
                  OR concat_ws(' ', c.first_name, c.last_name) ILIKE $4)
             AND ($5::uuid IS NULL OR c.id < $5)
           ORDER BY c.id DESC LIMIT $6"#,
        account_id,
        f.status as Option<ContactStatus>,
        f.tag,
        pattern,
        after,
        limit
    )
    .fetch_all(pool)
    .await
}

impl ContactStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ContactStatus::Pending => "pending",
            ContactStatus::Subscribed => "subscribed",
            ContactStatus::Unsubscribed => "unsubscribed",
        }
    }
}

pub struct ExportRow {
    pub id: Uuid,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub status: ContactStatus,
    pub tags: Vec<String>,
    pub fields: Json<Fields>,
    pub created_at: DateTime<Utc>,
}

/// Every field name used by any contact of the account, sorted: one CSV column each.
pub async fn field_keys(pool: &PgPool, account_id: Uuid) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar!(
        r#"SELECT DISTINCT k AS "k!" FROM contacts, jsonb_object_keys(fields) AS k WHERE account_id = $1 ORDER BY 1"#,
        account_id
    )
    .fetch_all(pool)
    .await
}

/// One export page, oldest first, after `after`.
pub async fn export_page(
    pool: &PgPool,
    account_id: Uuid,
    after: Option<Uuid>,
    limit: i64,
) -> sqlx::Result<Vec<ExportRow>> {
    sqlx::query_as!(
        ExportRow,
        r#"SELECT c.id, c.email, c.first_name, c.last_name, c.status AS "status!: ContactStatus",
                  ARRAY(SELECT t.name FROM contact_tags ct JOIN tags t ON t.id = ct.tag_id
                        WHERE ct.contact_id = c.id ORDER BY lower(t.name)) AS "tags!",
                  c.fields AS "fields!: Json<Fields>", c.created_at
           FROM contacts c
           WHERE c.account_id = $1 AND ($2::uuid IS NULL OR c.id > $2)
           ORDER BY c.id LIMIT $3"#,
        account_id,
        after,
        limit
    )
    .fetch_all(pool)
    .await
}

const FORMULA_START: [char; 6] = ['=', '+', '-', '@', '\t', '\r'];

/// OWASP CSV injection: a cell a spreadsheet would run as a formula gets a leading `'`.
fn defuse(cell: String) -> String {
    if cell.starts_with(FORMULA_START) {
        format!("'{cell}")
    } else {
        cell
    }
}

/// Undoes `defuse` on import, so an export imports back unchanged.
pub fn undefuse(cell: &str) -> &str {
    match cell.strip_prefix('\'') {
        Some(rest) if rest.starts_with(FORMULA_START) => rest,
        _ => cell,
    }
}

pub fn export_csv(
    keys: &[String],
    rows: &[ExportRow],
    header: bool,
) -> Result<Vec<u8>, csv::Error> {
    let mut w = csv::Writer::from_writer(Vec::new());
    if header {
        let mut h: Vec<&str> = vec![
            "email",
            "first_name",
            "last_name",
            "status",
            "tags",
            "created_at",
        ];
        h.extend(keys.iter().map(String::as_str));
        w.write_record(&h)?;
    }
    for r in rows {
        let mut rec: Vec<String> = vec![
            r.email.clone(),
            r.first_name.clone().unwrap_or_default(),
            r.last_name.clone().unwrap_or_default(),
            r.status.as_str().to_string(),
            r.tags.join(";"),
            r.created_at.to_rfc3339(),
        ];
        rec.extend(
            keys.iter()
                .map(|k| r.fields.0.get(k).cloned().unwrap_or_default()),
        );
        w.write_record(rec.into_iter().map(defuse))?;
    }
    w.into_inner().map_err(|e| e.into_error().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_csv_quotes_and_orders_columns() {
        let row = ExportRow {
            id: Uuid::nil(),
            email: "a@x.com".into(),
            first_name: Some("Lopez, \"Ana\"".into()),
            last_name: None,
            status: ContactStatus::Unsubscribed,
            tags: vec!["a".into(), "b".into()],
            fields: Json([("z".to_string(), "line\nbreak".to_string())].into()),
            created_at: DateTime::parse_from_rfc3339("2026-10-13T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        };
        let out = String::from_utf8(export_csv(&["y".into(), "z".into()], &[row], true).unwrap())
            .unwrap();
        assert_eq!(
            out,
            "email,first_name,last_name,status,tags,created_at,y,z\n\
             a@x.com,\"Lopez, \"\"Ana\"\"\",,unsubscribed,a;b,2026-10-13T10:00:00+00:00,,\"line\nbreak\"\n"
        );
    }

    #[test]
    fn export_csv_defuses_formulas() {
        let row = ExportRow {
            id: Uuid::nil(),
            email: "a@x.com".into(),
            first_name: Some("=HYPERLINK(\"http://evil\")".into()),
            last_name: Some("+1".into()),
            status: ContactStatus::Subscribed,
            tags: vec!["@vip".into()],
            fields: Json(
                [
                    ("z".to_string(), "-2".to_string()),
                    ("t".to_string(), "\tx".to_string()),
                ]
                .into(),
            ),
            created_at: DateTime::parse_from_rfc3339("2026-10-13T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        };
        let out = String::from_utf8(export_csv(&["t".into(), "z".into()], &[row], false).unwrap())
            .unwrap();
        assert_eq!(
            out,
            "a@x.com,\"'=HYPERLINK(\"\"http://evil\"\")\",'+1,subscribed,'@vip,2026-10-13T10:00:00+00:00,'\tx,'-2\n"
        );
    }

    #[test]
    fn field_keys() {
        assert!(valid_field_key("plan"));
        assert!(valid_field_key("signup_date_2"));
        assert!(!valid_field_key(""));
        assert!(!valid_field_key("Plan"));
        assert!(!valid_field_key("a-b"));
        assert!(!valid_field_key(&"a".repeat(41)));
    }

    #[test]
    fn fields_drop_empty_values_and_enforce_limits() {
        let f: Fields = [("plan".into(), "pro".into()), ("note".into(), "  ".into())].into();
        assert_eq!(clean_fields(f).unwrap().len(), 1);
        let many: Fields = (0..51).map(|i| (format!("k{i}"), "v".into())).collect();
        assert!(clean_fields(many).is_err());
        let long: Fields = [("k".into(), "x".repeat(501))].into();
        assert!(clean_fields(long).is_err());
        let bad: Fields = [("Bad Key".into(), "v".into())].into();
        assert!(clean_fields(bad).is_err());
    }

    #[test]
    fn input_is_normalized() {
        let c = ContactInput {
            email: "  Ana@Shop.COM ".into(),
            first_name: Some("  Ana ".into()),
            last_name: Some("   ".into()),
            fields: Fields::new(),
            tags: vec!["News".into(), " news ".into(), "VIP".into()],
        }
        .validate()
        .unwrap();
        assert_eq!(c.email, "ana@shop.com");
        assert_eq!(c.first_name.as_deref(), Some("Ana"));
        assert_eq!(c.last_name, None);
        assert_eq!(c.tags, vec!["News".to_string(), "VIP".to_string()]);
    }

    #[test]
    fn bad_input_is_refused() {
        let base = || ContactInput {
            email: "a@b.com".into(),
            ..Default::default()
        };
        assert!(
            ContactInput {
                email: "nope".into(),
                ..base()
            }
            .validate()
            .is_err()
        );
        assert!(
            ContactInput {
                first_name: Some("x".repeat(101)),
                ..base()
            }
            .validate()
            .is_err()
        );
        assert!(
            ContactInput {
                tags: vec!["x".repeat(51)],
                ..base()
            }
            .validate()
            .is_err()
        );
    }
}
