mod common;

use std::net::SocketAddr;

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use mailrust::tokens::{self, TokenKind};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

async fn raw(
    router: &axum::Router,
    req: Request<Body>,
) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let res = router.clone().oneshot(req).await.unwrap();
    let (parts, body) = res.into_parts();
    (
        parts.status,
        parts.headers,
        body.collect().await.unwrap().to_bytes().to_vec(),
    )
}

fn get(uri: &str, peer: [u8; 4], forwarded: bool) -> Request<Body> {
    let mut b = Request::builder().uri(uri);
    if forwarded {
        b = b.header("x-forwarded-for", "8.8.8.8");
    }
    let mut r = b.body(Body::empty()).unwrap();
    r.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from((peer, 4000))));
    r
}

async fn sent_email(t: &common::TestApp, seconds_ago: i64) -> Uuid {
    common::verified_domain(t, "shop.com").await;
    let id = common::send_email(
        t,
        json!({ "from": "hi@shop.com", "to": "a@x.com", "subject": "Hi", "text": "x" }),
    )
    .await;
    sqlx::query("UPDATE emails SET status = 'sent', sent_at = now() - make_interval(secs => $2), links = $3 WHERE id = $1")
        .bind(id).bind(seconds_ago as f64).bind(json!(["https://shop.com/sale?a=1"]))
        .execute(&t.state.pool).await.unwrap();
    id
}

fn secret(t: &common::TestApp) -> Vec<u8> {
    t.state.config.tracking_secret.clone()
}

async fn events(t: &common::TestApp, id: Uuid) -> Vec<(String, bool)> {
    sqlx::query_as("SELECT type, likely_bot FROM email_events WHERE email_id = $1 ORDER BY id")
        .bind(id)
        .fetch_all(&t.state.pool)
        .await
        .unwrap()
}

#[sqlx::test]
async fn open_records_once_per_load_and_always_returns_a_gif(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let id = sent_email(&t, 60).await;
    let token = tokens::sign(&secret(&t), id, TokenKind::Open);
    let (s, h, body) = raw(
        &t.router,
        get(&format!("/o/{token}.gif"), [8, 8, 8, 8], true),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(h["content-type"], "image/gif");
    assert_eq!(body, mailrust::tracking::PIXEL_GIF);
    assert_eq!(events(&t, id).await, vec![("opened".to_string(), false)]);

    let (s, _, body) = raw(&t.router, get("/o/garbage.gif", [8, 8, 8, 8], true)).await;
    assert_eq!((s, body.len()), (StatusCode::OK, 43));
    assert_eq!(events(&t, id).await.len(), 1);
}

#[sqlx::test]
async fn click_redirects_only_to_the_saved_link(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let id = sent_email(&t, 60).await;
    let token = tokens::sign(&secret(&t), id, TokenKind::Click(0));
    let (s, h, _) = raw(&t.router, get(&format!("/c/{token}"), [8, 8, 8, 8], true)).await;
    assert_eq!(s, StatusCode::FOUND);
    assert_eq!(h["location"], "https://shop.com/sale?a=1");
    assert_eq!(events(&t, id).await, vec![("clicked".to_string(), false)]);

    let out_of_range = tokens::sign(&secret(&t), id, TokenKind::Click(5));
    assert_eq!(
        raw(
            &t.router,
            get(&format!("/c/{out_of_range}"), [8, 8, 8, 8], true)
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let open_token = tokens::sign(&secret(&t), id, TokenKind::Open);
    assert_eq!(
        raw(
            &t.router,
            get(&format!("/c/{open_token}"), [8, 8, 8, 8], true)
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        raw(&t.router, get("/c/nope", [8, 8, 8, 8], true)).await.0,
        StatusCode::NOT_FOUND
    );
}

#[sqlx::test]
async fn very_fast_clicks_are_flagged_as_bots(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let id = sent_email(&t, 2).await;
    let token = tokens::sign(&secret(&t), id, TokenKind::Click(0));
    raw(&t.router, get(&format!("/c/{token}"), [8, 8, 8, 8], true)).await;
    assert_eq!(events(&t, id).await, vec![("clicked".to_string(), true)]);
}

#[sqlx::test]
async fn tls_allowed_answers_only_internal_callers(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let d = common::verified_domain(&t, "shop.com").await;
    sqlx::query("UPDATE domains SET tracking_subdomain = 'track.shop.com', tracking_verified = true WHERE id = $1")
        .bind(d.id).execute(&t.state.pool).await.unwrap();
    let ask = |host: &str, peer: [u8; 4], fwd: bool| {
        get(&format!("/internal/tls-allowed?domain={host}"), peer, fwd)
    };
    assert_eq!(
        raw(&t.router, ask("track.shop.com", [172, 18, 0, 3], false))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        raw(&t.router, ask("mail.test", [127, 0, 0, 1], false))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        raw(&t.router, ask("evil.com", [127, 0, 0, 1], false))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        raw(&t.router, ask("track.shop.com", [8, 8, 8, 8], false))
            .await
            .0,
        StatusCode::NOT_FOUND,
        "public caller"
    );
    assert_eq!(
        raw(&t.router, ask("track.shop.com", [172, 18, 0, 3], true))
            .await
            .0,
        StatusCode::NOT_FOUND,
        "came through the proxy"
    );
}
