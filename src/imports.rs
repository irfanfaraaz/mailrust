//! CSV contact imports: checked on upload, then processed in the background (Task 10).
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, types::Json};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    api::AppState,
    contacts::{self, ContactInput, ContactSource, Fields, MAX_FIELD_KEY},
};

pub const MAX_FILE_BYTES: usize = 20 * 1024 * 1024;
pub const MAX_ROWS: usize = 100_000;
pub const MAX_ERRORS: usize = 1_000;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize, utoipa::ToSchema,
)]
#[sqlx(type_name = "import_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ImportStatus {
    Queued,
    Running,
    Done,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ImportRowError {
    /// Line in the file (1 = the header row).
    pub line: u64,
    pub reason: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ImportView {
    pub id: Uuid,
    pub status: ImportStatus,
    pub tag_id: Option<Uuid>,
    pub rows_total: i32,
    pub rows_done: i32,
    pub added: i32,
    pub updated: i32,
    pub skipped: i32,
    /// The first 1,000 bad rows. Empty in the import list: get one import to see them.
    #[schema(value_type = Vec<ImportRowError>)]
    pub errors: Json<Vec<ImportRowError>>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

pub fn strip_bom(b: &[u8]) -> &[u8] {
    b.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(b)
}

fn reader(bytes: &[u8]) -> csv::Reader<&[u8]> {
    csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(strip_bom(bytes))
}

/// "Signup Date (UTC)" → "signup_date_utc": lower-case, runs of other characters become one `_`, ≤ 40 chars.
pub fn field_key(header: &str) -> Option<String> {
    let mut out = String::new();
    for c in header.trim().to_lowercase().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
        }
    }
    let out: String = out
        .trim_end_matches('_')
        .chars()
        .take(MAX_FIELD_KEY)
        .collect();
    let out = out.trim_end_matches('_').to_string();
    (!out.is_empty()).then_some(out)
}

#[derive(Debug, Default, PartialEq)]
pub struct Columns {
    pub email: usize,
    pub first_name: Option<usize>,
    pub last_name: Option<usize>,
    pub name: Option<usize>,
    pub tags: Option<usize>,
    pub fields: Vec<(usize, String)>,
}

pub fn columns(headers: &csv::StringRecord) -> Result<Columns, String> {
    let mut c = Columns::default();
    let mut email = None;
    for (i, h) in headers.iter().enumerate() {
        let Some(k) = field_key(h) else { continue };
        match k.as_str() {
            "email" if email.is_none() => email = Some(i),
            // Fixed export columns Mailrust sets itself (and a second email column): skipped, never a field.
            "email" | "status" | "created_at" => {}
            "first_name" => c.first_name = Some(i),
            "last_name" => c.last_name = Some(i),
            "name" => c.name = Some(i),
            "tags" => c.tags = Some(i),
            _ => {
                c.fields.retain(|(_, x)| *x != k); // a repeated header: the last one wins
                c.fields.push((i, k));
            }
        }
    }
    c.email = email.ok_or("the first row must name an email column")?;
    Ok(c)
}

/// Upload checks. The data-row count, or a 422 code and message.
pub fn check_file(bytes: &[u8]) -> Result<i32, (&'static str, String)> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err((
            "file_too_large",
            "the file is larger than 20 MB; split it and import the parts".into(),
        ));
    }
    if std::str::from_utf8(strip_bom(bytes)).is_err() {
        return Err((
            "not_utf8",
            "save the file as CSV UTF-8 and try again".into(),
        ));
    }
    let mut r = reader(bytes);
    let headers = r.headers().map_err(|e| ("bad_csv", e.to_string()))?.clone();
    if headers.iter().all(|h| h.trim().is_empty()) {
        return Err(("empty_file", "the file is empty".into()));
    }
    columns(&headers).map_err(|m| ("no_email_column", m))?;
    let mut n = 0usize;
    for _ in r.records() {
        n += 1; // a broken row still counts: it is reported, not fatal
        if n > MAX_ROWS {
            return Err((
                "too_many_rows",
                format!("a file holds at most {MAX_ROWS} rows"),
            ));
        }
    }
    if n == 0 {
        return Err((
            "empty_file",
            "the file has a header row but no contacts".into(),
        ));
    }
    Ok(n as i32)
}

pub struct Row {
    pub line: u64,
    pub input: Result<ContactInput, String>,
}

pub fn parse_all(bytes: &[u8]) -> Result<Vec<Row>, String> {
    let mut r = reader(bytes);
    let cols = columns(&r.headers().map_err(|e| e.to_string())?.clone())?;
    let mut lines = LineCounter {
        bytes: strip_bom(bytes),
        at: 0,
        line: 1,
    };
    Ok(r.records()
        .map(|rec| match rec {
            Ok(rec) => Row {
                line: lines.of(rec.position()),
                input: parse_row(&cols, &rec),
            },
            Err(e) => Row {
                line: lines.of(e.position()),
                input: Err(e.to_string()),
            },
        })
        .collect())
}

/// The file line a record starts on. `csv`'s own `Position::line` is one short after a CRLF
/// (the `\n` is read as part of the next record), so count `\n`s up to the first byte of the record.
struct LineCounter<'a> {
    bytes: &'a [u8],
    at: usize,
    line: u64,
}

impl LineCounter<'_> {
    fn of(&mut self, pos: Option<&csv::Position>) -> u64 {
        let Some(pos) = pos else { return 0 };
        let mut start = usize::try_from(pos.byte())
            .unwrap_or(usize::MAX)
            .min(self.bytes.len());
        while start < self.bytes.len() && matches!(self.bytes[start], b'\r' | b'\n') {
            start += 1;
        }
        if start >= self.at {
            self.line += self.bytes[self.at..start]
                .iter()
                .filter(|&&b| b == b'\n')
                .count() as u64;
            self.at = start;
        }
        self.line
    }
}

pub fn parse_row(c: &Columns, rec: &csv::StringRecord) -> Result<ContactInput, String> {
    let get = |i: Option<usize>| {
        i.and_then(|i| rec.get(i))
            .map(|s| contacts::undefuse(s.trim()))
            .filter(|s| !s.is_empty())
    };
    let email = get(Some(c.email)).ok_or("the email is empty")?;
    let (mut first, mut last) = (
        get(c.first_name).map(String::from),
        get(c.last_name).map(String::from),
    );
    if first.is_none()
        && last.is_none()
        && let Some(full) = get(c.name)
    {
        let (f, l) = full
            .split_once(' ')
            .map_or((full, ""), |(f, l)| (f, l.trim()));
        first = Some(f.to_string());
        last = (!l.is_empty()).then(|| l.to_string());
    }
    let tags = get(c.tags)
        .map(|t| {
            t.split([';', ','])
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();
    let fields: Fields = c
        .fields
        .iter()
        .filter_map(|(i, k)| get(Some(*i)).map(|v| (k.clone(), v.to_string())))
        .collect();
    ContactInput {
        email: email.to_string(),
        first_name: first,
        last_name: last,
        fields,
        tags,
    }
    .validate()
}

pub async fn view(pool: &PgPool, account_id: Uuid, id: Uuid) -> sqlx::Result<Option<ImportView>> {
    sqlx::query_as!(
        ImportView,
        r#"SELECT id, status AS "status: ImportStatus", tag_id, rows_total, rows_done, added, updated, skipped,
                  errors AS "errors: Json<Vec<ImportRowError>>", error, created_at, finished_at
           FROM contact_imports WHERE account_id = $1 AND id = $2"#,
        account_id,
        id
    )
    .fetch_optional(pool)
    .await
}

pub async fn list(pool: &PgPool, account_id: Uuid) -> sqlx::Result<Vec<ImportView>> {
    sqlx::query_as!(
        ImportView,
        r#"SELECT id, status AS "status: ImportStatus", tag_id, rows_total, rows_done, added, updated, skipped,
                  '[]'::jsonb AS "errors!: Json<Vec<ImportRowError>>", error, created_at, finished_at
           FROM contact_imports WHERE account_id = $1 ORDER BY id DESC LIMIT 20"#,
        account_id
    )
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_become_field_keys() {
        assert_eq!(field_key(" Email ").as_deref(), Some("email"));
        assert_eq!(
            field_key("Signup Date (UTC)").as_deref(),
            Some("signup_date_utc")
        );
        assert_eq!(field_key("first_name").as_deref(), Some("first_name"));
        assert_eq!(field_key("  ###  "), None);
        assert_eq!(field_key(&"abcdefghij".repeat(5)).unwrap().len(), 40);
    }

    #[test]
    fn real_world_csv_parses() {
        let csv = "\u{FEFF}Email ,first_name,last_name,Tags,Signup Date (UTC)\r\n\
                   ana@x.com,\"Ana, Jr.\",Lopez,VIP;news,2024-01-01\r\n\
                   bad-email,Bob,,,\r\n\
                   \"bo@x.com\",\"Bo\nBo\",,,\r\n\
                   CY@X.COM,,,a,\r\n\
                   \r\n";
        assert_eq!(check_file(csv.as_bytes()), Ok(4));
        let rows = parse_all(csv.as_bytes()).unwrap();
        let lines: Vec<u64> = rows.iter().map(|r| r.line).collect();
        assert_eq!(
            lines,
            [2, 3, 4, 6],
            "line numbers point at the real line, quoted line breaks included"
        );

        let ana = rows[0].input.as_ref().unwrap();
        assert_eq!(ana.email, "ana@x.com");
        assert_eq!(ana.first_name.as_deref(), Some("Ana, Jr."));
        assert_eq!(ana.tags, ["VIP", "news"]);
        assert_eq!(ana.fields["signup_date_utc"], "2024-01-01");
        assert!(
            rows[1]
                .input
                .as_ref()
                .unwrap_err()
                .contains("not a valid email")
        );
        assert!(
            rows[2].input.as_ref().unwrap_err().contains("line breaks"),
            "a name cannot hold a line break"
        );
        assert_eq!(rows[3].input.as_ref().unwrap().email, "cy@x.com");
    }

    #[test]
    fn a_name_column_splits_on_the_first_space() {
        let rows = parse_all(b"email,name\na@x.com,Ana Maria Lopez\nb@x.com,Bo\n").unwrap();
        assert_eq!(
            rows.iter().map(|r| r.line).collect::<Vec<_>>(),
            [2, 3],
            "LF files number lines too"
        );
        let a = rows[0].input.as_ref().unwrap();
        assert_eq!(
            (a.first_name.as_deref(), a.last_name.as_deref()),
            (Some("Ana"), Some("Maria Lopez"))
        );
        let b = rows[1].input.as_ref().unwrap();
        assert_eq!(
            (b.first_name.as_deref(), b.last_name.as_deref()),
            (Some("Bo"), None)
        );
    }

    #[test]
    fn rows_with_too_many_or_too_long_fields_are_bad() {
        let header: Vec<String> = std::iter::once("email".to_string())
            .chain((0..51).map(|i| format!("f{i}")))
            .collect();
        let row: Vec<String> = std::iter::once("a@x.com".to_string())
            .chain((0..51).map(|_| "v".to_string()))
            .collect();
        let csv = format!(
            "{}\n{}\nb@x.com,{}\n",
            header.join(","),
            row.join(","),
            "x".repeat(501)
        );
        let rows = parse_all(csv.as_bytes()).unwrap();
        assert!(rows[0].input.is_err());
        assert!(rows[1].input.is_err());
    }

    #[test]
    fn reserved_headers_never_become_fields() {
        let rows =
            parse_all(b"email,status,created_at,EMAIL,tags\na@x.com,vip,2024,b@x.com,t\n").unwrap();
        let a = rows[0].input.as_ref().unwrap();
        assert_eq!(a.email, "a@x.com");
        assert!(a.fields.is_empty(), "{:?}", a.fields);
        assert_eq!(a.tags, ["t"]);
    }

    #[test]
    fn uploads_are_checked() {
        assert_eq!(check_file(b"name\nAna\n").unwrap_err().0, "no_email_column");
        assert_eq!(
            check_file(b"email\n\xff\xfe@x.com\n").unwrap_err().0,
            "not_utf8"
        );
        assert_eq!(check_file(b"").unwrap_err().0, "empty_file");
        assert_eq!(check_file(b"email\n").unwrap_err().0, "empty_file");
        assert_eq!(
            check_file(&vec![b' '; MAX_FILE_BYTES + 1]).unwrap_err().0,
            "file_too_large"
        );
        let many = format!("email\n{}", "a@x.com\n".repeat(MAX_ROWS + 1));
        assert_eq!(check_file(many.as_bytes()).unwrap_err().0, "too_many_rows");
    }
}

pub const BATCH: usize = 1_000;
pub const LEASE_SECS: i32 = 300;
/// Claims in a row without progress before a job is marked failed: each claim after the first means the last run died.
pub const MAX_ATTEMPTS: i32 = 3;

#[derive(Debug, Clone)]
pub struct ClaimedImport {
    pub id: Uuid,
    pub account_id: Uuid,
    pub tag_id: Option<Uuid>,
    pub file: Vec<u8>,
    pub rows_done: i32,
    pub attempts: i32,
}

/// The oldest queued job, or a running one whose worker stopped renewing its lease.
/// SKIP LOCKED: two workers never take the same job.
pub async fn claim(pool: &PgPool) -> sqlx::Result<Option<ClaimedImport>> {
    sqlx::query_as!(
        ClaimedImport,
        r#"UPDATE contact_imports SET status = 'running', attempts = attempts + 1, lease_until = now() + make_interval(secs => $1)
           WHERE id = (SELECT id FROM contact_imports
                       WHERE status = 'queued' OR (status = 'running' AND lease_until < now())
                       ORDER BY created_at LIMIT 1 FOR UPDATE SKIP LOCKED)
           RETURNING id, account_id, tag_id, file AS "file!", rows_done, attempts"#,
        LEASE_SECS as f64
    )
    .fetch_optional(pool)
    .await
}

/// Processes up to `max_batches` batches from `rows_done`. Each batch is one transaction: the counters and
/// `rows_done` move with the contacts, so a crash never applies a batch twice.
pub async fn run_job(pool: &PgPool, job: ClaimedImport, max_batches: usize) -> anyhow::Result<()> {
    let parsed = if job.attempts > MAX_ATTEMPTS {
        tracing::warn!(import_id = %job.id, account_id = %job.account_id, "import given up");
        Err(format!(
            "the import stopped {MAX_ATTEMPTS} times without finishing; the first {} rows were imported",
            job.rows_done
        ))
    } else {
        parse_all(&job.file)
    };
    let rows = match parsed {
        Ok(rows) => rows,
        Err(e) => {
            sqlx::query!(
                "UPDATE contact_imports SET status = 'failed', error = $2, file = NULL, lease_until = NULL, finished_at = now() WHERE id = $1",
                job.id,
                e
            )
            .execute(pool)
            .await?;
            return Ok(());
        }
    };
    let mut done = job.rows_done as usize;
    for _ in 0..max_batches {
        if done >= rows.len() {
            break;
        }
        let batch = &rows[done..(done + BATCH).min(rows.len())];
        let mut tx = pool.begin().await?;
        // Still ours and still where we left it? A worker that lost its lease stops here.
        let Some(listed) = sqlx::query_scalar!(
            r#"UPDATE contact_imports SET lease_until = now() + make_interval(secs => $3)
               WHERE id = $1 AND status = 'running' AND rows_done = $2
               RETURNING jsonb_array_length(errors) AS "n!""#,
            job.id,
            done as i32,
            LEASE_SECS as f64
        )
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(());
        };
        let (mut added, mut updated, mut errors) = (0i32, 0i32, Vec::<ImportRowError>::new());
        for row in batch {
            match &row.input {
                Ok(input) => {
                    let (_, new) = contacts::save(
                        &mut tx,
                        job.account_id,
                        input,
                        ContactSource::Import,
                        job.tag_id,
                    )
                    .await?;
                    if new { added += 1 } else { updated += 1 }
                }
                Err(reason) => errors.push(ImportRowError {
                    line: row.line,
                    reason: reason.clone(),
                }),
            }
        }
        let skipped = errors.len() as i32;
        errors.truncate(MAX_ERRORS.saturating_sub(listed as usize));
        // attempts = 1: this run made progress, so only runs that die without any count toward MAX_ATTEMPTS.
        sqlx::query!(
            "UPDATE contact_imports SET rows_done = rows_done + $2, added = added + $3, updated = updated + $4,
                    skipped = skipped + $5, errors = errors || $6, attempts = 1
             WHERE id = $1",
            job.id,
            batch.len() as i32,
            added,
            updated,
            skipped,
            Json(&errors) as _
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        done += batch.len();
    }
    if done >= rows.len() {
        sqlx::query!(
            "UPDATE contact_imports SET status = 'done', file = NULL, lease_until = NULL, finished_at = now()
             WHERE id = $1 AND status = 'running' AND rows_done = $2",
            job.id,
            done as i32
        )
        .execute(pool)
        .await?;
        tracing::info!(import_id = %job.id, account_id = %job.account_id, "import done");
    }
    Ok(())
}

/// Claims and finishes one job. False when there was nothing to do.
pub async fn run_once(pool: &PgPool) -> anyhow::Result<bool> {
    let Some(job) = claim(pool).await? else {
        return Ok(false);
    };
    run_job(pool, job, usize::MAX).await?;
    Ok(true)
}

pub async fn worker_loop(state: AppState, shutdown: CancellationToken) -> anyhow::Result<()> {
    crate::background::every(Duration::from_secs(2), &shutdown, || async {
        loop {
            match run_once(&state.pool).await {
                Ok(true) => continue,
                Ok(false) => break,
                Err(e) => {
                    tracing::error!(error = ?e, "import batch failed; the job resumes when its lease runs out");
                    break;
                }
            }
        }
    })
    .await;
    Ok(())
}
