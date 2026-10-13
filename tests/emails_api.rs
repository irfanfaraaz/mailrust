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
    let (s, b) = common::call(
        &t.router,
        common::req("POST", "/emails", Some(&t.key), Some(email("a@x.com"))),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["priority"], "normal");
    let id = b["id"].as_str().unwrap();
    let (_, v) = common::call(
        &t.router,
        common::req("GET", &format!("/emails/{id}"), Some(&t.key), None),
    )
    .await;
    assert_eq!(v["priority"], "normal");
    let (_, l) = common::call(&t.router, common::req("GET", "/emails", Some(&t.key), None)).await;
    assert_eq!(l["data"][0]["priority"], "normal");
    let (_, c) = common::call(
        &t.router,
        common::req("POST", &format!("/emails/{id}/cancel"), Some(&t.key), None),
    )
    .await;
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
    let (s, b) = common::call(
        &t.router,
        common::req(
            "POST",
            "/emails/batch",
            Some(&t.key),
            Some(json!([urgent, bulk, email("c@x.com")])),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    let got: Vec<&str> = b["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["priority"].as_str().unwrap())
        .collect();
    assert_eq!(got, ["urgent", "bulk", "normal"]);
}

#[sqlx::test]
async fn unknown_priority_is_a_validation_error(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    common::verified_domain(&t, "shop.com").await;
    let mut e = email("a@x.com");
    e["priority"] = json!("asap");
    let (s, b) = common::call(
        &t.router,
        common::req("POST", "/emails", Some(&t.key), Some(e)),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(b["error"]["code"], "validation_error");
}

fn urgent(to: &str) -> Value {
    let mut e = email(to);
    e["priority"] = json!("urgent");
    e
}

#[sqlx::test]
async fn urgent_over_the_cap_is_stored_as_normal(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    common::verified_domain(&t, "shop.com").await;
    common::seed_emails(&pool, t.account.id, "urgent", "sent", 100, 1.0).await;
    common::seed_emails(&pool, t.account.id, "normal", "sent", 899, 1.0).await; // 999 + this one = 1,000 → cap 100
    let (s, b) = common::call(
        &t.router,
        common::req("POST", "/emails", Some(&t.key), Some(urgent("a@x.com"))),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["priority"], "normal");
}

#[sqlx::test]
async fn the_cap_grows_with_volume_and_old_mail_does_not_count(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    common::verified_domain(&t, "shop.com").await;
    common::seed_emails(&pool, t.account.id, "urgent", "sent", 100, 25.0).await; // older than 24 h
    let (_, b) = common::call(
        &t.router,
        common::req("POST", "/emails", Some(&t.key), Some(urgent("a@x.com"))),
    )
    .await;
    assert_eq!(b["priority"], "urgent");
    common::seed_emails(&pool, t.account.id, "urgent", "sent", 100, 1.0).await;
    common::seed_emails(&pool, t.account.id, "normal", "sent", 1100, 1.0).await; // ~1,200 → cap 120
    let (_, b) = common::call(
        &t.router,
        common::req("POST", "/emails", Some(&t.key), Some(urgent("b@x.com"))),
    )
    .await;
    assert_eq!(b["priority"], "urgent");
}

#[sqlx::test]
async fn a_batch_crossing_the_cap_downgrades_only_the_extra(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    common::verified_domain(&t, "shop.com").await;
    common::seed_emails(&pool, t.account.id, "urgent", "sent", 99, 1.0).await;
    let (s, b) = common::call(
        &t.router,
        common::req(
            "POST",
            "/emails/batch",
            Some(&t.key),
            Some(json!([urgent("a@x.com"), urgent("b@x.com")])),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["data"][0]["priority"], "urgent");
    assert_eq!(b["data"][1]["priority"], "normal");
}

#[sqlx::test]
async fn an_idempotent_replay_returns_the_stored_priority(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    common::verified_domain(&t, "shop.com").await;
    common::seed_emails(&pool, t.account.id, "urgent", "sent", 100, 1.0).await;
    let body = urgent("a@x.com");
    let (_, first) = common::call(&t.router, with_key_header(&t.key, "k1", &body)).await;
    let (_, again) = common::call(&t.router, with_key_header(&t.key, "k1", &body)).await;
    assert_eq!(first["priority"], "normal");
    assert_eq!(again["priority"], "normal");
    assert_eq!(first["id"], again["id"]);
}

#[sqlx::test]
async fn a_batch_counts_its_own_emails_toward_the_cap(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    common::verified_domain(&t, "shop.com").await;
    common::seed_emails(&pool, t.account.id, "urgent", "sent", 100, 1.0).await;
    common::seed_emails(&pool, t.account.id, "normal", "sent", 1000, 1.0).await;
    let batch: Vec<Value> = (0..30).map(|i| urgent(&format!("u{i}@x.com"))).collect();
    let (s, b) = common::call(
        &t.router,
        common::req("POST", "/emails/batch", Some(&t.key), Some(json!(batch))),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    // Each email (urgent or downgraded) raises the total, so the cap opens again later in the batch.
    let got: String = b["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| if e["priority"] == "urgent" { 'U' } else { 'N' })
        .collect();
    assert_eq!(got, "UUUUUUUUUUUNNNNNNNNNUNNNNNNNNN");
}

#[sqlx::test]
async fn emails_can_be_filtered_by_priority_time_and_text(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    common::seed_emails(&pool, t.account.id, "bulk", "sent", 2, 48.0).await;
    let urgent = common::seed_emails(&pool, t.account.id, "urgent", "sent", 1, 1.0).await;
    sqlx::query("UPDATE emails SET subject = 'Reset 100%_off', to_addresses = ARRAY['Ana@Shop.com'] WHERE id = $1")
        .bind(urgent[0]).execute(&pool).await.unwrap();
    let get = |uri: String| {
        let t = &t;
        async move {
            common::call(&t.router, common::req("GET", &uri, Some(&t.key), None))
                .await
                .1["data"]
                .as_array()
                .unwrap()
                .len()
        }
    };

    assert_eq!(get("/emails?priority=bulk".into()).await, 2);
    let since = (chrono::Utc::now() - chrono::Duration::hours(2)).to_rfc3339();
    assert_eq!(
        get(format!("/emails?since={}", urlencoding(&since))).await,
        1
    );
    assert_eq!(get("/emails?q=ana%40shop".into()).await, 1);
    assert_eq!(
        get("/emails?q=100%25_off".into()).await,
        1,
        "% and _ are literal"
    );
    assert_eq!(get("/emails?q=1000".into()).await, 0);
    assert_eq!(
        get("/emails?q=_".into()).await,
        1,
        "_ alone matches only a real underscore"
    );
}

fn urlencoding(s: &str) -> String {
    s.replace('+', "%2B").replace(':', "%3A")
}
