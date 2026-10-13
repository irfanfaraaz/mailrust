mod common;

use axum::http::StatusCode;
use serde_json::{Value, json};

async fn settings(t: &common::TestApp, key: &str, body: Option<Value>) -> (StatusCode, Value) {
    let m = if body.is_some() { "PATCH" } else { "GET" };
    common::call(&t.router, common::req(m, "/newsletter", Some(key), body)).await
}

#[sqlx::test]
async fn the_form_opens_only_when_everything_is_set(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let (s, b) = settings(&t, &t.key, None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(b["signup_open"], false);
    assert!(b["closed_reason"].is_string());

    common::verified_domain(&t, "news.shop.com").await;
    let (s, b) = settings(&t, &t.key, Some(json!({
        "public_slug": "  Weekly ", "newsletter_name": "Shop Weekly",
        "newsletter_from": "Hello@News.Shop.com", "signup_enabled": true
    }))).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["public_slug"], "weekly");
    assert_eq!(b["newsletter_from"], "hello@news.shop.com");
    assert_eq!(b["signup_open"], true);
    assert!(b["closed_reason"].is_null());

    mailrust::accounts::pause(&pool, t.account.id, "test").await.unwrap();
    let (_, b) = settings(&t, &t.key, None).await;
    assert_eq!(b["signup_open"], false);
    assert!(b["closed_reason"].as_str().unwrap().contains("paused"));
}

#[sqlx::test]
async fn the_from_address_must_be_on_a_verified_domain(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let (s, b) = settings(&t, &t.key, Some(json!({"newsletter_from": "hi@elsewhere.com"}))).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{b}");
    assert_eq!(b["error"]["code"], "from_not_verified");
    let (s, _) = settings(&t, &t.key, Some(json!({"newsletter_from": "nope"}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn slugs_are_checked_and_unique(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    for bad in ["ab", "a_b", &"x".repeat(41)] {
        let (s, _) = settings(&t, &t.key, Some(json!({"public_slug": bad}))).await;
        assert_eq!(s, StatusCode::BAD_REQUEST, "{bad}");
    }
    let (s, _) = settings(&t, &t.key, Some(json!({"signup_enabled": true}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "no slug yet");
    let (s, _) = settings(&t, &t.key, Some(json!({"public_slug": "weekly"}))).await;
    assert_eq!(s, StatusCode::OK);

    let other = mailrust::accounts::create(&pool, "Other").await.unwrap();
    let key = mailrust::keys::create(&pool, other.id, "k", mailrust::keys::KeyAccess::Full).await.unwrap().token;
    for taken in ["weekly", " WEEKLY "] {
        let (s, b) = settings(&t, &key, Some(json!({"public_slug": taken}))).await;
        assert_eq!(s, StatusCode::CONFLICT, "{b}");
        assert_eq!(b["error"]["code"], "slug_taken");
    }
    let (s, b) = settings(&t, &t.key, Some(json!({"public_slug": ""}))).await;
    assert_eq!(s, StatusCode::OK);
    assert!(b["public_slug"].is_null());
    let (s, _) = settings(&t, &key, Some(json!({"public_slug": "weekly"}))).await;
    assert_eq!(s, StatusCode::OK, "a freed slug can be taken");
}

use mailrust::tokens;

async fn contact(t: &common::TestApp, email: &str) -> uuid::Uuid {
    let (s, b) = common::call(&t.router, common::req("POST", "/contacts", Some(&t.key), Some(json!({"email": email})))).await;
    assert_eq!(s, StatusCode::CREATED, "{b}");
    b["id"].as_str().unwrap().parse().unwrap()
}

async fn status_of(pool: &sqlx::PgPool, id: uuid::Uuid) -> String {
    sqlx::query_scalar("SELECT status::text FROM contacts WHERE id = $1").bind(id).fetch_one(pool).await.unwrap()
}

#[sqlx::test]
async fn public_routes_answer_only_the_web_app(pool: sqlx::PgPool) {
    let mut t = common::app(pool).await;
    // WEB_SECRET unset: nobody gets in.
    let (s, _) = common::call(&t.router, common::web_req("GET", "/public/newsletter/weekly", "1.2.3.4", None)).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    common::with_web_secret(&mut t);
    settings(&t, &t.key, Some(json!({"public_slug": "weekly"}))).await;
    let (s, _) = common::call(&t.router, common::req("GET", "/public/newsletter/weekly", None, None)).await;
    assert_eq!(s, StatusCode::NOT_FOUND, "no header");
    let mut wrong = common::web_req("GET", "/public/newsletter/weekly", "1.2.3.4", None);
    wrong.headers_mut().insert("x-mailrust-web", "x".repeat(37).parse().unwrap());
    let (s, _) = common::call(&t.router, wrong).await;
    assert_eq!(s, StatusCode::NOT_FOUND, "wrong secret");
    let (s, b) = common::call(&t.router, common::web_req("GET", "/public/newsletter/Weekly", "1.2.3.4", None)).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b, json!({"name": "Test", "open": false}));
    let (s, _) = common::call(&t.router, common::web_req("GET", "/public/newsletter/nobody", "1.2.3.4", None)).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn public_routes_are_rate_limited_per_ip(pool: sqlx::PgPool) {
    let mut t = common::app(pool).await;
    common::with_web_secret(&mut t);
    let mut limited = false;
    for _ in 0..40 {
        let (s, _) = common::call(&t.router, common::web_req("GET", "/public/newsletter/x", "9.9.9.9", None)).await;
        limited |= s == StatusCode::TOO_MANY_REQUESTS;
    }
    assert!(limited);
    let (s, _) = common::call(&t.router, common::web_req("GET", "/public/newsletter/x", "8.8.8.8", None)).await;
    assert_eq!(s, StatusCode::NOT_FOUND, "another IP is not limited");
}

#[sqlx::test]
async fn the_link_unsubscribes_and_can_resubscribe(pool: sqlx::PgPool) {
    let mut t = common::app(pool.clone()).await;
    common::with_web_secret(&mut t);
    settings(&t, &t.key, Some(json!({"newsletter_name": "Shop Weekly"}))).await;
    common::call(&t.router, common::req("POST", "/suppressions", Some(&t.key), Some(json!({"address": "other@x.com"})))).await;
    let id = contact(&t, "ana@x.com").await;
    let token = tokens::sign_unsubscribe(&t.state.config.tracking_secret, id);
    let web = |m: &str, uri: &str, body: Option<Value>| common::web_req(m, uri, "1.2.3.4", body);

    let (s, b) = common::call(&t.router, web("GET", &format!("/public/unsubscribe/{token}"), None)).await;
    assert_eq!((s, b), (StatusCode::OK, json!({"newsletter_name": "Shop Weekly"})));
    assert_eq!(status_of(&pool, id).await, "subscribed", "looking does not unsubscribe");

    for _ in 0..2 {
        let (s, b) = common::call(&t.router, web("POST", "/public/unsubscribe", Some(json!({"token": token})))).await;
        assert_eq!((s, b), (StatusCode::OK, json!({"newsletter_name": "Shop Weekly"})));
    }
    assert_eq!(status_of(&pool, id).await, "unsubscribed");
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM suppressions").fetch_one(&pool).await.unwrap();
    assert_eq!(n, 1, "the suppression list is unchanged");

    let (s, _) = common::call(&t.router, web("POST", "/public/resubscribe", Some(json!({"token": token})))).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert_eq!(status_of(&pool, id).await, "subscribed");
    let (s, b) = common::call(&t.router, web("POST", "/public/resubscribe", Some(json!({"token": token})))).await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_eq!(b["error"]["code"], "resubscribe_not_allowed");
}

#[sqlx::test]
async fn the_link_cannot_undo_an_unsubscribe_made_elsewhere(pool: sqlx::PgPool) {
    let mut t = common::app(pool.clone()).await;
    common::with_web_secret(&mut t);
    let id = contact(&t, "ana@x.com").await;
    common::call(&t.router, common::req("PATCH", &format!("/contacts/{id}"), Some(&t.key), Some(json!({"status": "unsubscribed"})))).await;
    let token = tokens::sign_unsubscribe(&t.state.config.tracking_secret, id);
    let (s, _) = common::call(&t.router, common::web_req("POST", "/public/resubscribe", "1.2.3.4", Some(json!({"token": token})))).await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_eq!(status_of(&pool, id).await, "unsubscribed");
}

#[sqlx::test]
async fn bad_or_stale_tokens_change_nothing(pool: sqlx::PgPool) {
    let mut t = common::app(pool.clone()).await;
    common::with_web_secret(&mut t);
    let id = contact(&t, "ana@x.com").await;
    let secret = t.state.config.tracking_secret.clone();
    let good = tokens::sign_unsubscribe(&secret, id);
    let mut flipped = good.clone().into_bytes();
    flipped[30] = if flipped[30] == b'A' { b'B' } else { b'A' };
    let bad = [
        String::from_utf8(flipped).unwrap(),
        good[..40].to_string(),
        tokens::sign(&secret, id, tokens::TokenKind::Open),
        tokens::sign_unsubscribe(b"another-secret-another-secret-xx", id),
        "garbage".into(),
    ];
    for (i, token) in bad.iter().enumerate() {
        let ip = format!("10.0.0.{i}"); // one IP per token keeps the per-IP rate limit out of this test
        let (s, b) = common::call(&t.router, common::web_req("POST", "/public/unsubscribe", &ip, Some(json!({"token": token})))).await;
        assert_eq!((s, b), (StatusCode::OK, json!({"newsletter_name": null})), "{token}");
        let (s, _) = common::call(&t.router, common::web_req("POST", "/public/resubscribe", &ip, Some(json!({"token": token})))).await;
        assert_eq!(s, StatusCode::CONFLICT);
    }
    assert_eq!(status_of(&pool, id).await, "subscribed");

    common::call(&t.router, common::req("DELETE", &format!("/contacts/{id}"), Some(&t.key), None)).await;
    let (s, b) = common::call(&t.router, common::web_req("POST", "/public/unsubscribe", "1.2.3.4", Some(json!({"token": good})))).await;
    assert_eq!((s, b), (StatusCode::OK, json!({"newsletter_name": null})), "a deleted contact still gets the plain answer");
}
