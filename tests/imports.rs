mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use mailrust::imports;
use serde_json::json;

const BOUNDARY: &str = "XmailrustBoundaryX";
const TRUE: &[u8] = b"true";

/// Upload parts, the status expected back, and the error code expected.
type Case<'a> = (Vec<(&'a str, &'a [u8])>, StatusCode, &'a str);

/// A multipart upload to /contacts/imports with these parts (a part named "file" gets a filename).
fn upload(key: &str, parts: &[(&str, &[u8])]) -> Request<Body> {
    let mut body = Vec::new();
    for (name, data) in parts {
        let filename = if *name == "file" {
            "; filename=\"contacts.csv\""
        } else {
            ""
        };
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"{filename}\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(data);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    Request::builder()
        .method("POST")
        .uri("/contacts/imports")
        .header("authorization", format!("Bearer {key}"))
        .header(
            "content-type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .body(Body::from(body))
        .unwrap()
}

#[sqlx::test]
async fn a_good_file_is_queued(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let (s, b) = common::call(
        &t.router,
        upload(
            &t.key,
            &[
                ("file", &b"email\na@x.com\nb@x.com\n"[..]),
                ("consent", TRUE),
            ],
        ),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{b}");
    assert_eq!(b["status"], "queued");
    assert_eq!(b["rows_total"], 2);
    let id = b["id"].as_str().unwrap();
    let (s, one) = common::call(
        &t.router,
        common::req(
            "GET",
            &format!("/contacts/imports/{id}"),
            Some(&t.key),
            None,
        ),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(one["id"], b["id"]);
    let (_, list) = common::call(
        &t.router,
        common::req("GET", "/contacts/imports", Some(&t.key), None),
    )
    .await;
    assert_eq!(list["data"][0]["id"], b["id"]);

    let other = mailrust::accounts::create(&pool, "Other").await.unwrap();
    let key = mailrust::keys::create(&pool, other.id, "k", mailrust::keys::KeyAccess::Full)
        .await
        .unwrap()
        .token;
    let (s, _) = common::call(
        &t.router,
        common::req("GET", &format!("/contacts/imports/{id}"), Some(&key), None),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn bad_uploads_are_refused_and_nothing_is_saved(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let ok_file: &[u8] = b"email\na@x.com\n";
    let cases: Vec<Case> = vec![
        (
            vec![("file", ok_file)],
            StatusCode::UNPROCESSABLE_ENTITY,
            "consent_required",
        ),
        (
            vec![("file", ok_file), ("consent", &b"false"[..])],
            StatusCode::UNPROCESSABLE_ENTITY,
            "consent_required",
        ),
        (
            vec![("consent", TRUE)],
            StatusCode::BAD_REQUEST,
            "validation_error",
        ),
        (
            vec![("file", &b"name\nAna\n"[..]), ("consent", TRUE)],
            StatusCode::UNPROCESSABLE_ENTITY,
            "no_email_column",
        ),
        (
            vec![
                ("file", ok_file),
                ("consent", TRUE),
                ("tag_id", &b"0190f5a0-0000-7000-8000-000000000000"[..]),
            ],
            StatusCode::BAD_REQUEST,
            "validation_error",
        ),
    ];
    for (parts, status, code) in cases {
        let (s, b) = common::call(&t.router, upload(&t.key, &parts)).await;
        assert_eq!(
            (s, b["error"]["code"].as_str().unwrap()),
            (status, code),
            "{b}"
        );
    }
    let big = vec![b' '; mailrust::imports::MAX_FILE_BYTES + 1];
    let (s, b) = common::call(
        &t.router,
        upload(&t.key, &[("file", &big[..]), ("consent", TRUE)]),
    )
    .await;
    assert_eq!(
        (s, b["error"]["code"].clone()),
        (StatusCode::UNPROCESSABLE_ENTITY, json!("file_too_large"))
    );
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM contact_imports")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0);
}

#[sqlx::test]
async fn ten_thousand_rows_import_and_resume_after_a_crash(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let api = |m: &str, uri: &str, body: Option<serde_json::Value>| {
        common::req(m, uri, Some(&t.key), body)
    };
    // An existing unsubscribed contact, a suppressed address and a tag for every row.
    let (_, keep) = common::call(
        &t.router,
        api("POST", "/contacts", Some(json!({"email": "keep@x.com"}))),
    )
    .await;
    let keep_uri = format!("/contacts/{}", keep["id"].as_str().unwrap());
    common::call(
        &t.router,
        api("PATCH", &keep_uri, Some(json!({"status": "unsubscribed"}))),
    )
    .await;
    common::call(
        &t.router,
        api(
            "POST",
            "/suppressions",
            Some(json!({"address": "r0@x.com"})),
        ),
    )
    .await;
    let (_, tag) = common::call(
        &t.router,
        api("POST", "/tags", Some(json!({"name": "Imported"}))),
    )
    .await;

    let mut csv = String::from("email,first_name,plan\n");
    for i in 0..9_998 {
        csv.push_str(&format!("r{i}@x.com,R{i},free\n"));
    }
    csv.push_str("KEEP@X.com,Keeper,pro\nnot-an-email,Bad,free\n");
    let (s, b) = common::call(
        &t.router,
        upload(
            &t.key,
            &[
                ("file", csv.as_bytes()),
                ("consent", TRUE),
                ("tag_id", tag["id"].as_str().unwrap().as_bytes()),
            ],
        ),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{b}");
    assert_eq!(b["rows_total"], 10_000);
    let id = b["id"].as_str().unwrap().to_string();

    // One batch, then the worker "dies": its lease runs out and another worker resumes.
    let job = imports::claim(&pool).await.unwrap().expect("claimed");
    assert!(
        imports::claim(&pool).await.unwrap().is_none(),
        "a leased job is not claimed twice"
    );
    let stale = job.clone();
    imports::run_job(&pool, job, 1).await.unwrap();
    let done: i32 = sqlx::query_scalar("SELECT rows_done FROM contact_imports")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(done, 1_000);
    sqlx::query("UPDATE contact_imports SET lease_until = now() - interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(imports::run_once(&pool).await.unwrap());
    // The old worker wakes up late: it must not apply anything twice.
    imports::run_job(&pool, stale, usize::MAX).await.unwrap();

    let (_, v) = common::call(
        &t.router,
        api("GET", &format!("/contacts/imports/{id}"), None),
    )
    .await;
    assert_eq!(v["status"], "done");
    assert_eq!(
        (
            v["rows_done"].as_i64(),
            v["added"].as_i64(),
            v["updated"].as_i64(),
            v["skipped"].as_i64()
        ),
        (Some(10_000), Some(9_998), Some(1), Some(1)),
        "{v}"
    );
    assert_eq!(
        v["errors"],
        json!([{"line": 10_001, "reason": "not-an-email is not a valid email address"}])
    );
    assert!(v["finished_at"].is_string());
    let file_gone: bool = sqlx::query_scalar("SELECT file IS NULL FROM contact_imports")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(file_gone);

    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM contacts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 9_999, "no duplicates after the resume");
    let tagged: i64 = sqlx::query_scalar("SELECT count(*) FROM contact_tags")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(tagged, 9_999);
    let (_, keep) = common::call(&t.router, api("GET", &keep_uri, None)).await;
    assert_eq!(
        keep["status"], "unsubscribed",
        "a re-import never re-subscribes"
    );
    assert_eq!(keep["first_name"], "Keeper");
    let (_, r0) = common::call(&t.router, api("GET", "/contacts?q=r0%40x.com", None)).await;
    assert_eq!(r0["data"][0]["blocked"], true);
    assert_eq!(r0["data"][0]["source"], "import");
}

#[sqlx::test]
async fn nothing_to_do_is_not_an_error(pool: sqlx::PgPool) {
    assert!(!imports::run_once(&pool).await.unwrap());
}

#[sqlx::test]
async fn a_nul_byte_row_is_skipped_and_the_rest_imports(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let csv = b"email,first_name,plan\na@x.com,A\0na,free\nb@x.com,Bo,pro\0\nc@x.com,Cy,free\n";
    let (s, b) = common::call(
        &t.router,
        upload(&t.key, &[("file", &csv[..]), ("consent", TRUE)]),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{b}");
    let id = b["id"].as_str().unwrap().to_string();
    assert!(imports::run_once(&pool).await.unwrap());
    let (_, v) = common::call(
        &t.router,
        common::req(
            "GET",
            &format!("/contacts/imports/{id}"),
            Some(&t.key),
            None,
        ),
    )
    .await;
    assert_eq!(v["status"], "done", "{v}");
    assert_eq!(
        (v["added"].as_i64(), v["skipped"].as_i64()),
        (Some(1), Some(2)),
        "{v}"
    );
    assert_eq!(v["errors"][0]["line"], 2);
    assert_eq!(v["errors"][1]["line"], 3);
}

#[sqlx::test]
async fn a_job_that_keeps_failing_is_given_up(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let (s, b) = common::call(
        &t.router,
        upload(
            &t.key,
            &[("file", &b"email\na@x.com\n"[..]), ("consent", TRUE)],
        ),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{b}");
    // Every try so far died mid-batch: the lease ran out each time.
    for _ in 0..imports::MAX_ATTEMPTS {
        imports::claim(&pool).await.unwrap().expect("claimed");
        sqlx::query("UPDATE contact_imports SET lease_until = now() - interval '1 second'")
            .execute(&pool)
            .await
            .unwrap();
    }
    assert!(imports::run_once(&pool).await.unwrap());
    let (status, error, file_gone, finished): (String, Option<String>, bool, bool) = sqlx::query_as(
        "SELECT status::text, error, file IS NULL, finished_at IS NOT NULL FROM contact_imports",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status, "failed");
    assert!(error.is_some());
    assert!(file_gone && finished);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM contacts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0, "a given-up job does not run again");
}

#[sqlx::test]
async fn an_export_imports_back_unchanged(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let fields = || async {
        sqlx::query_scalar::<_, serde_json::Value>("SELECT fields FROM contacts")
            .fetch_one(&pool)
            .await
            .unwrap()
    };
    let csv = b"email,phone,delta,note,handle\na@x.com,+1 555 0100,-5,=1+1,@ana\n";
    let (s, b) = common::call(
        &t.router,
        upload(&t.key, &[("file", &csv[..]), ("consent", TRUE)]),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{b}");
    assert!(imports::run_once(&pool).await.unwrap());
    let before = fields().await;
    assert_eq!(before["phone"], "+1 555 0100");

    let (_, _, export) = common::call_raw(
        &t.router,
        common::req("GET", "/contacts/export", Some(&t.key), None),
    )
    .await;
    assert!(
        export.contains(",'+1 555 0100"),
        "the export stays defused: {export}"
    );
    let (s, b) = common::call(
        &t.router,
        upload(&t.key, &[("file", export.as_bytes()), ("consent", TRUE)]),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{b}");
    assert!(imports::run_once(&pool).await.unwrap());
    assert_eq!(
        fields().await,
        before,
        "import → export → import round-trips"
    );
}

#[sqlx::test]
async fn a_slow_job_that_keeps_making_progress_is_not_given_up(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let mut csv = String::from("email\n");
    let rows = (imports::MAX_ATTEMPTS as usize + 2) * imports::BATCH;
    for i in 0..rows {
        csv.push_str(&format!("r{i}@x.com\n"));
    }
    let (s, b) = common::call(
        &t.router,
        upload(&t.key, &[("file", csv.as_bytes()), ("consent", TRUE)]),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{b}");
    // Each run gets one batch done, then its lease runs out before it finishes.
    for _ in 0..=imports::MAX_ATTEMPTS {
        let job = imports::claim(&pool).await.unwrap().expect("claimed");
        imports::run_job(&pool, job, 1).await.unwrap();
        sqlx::query("UPDATE contact_imports SET lease_until = now() - interval '1 second'")
            .execute(&pool)
            .await
            .unwrap();
    }
    assert!(imports::run_once(&pool).await.unwrap());
    let (status, done): (String, i32) =
        sqlx::query_as("SELECT status::text, rows_done FROM contact_imports")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((status.as_str(), done as usize), ("done", rows));
}

#[sqlx::test]
async fn the_import_list_leaves_out_row_errors(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let (_, b) = common::call(
        &t.router,
        upload(
            &t.key,
            &[("file", &b"email\nbad\na@x.com\n"[..]), ("consent", TRUE)],
        ),
    )
    .await;
    assert!(imports::run_once(&pool).await.unwrap());
    let get = |uri: String| common::req("GET", &uri, Some(&t.key), None);
    let (_, list) = common::call(&t.router, get("/contacts/imports".into())).await;
    assert_eq!(list["data"][0]["errors"], json!([]), "{list}");
    assert_eq!(list["data"][0]["skipped"], 1);
    let (_, one) = common::call(
        &t.router,
        get(format!("/contacts/imports/{}", b["id"].as_str().unwrap())),
    )
    .await;
    assert_eq!(one["errors"][0]["line"], 2, "{one}");
}
