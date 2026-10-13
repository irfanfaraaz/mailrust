mod common;

use axum::{body::Body, http::{Request, StatusCode}};
use serde_json::json;

const BOUNDARY: &str = "XmailrustBoundaryX";
const TRUE: &[u8] = b"true";

/// A multipart upload to /contacts/imports with these parts (a part named "file" gets a filename).
fn upload(key: &str, parts: &[(&str, &[u8])]) -> Request<Body> {
    let mut body = Vec::new();
    for (name, data) in parts {
        let filename = if *name == "file" { "; filename=\"contacts.csv\"" } else { "" };
        body.extend_from_slice(format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"{filename}\r\n\r\n").as_bytes());
        body.extend_from_slice(data);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    Request::builder()
        .method("POST")
        .uri("/contacts/imports")
        .header("authorization", format!("Bearer {key}"))
        .header("content-type", format!("multipart/form-data; boundary={BOUNDARY}"))
        .body(Body::from(body))
        .unwrap()
}

#[sqlx::test]
async fn a_good_file_is_queued(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let (s, b) = common::call(&t.router, upload(&t.key, &[("file", &b"email\na@x.com\nb@x.com\n"[..]), ("consent", TRUE)])).await;
    assert_eq!(s, StatusCode::CREATED, "{b}");
    assert_eq!(b["status"], "queued");
    assert_eq!(b["rows_total"], 2);
    let id = b["id"].as_str().unwrap();
    let (s, one) = common::call(&t.router, common::req("GET", &format!("/contacts/imports/{id}"), Some(&t.key), None)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(one["id"], b["id"]);
    let (_, list) = common::call(&t.router, common::req("GET", "/contacts/imports", Some(&t.key), None)).await;
    assert_eq!(list["data"][0]["id"], b["id"]);

    let other = mailrust::accounts::create(&pool, "Other").await.unwrap();
    let key = mailrust::keys::create(&pool, other.id, "k", mailrust::keys::KeyAccess::Full).await.unwrap().token;
    let (s, _) = common::call(&t.router, common::req("GET", &format!("/contacts/imports/{id}"), Some(&key), None)).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn bad_uploads_are_refused_and_nothing_is_saved(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let ok_file: &[u8] = b"email\na@x.com\n";
    let cases: Vec<(Vec<(&str, &[u8])>, StatusCode, &str)> = vec![
        (vec![("file", ok_file)], StatusCode::UNPROCESSABLE_ENTITY, "consent_required"),
        (vec![("file", ok_file), ("consent", &b"false"[..])], StatusCode::UNPROCESSABLE_ENTITY, "consent_required"),
        (vec![("consent", TRUE)], StatusCode::BAD_REQUEST, "validation_error"),
        (vec![("file", &b"name\nAna\n"[..]), ("consent", TRUE)], StatusCode::UNPROCESSABLE_ENTITY, "no_email_column"),
        (vec![("file", ok_file), ("consent", TRUE), ("tag_id", &b"0190f5a0-0000-7000-8000-000000000000"[..])], StatusCode::BAD_REQUEST, "validation_error"),
    ];
    for (parts, status, code) in cases {
        let (s, b) = common::call(&t.router, upload(&t.key, &parts)).await;
        assert_eq!((s, b["error"]["code"].as_str().unwrap()), (status, code), "{b}");
    }
    let big = vec![b' '; mailrust::imports::MAX_FILE_BYTES + 1];
    let (s, b) = common::call(&t.router, upload(&t.key, &[("file", &big[..]), ("consent", TRUE)])).await;
    assert_eq!((s, b["error"]["code"].clone()), (StatusCode::UNPROCESSABLE_ENTITY, json!("file_too_large")));
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM contact_imports").fetch_one(&pool).await.unwrap();
    assert_eq!(n, 0);
}
