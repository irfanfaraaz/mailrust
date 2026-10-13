mod common;

use axum::http::StatusCode;
use mailrust::users::{self, MemberRole};
use serde_json::json;

async fn setup(pool: &sqlx::PgPool) -> common::TestApp {
    let mut t = common::app(pool.clone()).await;
    common::with_system_sender(&mut t, "Mailrust <login@mail.shop.com>").await;
    users::invite_direct(pool, "ana@shop.com", t.account.id, MemberRole::Owner).await.unwrap();
    t
}

async fn login_mails(pool: &sqlx::PgPool, to: &str) -> Vec<String> {
    sqlx::query_scalar("SELECT text_body FROM emails WHERE $1 = ANY(to_addresses) ORDER BY created_at")
        .bind(to)
        .fetch_all(pool)
        .await
        .unwrap()
}

fn token_in(text: &str) -> String {
    let start = text.find("token=").expect("link") + "token=".len();
    text[start..].split_whitespace().next().unwrap().to_string()
}

#[sqlx::test]
async fn a_known_email_gets_one_link_that_logs_in_once(pool: sqlx::PgPool) {
    let t = setup(&pool).await;
    let (s, _) = common::call(&t.router, common::req("POST", "/auth/login", None, Some(json!({"email": " Ana@Shop.com "})))).await;
    assert_eq!(s, StatusCode::ACCEPTED);
    let mails = login_mails(&pool, "ana@shop.com").await;
    assert_eq!(mails.len(), 1);
    assert!(mails[0].contains("http://app.test/login/confirm?token=mr_login_"), "{}", mails[0]);
    let token = token_in(&mails[0]);

    let (s, b) = common::call(&t.router, common::req("POST", "/auth/verify", None, Some(json!({"token": token})))).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert!(b["session_token"].as_str().unwrap().starts_with("mr_sess_"));

    let (s, b) = common::call(&t.router, common::req("POST", "/auth/verify", None, Some(json!({"token": token})))).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    assert_eq!(b["error"]["code"], "invalid_token");
}

#[sqlx::test]
async fn an_unknown_email_gets_the_same_answer_and_no_mail(pool: sqlx::PgPool) {
    let t = setup(&pool).await;
    let (s, b) = common::call(&t.router, common::req("POST", "/auth/login", None, Some(json!({"email": "who@x.com"})))).await;
    assert_eq!(s, StatusCode::ACCEPTED);
    assert_eq!(b, serde_json::Value::Null);
    assert!(login_mails(&pool, "who@x.com").await.is_empty());
}

#[sqlx::test]
async fn an_expired_or_wrong_token_is_refused(pool: sqlx::PgPool) {
    let t = setup(&pool).await;
    common::call(&t.router, common::req("POST", "/auth/login", None, Some(json!({"email": "ana@shop.com"})))).await;
    let token = token_in(&login_mails(&pool, "ana@shop.com").await[0]);
    sqlx::query("UPDATE login_tokens SET expires_at = now() - interval '1 second'").execute(&pool).await.unwrap();
    for tok in [token.as_str(), "mr_login_nope"] {
        let (s, _) = common::call(&t.router, common::req("POST", "/auth/verify", None, Some(json!({"token": tok})))).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
    }
}

#[sqlx::test]
async fn the_sixth_request_in_an_hour_sends_nothing(pool: sqlx::PgPool) {
    let t = setup(&pool).await;
    for _ in 0..6 {
        let (s, _) = common::call(&t.router, common::req("POST", "/auth/login", None, Some(json!({"email": "ana@shop.com"})))).await;
        assert_eq!(s, StatusCode::ACCEPTED);
    }
    assert_eq!(login_mails(&pool, "ana@shop.com").await.len(), 5);
}

fn from_ip(ip: &str, secret: Option<&str>, email: &str) -> axum::http::Request<axum::body::Body> {
    let mut r = common::req("POST", "/auth/login", None, Some(json!({ "email": email })));
    r.headers_mut().insert("x-mailrust-client-ip", ip.parse().unwrap());
    if let Some(s) = secret {
        r.headers_mut().insert("x-mailrust-web", s.parse().unwrap());
    }
    r
}

#[sqlx::test]
async fn the_ip_limit_uses_the_passed_ip_only_with_the_secret(pool: sqlx::PgPool) {
    let mut t = setup(&pool).await;
    let secret = "w".repeat(32);
    let mut config = (*t.state.config).clone();
    config.web_secret = Some(secret.clone());
    t.state.config = std::sync::Arc::new(config);
    t.router = mailrust::api::router(t.state.clone());

    // 20 requests from 1.2.3.4 for different (unknown) emails use up that IP's hour.
    for i in 0..20 {
        common::call(&t.router, from_ip("1.2.3.4", Some(&secret), &format!("u{i}@x.com"))).await;
    }
    common::call(&t.router, from_ip("1.2.3.4", Some(&secret), "ana@shop.com")).await;
    assert!(login_mails(&pool, "ana@shop.com").await.is_empty(), "IP over its limit");

    // Without the secret the header is ignored, so this request is not counted against 1.2.3.4.
    common::call(&t.router, from_ip("1.2.3.4", None, "ana@shop.com")).await;
    assert_eq!(login_mails(&pool, "ana@shop.com").await.len(), 1);
}

#[sqlx::test]
async fn sessions_slide_when_used(pool: sqlx::PgPool) {
    let t = setup(&pool).await;
    let user = users::find_by_email(&pool, "ana@shop.com").await.unwrap().unwrap();
    let mut conn = pool.acquire().await.unwrap();
    let s = mailrust::sessions::create(&mut conn, user.id).await.unwrap();
    sqlx::query("UPDATE sessions SET last_used_at = now() - interval '2 hours', expires_at = now() + interval '1 day'")
        .execute(&pool).await.unwrap();
    let found = mailrust::sessions::find(&pool, &s.token).await.unwrap().unwrap();
    assert_eq!(found.email, "ana@shop.com");
    let days: f64 = sqlx::query_scalar("SELECT (extract(epoch FROM expires_at - now()) / 86400)::float8 FROM sessions")
        .fetch_one(&pool).await.unwrap();
    assert!(days > 29.9, "{days}");
    sqlx::query("UPDATE sessions SET expires_at = now() - interval '1 second'").execute(&pool).await.unwrap();
    assert!(mailrust::sessions::find(&pool, &s.token).await.unwrap().is_none());
    let _ = t;
}
