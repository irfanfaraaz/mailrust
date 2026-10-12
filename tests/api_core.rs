mod common;

use axum::{Json, Router, http::StatusCode, routing::get};
use mailrust::{api::{AppState, auth::Auth}, accounts, keys::{self, KeyAccess}};
use serde_json::json;

fn whoami_router(state: AppState) -> Router {
    Router::new()
        .route("/whoami", get(|auth: Auth| async move {
            auth.require_active()?;
            Ok::<_, mailrust::error::ApiError>(Json(json!({ "account": auth.account_id() })))
        }))
        .with_state(state)
}

#[sqlx::test]
async fn health_is_ok(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let (status, body) = common::call(&t.router, common::req("GET", "/health", None, None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[sqlx::test]
async fn missing_and_bad_keys_get_401_in_the_error_shape(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let r = whoami_router(t.state.clone());
    let (s, b) = common::call(&r, common::req("GET", "/whoami", None, None)).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    assert_eq!(b["error"]["code"], "unauthorized");
    let (s, _) = common::call(&r, common::req("GET", "/whoami", Some("mr_live_nope"), None)).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, b) = common::call(&r, common::req("GET", "/whoami", Some(&t.key), None)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(b["account"], t.account.id.to_string());
}

#[sqlx::test]
async fn paused_account_gets_account_paused(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    accounts::pause(&t.state.pool, t.account.id, "too many bounces").await.unwrap();
    let (s, b) = common::call(&whoami_router(t.state.clone()), common::req("GET", "/whoami", Some(&t.key), None)).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(b["error"]["code"], "account_paused");
}

#[sqlx::test]
async fn more_than_ten_requests_a_second_get_429(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let key = keys::create(&t.state.pool, t.account.id, "burst", KeyAccess::Full).await.unwrap().token;
    let r = whoami_router(t.state.clone());
    let mut limited = 0;
    for _ in 0..15 {
        let (s, _) = common::call(&r, common::req("GET", "/whoami", Some(&key), None)).await;
        if s == StatusCode::TOO_MANY_REQUESTS {
            limited += 1;
        }
    }
    assert!(limited >= 1, "expected at least one 429");
}
