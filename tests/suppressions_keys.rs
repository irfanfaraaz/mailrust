mod common;

use axum::http::StatusCode;
use serde_json::json;

#[sqlx::test]
async fn suppressions_add_list_remove(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let k = Some(t.key.as_str());
    let (s, b) = common::call(
        &t.router,
        common::req(
            "POST",
            "/suppressions",
            k,
            Some(json!({ "address": "Ann <ANN@x.com>" })),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);
    assert_eq!(b["address"], "ann@x.com");
    assert_eq!(b["reason"], "manual");
    let (s, _) = common::call(
        &t.router,
        common::req(
            "POST",
            "/suppressions",
            k,
            Some(json!({ "address": "nope" })),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (_, l) = common::call(&t.router, common::req("GET", "/suppressions", k, None)).await;
    assert_eq!(l["data"].as_array().unwrap().len(), 1);
    let (s, _) = common::call(
        &t.router,
        common::req("DELETE", "/suppressions/ANN@x.com", k, None),
    )
    .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let (s, _) = common::call(
        &t.router,
        common::req("DELETE", "/suppressions/ann@x.com", k, None),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn api_keys_create_list_revoke(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let k = Some(t.key.as_str());
    let (s, b) = common::call(
        &t.router,
        common::req(
            "POST",
            "/api-keys",
            k,
            Some(json!({ "name": "ci", "access": "sending" })),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);
    let token = b["token"].as_str().unwrap().to_string();
    assert!(token.starts_with("mr_live_"));
    let (_, l) = common::call(&t.router, common::req("GET", "/api-keys", k, None)).await;
    assert_eq!(l["data"].as_array().unwrap().len(), 2);
    assert!(l["data"][1].get("token").is_none());

    let (s, _) = common::call(
        &t.router,
        common::req("GET", "/api-keys", Some(&token), None),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN, "sending keys cannot manage keys");

    let id = b["id"].as_str().unwrap();
    let (s, _) = common::call(
        &t.router,
        common::req("DELETE", &format!("/api-keys/{id}"), k, None),
    )
    .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let (s, _) = common::call(
        &t.router,
        common::req("GET", "/suppressions", Some(&token), None),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED, "revoked keys stop working");
}
