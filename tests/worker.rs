mod common;

use mailrust::{
    accounts::{self, AccountStatus},
    limiter::SendLimiter,
    queue,
    safety::{self, SuppressionReason},
    sender::SenderError,
    worker,
};
use serde_json::json;

fn limiter() -> SendLimiter {
    SendLimiter::new(10_000, 1)
}

fn email(to: serde_json::Value) -> serde_json::Value {
    json!({ "from": "Shop <hi@shop.com>", "to": to, "subject": "Hi", "html": "<p><a href=\"https://shop.com/x\">x</a></p>" })
}

#[sqlx::test]
async fn sends_and_marks_sent_with_the_fake_sender(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    common::verified_domain(&t, "shop.com").await;
    let id = common::send_email(&t, email(json!("a@x.com"))).await;
    assert_eq!(worker::tick(&t.state, &limiter()).await.unwrap(), 1);
    assert_eq!(common::status_of(&t.state.pool, id).await, "delivered"); // the fake sender also simulates SES events, so a good send ends as delivered
    assert!(t.outbox.join(format!("{id}.eml")).exists());
    let sent: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM email_events WHERE email_id = $1 AND type = 'sent'",
    )
    .bind(id)
    .fetch_one(&t.state.pool)
    .await
    .unwrap();
    assert_eq!(sent, 1);
}

#[sqlx::test]
async fn future_emails_and_paused_accounts_are_not_claimed(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    common::verified_domain(&t, "shop.com").await;
    let mut later = email(json!("a@x.com"));
    later["scheduled_at"] = json!((chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339());
    common::send_email(&t, later).await;
    assert_eq!(worker::tick(&t.state, &limiter()).await.unwrap(), 0);

    common::send_email(&t, email(json!("b@x.com"))).await;
    accounts::pause(&t.state.pool, t.account.id, "test")
        .await
        .unwrap();
    assert_eq!(worker::tick(&t.state, &limiter()).await.unwrap(), 0);
}

#[sqlx::test]
async fn two_claims_never_overlap(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    common::verified_domain(&t, "shop.com").await;
    let batch: Vec<_> = (0..60)
        .map(|i| email(json!(format!("u{i}@x.com"))))
        .collect();
    let (s, _) = common::call(
        &t.router,
        common::req("POST", "/emails/batch", Some(&t.key), Some(json!(batch))),
    )
    .await;
    assert_eq!(s, axum::http::StatusCode::OK);
    let (a, b) = tokio::join!(
        queue::claim(&t.state.pool, 50),
        queue::claim(&t.state.pool, 50)
    );
    let (a, b) = (a.unwrap(), b.unwrap());
    assert_eq!(a.len() + b.len(), 60);
    let ids: std::collections::HashSet<_> = a.iter().chain(&b).map(|e| e.id).collect();
    assert_eq!(ids.len(), 60, "no email claimed twice");
}

#[sqlx::test]
async fn block_list_matches_display_names_and_case(pool: sqlx::PgPool) {
    let rec = common::RecordingSender::ok();
    let t = common::app_with_sender(pool, rec.clone()).await;
    common::verified_domain(&t, "shop.com").await;
    let mut conn = t.state.pool.acquire().await.unwrap();
    safety::add_suppression(
        &mut conn,
        t.account.id,
        "ann@shop.com",
        SuppressionReason::Bounce,
        None,
    )
    .await
    .unwrap();
    drop(conn);

    let only_ann = common::send_email(&t, email(json!("Ann <ANN@Shop.com>"))).await;
    let mixed = common::send_email(&t, email(json!(["Ann <ANN@Shop.com>", "bob@x.com"]))).await;
    worker::tick(&t.state, &limiter()).await.unwrap();

    assert_eq!(
        common::status_of(&t.state.pool, only_ann).await,
        "suppressed"
    );
    assert_eq!(common::status_of(&t.state.pool, mixed).await, "delivered");
    let sent = rec.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].recipients, vec!["bob@x.com"]);
}

#[sqlx::test]
async fn temporary_errors_retry_then_fail(pool: sqlx::PgPool) {
    let t = common::app_with_sender(
        pool,
        common::RecordingSender::failing(|| SenderError::Retryable("busy".into())),
    )
    .await;
    common::verified_domain(&t, "shop.com").await;
    let id = common::send_email(&t, email(json!("a@x.com"))).await;
    worker::tick(&t.state, &limiter()).await.unwrap();
    let (status, attempts, later): (String, i32, bool) =
        sqlx::query_as("SELECT status::text, attempts, send_at > now() FROM emails WHERE id = $1")
            .bind(id)
            .fetch_one(&t.state.pool)
            .await
            .unwrap();
    assert_eq!((status.as_str(), attempts, later), ("queued", 1, true));

    sqlx::query("UPDATE emails SET attempts = 5, send_at = now() WHERE id = $1")
        .bind(id)
        .execute(&t.state.pool)
        .await
        .unwrap();
    worker::tick(&t.state, &limiter()).await.unwrap();
    assert_eq!(common::status_of(&t.state.pool, id).await, "failed");
}

#[sqlx::test]
async fn tenant_pause_pauses_the_account_and_keeps_the_email(pool: sqlx::PgPool) {
    let t = common::app_with_sender(
        pool,
        common::RecordingSender::failing(|| SenderError::TenantPaused),
    )
    .await;
    common::verified_domain(&t, "shop.com").await;
    let id = common::send_email(&t, email(json!("a@x.com"))).await;
    worker::tick(&t.state, &limiter()).await.unwrap();
    assert_eq!(common::status_of(&t.state.pool, id).await, "queued");
    let acc = accounts::get(&t.state.pool, t.account.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(acc.status, AccountStatus::Paused);
    let attempts: i32 = sqlx::query_scalar("SELECT attempts FROM emails WHERE id = $1")
        .bind(id)
        .fetch_one(&t.state.pool)
        .await
        .unwrap();
    assert_eq!(attempts, 0);
}

#[sqlx::test]
async fn expired_leases_go_unknown_then_back_in_line(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    common::verified_domain(&t, "shop.com").await;
    let id = common::send_email(&t, email(json!("a@x.com"))).await;
    sqlx::query("UPDATE emails SET status = 'sending', lease_until = now() - interval '1 second' WHERE id = $1")
        .bind(id).execute(&t.state.pool).await.unwrap();
    assert_eq!(queue::expire_leases(&t.state.pool).await.unwrap(), 1);
    assert_eq!(common::status_of(&t.state.pool, id).await, "unknown");
    assert_eq!(
        queue::requeue_unknown(&t.state.pool).await.unwrap(),
        0,
        "still inside the grace period"
    );
    sqlx::query("UPDATE emails SET unknown_since = now() - interval '3 minutes' WHERE id = $1")
        .bind(id)
        .execute(&t.state.pool)
        .await
        .unwrap();
    assert_eq!(queue::requeue_unknown(&t.state.pool).await.unwrap(), 1);
    assert_eq!(common::status_of(&t.state.pool, id).await, "queued");
}

#[sqlx::test]
async fn click_tracking_rewrites_and_saves_links(pool: sqlx::PgPool) {
    let rec = common::RecordingSender::ok();
    let t = common::app_with_sender(pool, rec.clone()).await;
    let d = common::verified_domain(&t, "shop.com").await;
    sqlx::query("UPDATE domains SET click_tracking = true, open_tracking = true WHERE id = $1")
        .bind(d.id)
        .execute(&t.state.pool)
        .await
        .unwrap();
    let id = common::send_email(&t, email(json!("a@x.com"))).await;
    worker::tick(&t.state, &limiter()).await.unwrap();
    let raw = String::from_utf8_lossy(&rec.sent()[0].raw).to_string();
    assert!(
        raw.contains("http://mail.test/c/"),
        "falls back to PUBLIC_URL when no tracking domain"
    );
    assert!(raw.contains("http://mail.test/o/"));
    let links: serde_json::Value = sqlx::query_scalar("SELECT links FROM emails WHERE id = $1")
        .bind(id)
        .fetch_one(&t.state.pool)
        .await
        .unwrap();
    assert_eq!(links, json!(["https://shop.com/x"]));
}

#[sqlx::test]
async fn removed_domain_fails_the_email(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let d = common::verified_domain(&t, "shop.com").await;
    let id = common::send_email(&t, email(json!("a@x.com"))).await;
    sqlx::query("DELETE FROM domains WHERE id = $1")
        .bind(d.id)
        .execute(&t.state.pool)
        .await
        .unwrap();
    worker::tick(&t.state, &limiter()).await.unwrap();
    assert_eq!(common::status_of(&t.state.pool, id).await, "failed");
}
