mod common;

use axum::{Router, body::Bytes, http::StatusCode, routing::post};
use mailrust::{events, limiter::SendLimiter, webhooks, worker};
use serde_json::{Value, json};
use tokio::sync::mpsc;

#[sqlx::test]
async fn the_whole_loop(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let k = Some(t.key.as_str());
    let limiter = SendLimiter::new(10_000, 1);

    let (tx, mut rx) = mpsc::unbounded_channel::<Value>();
    let hook = Router::new().route(
        "/hook",
        post(move |b: Bytes| {
            let tx = tx.clone();
            async move {
                let _ = tx.send(serde_json::from_slice(&b).unwrap());
                StatusCode::OK
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, hook).await.unwrap() });

    let (s, _) = common::call(
        &t.router,
        common::req("POST", "/domains", k, Some(json!({ "name": "shop.com" }))),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);
    let (s, _) = common::call(
        &t.router,
        common::req(
            "POST",
            "/webhooks",
            k,
            Some(json!({ "url": format!("http://{addr}/hook"), "events": ["email.bounced"] })),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);

    let mail = json!({ "from": "Shop <hi@shop.com>", "to": events::SIM_BOUNCE, "subject": "Order", "text": "Thanks" });
    let (_, first) = common::call(
        &t.router,
        common::req("POST", "/emails", k, Some(mail.clone())),
    )
    .await;
    worker::tick(&t.state, &limiter).await.unwrap();
    webhooks::deliver_due(&t.state).await.unwrap();

    let id = first["id"].as_str().unwrap();
    let (_, view) = common::call(
        &t.router,
        common::req("GET", &format!("/emails/{id}"), k, None),
    )
    .await;
    assert_eq!(view["status"], "bounced");
    let got = rx.recv().await.unwrap();
    assert_eq!(got["type"], "email.bounced");
    assert_eq!(got["data"]["email_id"], id);

    let (_, list) = common::call(&t.router, common::req("GET", "/suppressions", k, None)).await;
    assert_eq!(list["data"][0]["address"], events::SIM_BOUNCE);

    let (_, second) = common::call(&t.router, common::req("POST", "/emails", k, Some(mail))).await;
    worker::tick(&t.state, &limiter).await.unwrap();
    let id2 = second["id"].as_str().unwrap();
    let (_, view2) = common::call(
        &t.router,
        common::req("GET", &format!("/emails/{id2}"), k, None),
    )
    .await;
    assert_eq!(view2["status"], "suppressed");
}
