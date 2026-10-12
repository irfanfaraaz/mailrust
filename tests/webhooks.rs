mod common;

use axum::{
    Router,
    body::Bytes,
    http::{HeaderMap, StatusCode},
    routing::post,
};
use mailrust::{
    api::AppState,
    events, safety,
    webhooks::{self, sign},
};
use serde_json::json;
use tokio::sync::mpsc;
use uuid::Uuid;

async fn hook_server(answer: StatusCode) -> (String, mpsc::UnboundedReceiver<(HeaderMap, Bytes)>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let app = Router::new().route(
        "/hook",
        post(move |h: HeaderMap, b: Bytes| {
            let tx = tx.clone();
            async move {
                let _ = tx.send((h, b));
                answer
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}/hook"), rx)
}

async fn create_hook(t: &common::TestApp, url: &str, events: &[&str]) -> serde_json::Value {
    let (s, b) = common::call(
        &t.router,
        common::req(
            "POST",
            "/webhooks",
            Some(&t.key),
            Some(json!({ "url": url, "events": events })),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{b}");
    b
}

async fn deliver(t: &common::TestApp, id: Uuid) {
    let ev: events::SesEvent = serde_json::from_value(json!({
        "eventType": "Delivery",
        "mail": { "messageId": "ses-1", "tags": { "mailrust_email_id": [id.to_string()] } },
        "delivery": { "recipients": ["a@x.com"] }
    }))
    .unwrap();
    events::ingest(&t.state.pool, &ev, None).await.unwrap();
}

async fn queued(t: &common::TestApp) -> Uuid {
    common::verified_domain(t, "shop.com").await;
    common::send_email(
        t,
        json!({ "from": "hi@shop.com", "to": "a@x.com", "subject": "Hi", "text": "x" }),
    )
    .await
}

#[sqlx::test]
async fn crud_and_validation(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let (s, _) = common::call(
        &t.router,
        common::req(
            "POST",
            "/webhooks",
            Some(&t.key),
            Some(json!({ "url": "https://1.1.1.1/h", "events": ["email.nope"] })),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let created = create_hook(&t, "http://127.0.0.1:9/h", &["email.delivered"]).await;
    assert!(created["secret"].as_str().unwrap().starts_with("whsec_"));
    let (_, l) = common::call(
        &t.router,
        common::req("GET", "/webhooks", Some(&t.key), None),
    )
    .await;
    assert!(
        l["data"][0].get("secret").is_none(),
        "secret is shown only once"
    );
    let id = created["id"].as_str().unwrap();
    let (s, _) = common::call(
        &t.router,
        common::req("DELETE", &format!("/webhooks/{id}"), Some(&t.key), None),
    )
    .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
}

#[sqlx::test]
async fn private_targets_are_refused_when_not_allowed(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let mut cfg = (*t.state.config).clone();
    cfg.allow_private_webhooks = false;
    let state = AppState::new(t.state.pool.clone(), cfg, t.state.sender.clone());
    let router = mailrust::api::router(state);
    for url in [
        "http://1.1.1.1/h",
        "https://127.0.0.1/h",
        "https://169.254.169.254/latest",
    ] {
        let (s, b) = common::call(
            &router,
            common::req(
                "POST",
                "/webhooks",
                Some(&t.key),
                Some(json!({ "url": url, "events": ["email.sent"] })),
            ),
        )
        .await;
        assert_eq!(s, StatusCode::BAD_REQUEST, "{url}: {b}");
    }
}

#[sqlx::test]
async fn subscribed_events_are_delivered_and_signed(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let (url, mut rx) = hook_server(StatusCode::OK).await;
    let hook = create_hook(&t, &url, &["email.delivered"]).await;
    let id = queued(&t).await;
    deliver(&t, id).await;
    assert_eq!(webhooks::deliver_due(&t.state).await.unwrap(), 1);

    let (headers, body) = rx.recv().await.unwrap();
    let msg_id = headers["webhook-id"].to_str().unwrap();
    let ts: i64 = headers["webhook-timestamp"]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    let expected = sign::signature(hook["secret"].as_str().unwrap(), msg_id, ts, &body).unwrap();
    assert_eq!(headers["webhook-signature"].to_str().unwrap(), expected);
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["type"], "email.delivered");
    assert_eq!(payload["data"]["email_id"], id.to_string());

    let status: String = sqlx::query_scalar("SELECT status::text FROM webhook_deliveries")
        .fetch_one(&t.state.pool)
        .await
        .unwrap();
    assert_eq!(status, "delivered");
}

#[sqlx::test]
async fn unsubscribed_events_are_not_queued(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    create_hook(&t, "http://127.0.0.1:9/h", &["email.bounced"]).await;
    let id = queued(&t).await;
    deliver(&t, id).await;
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM webhook_deliveries")
        .fetch_one(&t.state.pool)
        .await
        .unwrap();
    assert_eq!(n, 0);
}

#[sqlx::test]
async fn failures_retry_then_give_up(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let (url, _rx) = hook_server(StatusCode::INTERNAL_SERVER_ERROR).await;
    create_hook(&t, &url, &["email.delivered"]).await;
    let id = queued(&t).await;
    deliver(&t, id).await;
    webhooks::deliver_due(&t.state).await.unwrap();
    let (status, attempts, later, code): (String, i32, bool, Option<i32>) = sqlx::query_as(
        "SELECT status::text, attempts, next_attempt_at > now(), last_response_code FROM webhook_deliveries",
    ).fetch_one(&t.state.pool).await.unwrap();
    assert_eq!(
        (status.as_str(), attempts, later, code),
        ("pending", 1, true, Some(500))
    );

    sqlx::query("UPDATE webhook_deliveries SET attempts = 7, next_attempt_at = now()")
        .execute(&t.state.pool)
        .await
        .unwrap();
    webhooks::deliver_due(&t.state).await.unwrap();
    let status: String = sqlx::query_scalar("SELECT status::text FROM webhook_deliveries")
        .fetch_one(&t.state.pool)
        .await
        .unwrap();
    assert_eq!(status, "failed");
}

#[sqlx::test]
async fn private_address_is_blocked_at_delivery_time(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    // Saved earlier while it pointed somewhere public; now it points inside.
    sqlx::query("INSERT INTO webhooks (id, account_id, url, secret, event_types) VALUES ($1, $2, 'https://127.0.0.1:9/h', $3, '{email.delivered}')")
        .bind(Uuid::now_v7()).bind(t.account.id).bind(sign::new_secret())
        .execute(&t.state.pool).await.unwrap();
    let mut cfg = (*t.state.config).clone();
    cfg.allow_private_webhooks = false;
    let state = AppState::new(t.state.pool.clone(), cfg, t.state.sender.clone());
    let id = queued(&t).await;
    deliver(&t, id).await;
    webhooks::deliver_due(&state).await.unwrap();
    let err: Option<String> = sqlx::query_scalar("SELECT last_error FROM webhook_deliveries")
        .fetch_one(&t.state.pool)
        .await
        .unwrap();
    assert!(err.unwrap().contains("private network"));
}

#[sqlx::test]
async fn auto_pause_sends_account_paused(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    create_hook(&t, "http://127.0.0.1:9/h", &["account.paused"]).await;
    let id = queued(&t).await;
    let recipients: Vec<String> = (0..100).map(|i| format!("u{i}@x.com")).collect();
    sqlx::query("INSERT INTO email_events (id, account_id, email_id, type, data) VALUES ($1, $2, $3, 'sent', $4)")
        .bind(Uuid::now_v7()).bind(t.account.id).bind(id).bind(json!({ "recipients": recipients }))
        .execute(&t.state.pool).await.unwrap();
    for i in 0..3 {
        sqlx::query("INSERT INTO email_events (id, account_id, email_id, type, recipient) VALUES ($1, $2, $3, 'bounced', $4)")
            .bind(Uuid::now_v7()).bind(t.account.id).bind(id).bind(format!("u{i}@x.com"))
            .execute(&t.state.pool).await.unwrap();
    }
    assert!(
        safety::check_account(&t.state.pool, t.account.id)
            .await
            .unwrap()
            .is_some()
    );
    let kind: String = sqlx::query_scalar("SELECT event_type FROM webhook_deliveries")
        .fetch_one(&t.state.pool)
        .await
        .unwrap();
    assert_eq!(kind, "account.paused");
}
