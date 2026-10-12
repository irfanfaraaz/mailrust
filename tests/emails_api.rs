mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use mailrust::{
    accounts,
    keys::{self, KeyAccess},
};
use serde_json::{Value, json};

fn email(to: &str) -> Value {
    json!({ "from": "Shop <hi@shop.com>", "to": to, "subject": "Receipt", "html": "<p>Thanks</p>" })
}

async fn count(pool: &sqlx::PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM emails")
        .fetch_one(pool)
        .await
        .unwrap()
}

fn with_key_header(key: &str, idem: &str, body: &Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/emails")
        .header("authorization", format!("Bearer {key}"))
        .header("content-type", "application/json")
        .header("idempotency-key", idem)
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[sqlx::test]
async fn send_queues_an_email(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    common::verified_domain(&t, "shop.com").await;
    let (s, b) = common::call(
        &t.router,
        common::req("POST", "/emails", Some(&t.key), Some(email("a@x.com"))),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    let id = b["id"].as_str().unwrap();
    let (s, v) = common::call(
        &t.router,
        common::req("GET", &format!("/emails/{id}"), Some(&t.key), None),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["status"], "queued");
    assert_eq!(v["to"], json!(["a@x.com"]));
}

#[sqlx::test]
async fn unknown_or_unverified_domains_are_refused(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let (s, b) = common::call(
        &t.router,
        common::req("POST", "/emails", Some(&t.key), Some(email("a@x.com"))),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(b["error"]["code"], "domain_not_found");

    let d = common::verified_domain(&t, "shop.com").await;
    sqlx::query("UPDATE domains SET status = 'pending' WHERE id = $1")
        .bind(d.id)
        .execute(&t.state.pool)
        .await
        .unwrap();
    let (s, b) = common::call(
        &t.router,
        common::req("POST", "/emails", Some(&t.key), Some(email("a@x.com"))),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(b["error"]["code"], "domain_not_verified");
}

#[sqlx::test]
async fn idempotency_same_body_same_id_different_body_conflict(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    common::verified_domain(&t, "shop.com").await;
    let (_, a) = common::call(
        &t.router,
        with_key_header(&t.key, "order-1", &email("a@x.com")),
    )
    .await;
    let (_, b) = common::call(
        &t.router,
        with_key_header(&t.key, "order-1", &email("a@x.com")),
    )
    .await;
    assert_eq!(a["id"], b["id"]);
    let (s, c) = common::call(
        &t.router,
        with_key_header(&t.key, "order-1", &email("other@x.com")),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_eq!(c["error"]["code"], "idempotency_conflict");
    assert_eq!(count(&t.state.pool).await, 1);
}

#[sqlx::test]
async fn concurrent_same_key_creates_one_email(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    common::verified_domain(&t, "shop.com").await;
    let body = email("a@x.com");
    let calls = (0..5).map(|_| common::call(&t.router, with_key_header(&t.key, "race", &body)));
    let results = futures::future::join_all(calls).await;
    for (s, b) in &results {
        assert!(
            *s == StatusCode::OK || *s == StatusCode::CONFLICT,
            "{s} {b}"
        );
    }
    assert_eq!(
        count(&t.state.pool).await,
        1,
        "never two emails for one key"
    );
}

#[sqlx::test]
async fn batch_is_all_or_nothing(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    common::verified_domain(&t, "shop.com").await;
    let good = json!([email("a@x.com"), email("b@x.com"), email("c@x.com")]);
    let (s, b) = common::call(
        &t.router,
        common::req("POST", "/emails/batch", Some(&t.key), Some(good)),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(b["data"].as_array().unwrap().len(), 3);

    let bad = json!([email("d@x.com"), { "from": "hi@shop.com", "to": "e@x.com", "subject": "" , "text": "x"}]);
    let (s, b) = common::call(
        &t.router,
        common::req("POST", "/emails/batch", Some(&t.key), Some(bad)),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert!(
        b["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("emails[1]")
    );
    assert_eq!(count(&t.state.pool).await, 3);

    let too_many: Vec<Value> = (0..101).map(|i| email(&format!("u{i}@x.com"))).collect();
    let (s, _) = common::call(
        &t.router,
        common::req("POST", "/emails/batch", Some(&t.key), Some(json!(too_many))),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn scheduled_emails_show_and_can_be_canceled_once(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    common::verified_domain(&t, "shop.com").await;
    let mut e = email("a@x.com");
    e["scheduled_at"] = json!((chrono::Utc::now() + chrono::Duration::hours(2)).to_rfc3339());
    let (_, b) = common::call(
        &t.router,
        common::req("POST", "/emails", Some(&t.key), Some(e)),
    )
    .await;
    let id = b["id"].as_str().unwrap().to_string();
    let (_, v) = common::call(
        &t.router,
        common::req("GET", &format!("/emails/{id}"), Some(&t.key), None),
    )
    .await;
    assert_eq!(v["status"], "scheduled");
    let (s, _) = common::call(
        &t.router,
        common::req("POST", &format!("/emails/{id}/cancel"), Some(&t.key), None),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, b) = common::call(
        &t.router,
        common::req("POST", &format!("/emails/{id}/cancel"), Some(&t.key), None),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_eq!(b["error"]["code"], "not_cancelable");
}

#[sqlx::test]
async fn permissions_pause_and_isolation(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    common::verified_domain(&t, "shop.com").await;
    let sending = keys::create(&t.state.pool, t.account.id, "s", KeyAccess::Sending)
        .await
        .unwrap()
        .token;
    let (s, b) = common::call(
        &t.router,
        common::req("POST", "/emails", Some(&sending), Some(email("a@x.com"))),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "sending keys can send");
    let id = b["id"].as_str().unwrap().to_string();
    let (s, _) = common::call(
        &t.router,
        common::req("GET", &format!("/emails/{id}"), Some(&sending), None),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "sending keys can read one email");
    let (s, _) = common::call(
        &t.router,
        common::req("GET", "/emails", Some(&sending), None),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN, "sending keys cannot list");

    let other = accounts::create(&t.state.pool, "Other").await.unwrap();
    let ok = keys::create(&t.state.pool, other.id, "o", KeyAccess::Full)
        .await
        .unwrap()
        .token;
    let (s, _) = common::call(
        &t.router,
        common::req("GET", &format!("/emails/{id}"), Some(&ok), None),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    accounts::pause(&t.state.pool, t.account.id, "test")
        .await
        .unwrap();
    let (s, b) = common::call(
        &t.router,
        common::req("POST", "/emails", Some(&t.key), Some(email("a@x.com"))),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(b["error"]["code"], "account_paused");
}

#[sqlx::test]
async fn list_paginates_newest_first(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    common::verified_domain(&t, "shop.com").await;
    for i in 0..3 {
        common::call(
            &t.router,
            common::req(
                "POST",
                "/emails",
                Some(&t.key),
                Some(email(&format!("u{i}@x.com"))),
            ),
        )
        .await;
    }
    let (_, p1) = common::call(
        &t.router,
        common::req("GET", "/emails?limit=2", Some(&t.key), None),
    )
    .await;
    assert_eq!(p1["data"].as_array().unwrap().len(), 2);
    let cursor = p1["next_cursor"].as_str().unwrap();
    let (_, p2) = common::call(
        &t.router,
        common::req(
            "GET",
            &format!("/emails?limit=2&after={cursor}"),
            Some(&t.key),
            None,
        ),
    )
    .await;
    assert_eq!(p2["data"].as_array().unwrap().len(), 1);
    assert!(p2["next_cursor"].is_null());
}

#[sqlx::test]
async fn priority_defaults_to_normal_and_shows_everywhere(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    common::verified_domain(&t, "shop.com").await;
    let (s, b) = common::call(&t.router, common::req("POST", "/emails", Some(&t.key), Some(email("a@x.com")))).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["priority"], "normal");
    let id = b["id"].as_str().unwrap();
    let (_, v) = common::call(&t.router, common::req("GET", &format!("/emails/{id}"), Some(&t.key), None)).await;
    assert_eq!(v["priority"], "normal");
    let (_, l) = common::call(&t.router, common::req("GET", "/emails", Some(&t.key), None)).await;
    assert_eq!(l["data"][0]["priority"], "normal");
    let (_, c) = common::call(&t.router, common::req("POST", &format!("/emails/{id}/cancel"), Some(&t.key), None)).await;
    assert_eq!(c["priority"], "normal");
}

#[sqlx::test]
async fn priority_is_saved_per_batch_item(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    common::verified_domain(&t, "shop.com").await;
    let mut urgent = email("a@x.com");
    urgent["priority"] = json!("urgent");
    let mut bulk = email("b@x.com");
    bulk["priority"] = json!("bulk");
    let (s, b) = common::call(&t.router, common::req("POST", "/emails/batch", Some(&t.key), Some(json!([urgent, bulk, email("c@x.com")])))).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    let got: Vec<&str> = b["data"].as_array().unwrap().iter().map(|e| e["priority"].as_str().unwrap()).collect();
    assert_eq!(got, ["urgent", "bulk", "normal"]);
}

#[sqlx::test]
async fn unknown_priority_is_a_validation_error(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    common::verified_domain(&t, "shop.com").await;
    let mut e = email("a@x.com");
    e["priority"] = json!("asap");
    let (s, b) = common::call(&t.router, common::req("POST", "/emails", Some(&t.key), Some(e))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(b["error"]["code"], "validation_error");
}
