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
