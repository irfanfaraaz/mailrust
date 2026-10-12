mod common;

use axum::http::StatusCode;
use mailrust::{
    accounts::{self, AccountStatus},
    events,
    limiter::SendLimiter,
    safety, worker,
};
use serde_json::json;
use uuid::Uuid;

async fn seed(pool: &sqlx::PgPool, account: Uuid, email: Uuid, sent: usize, bounced: usize) {
    let recipients: Vec<String> = (0..sent).map(|i| format!("u{i}@x.com")).collect();
    sqlx::query("INSERT INTO email_events (id, account_id, email_id, type, data) VALUES ($1, $2, $3, 'sent', $4)")
        .bind(Uuid::now_v7()).bind(account).bind(email).bind(json!({ "recipients": recipients }))
        .execute(pool).await.unwrap();
    for i in 0..bounced {
        sqlx::query("INSERT INTO email_events (id, account_id, email_id, type, recipient) VALUES ($1, $2, $3, 'bounced', $4)")
            .bind(Uuid::now_v7()).bind(account).bind(email).bind(format!("u{i}@x.com"))
            .execute(pool).await.unwrap();
    }
}

#[sqlx::test]
async fn too_many_bounces_pause_only_that_account(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    common::verified_domain(&t, "shop.com").await;
    let id = common::send_email(
        &t,
        json!({ "from": "hi@shop.com", "to": "a@x.com", "subject": "Hi", "text": "x" }),
    )
    .await;
    let other = accounts::create(&t.state.pool, "Other").await.unwrap();

    seed(&t.state.pool, t.account.id, id, 100, 2).await;
    assert!(
        safety::check_account(&t.state.pool, t.account.id)
            .await
            .unwrap()
            .is_none()
    );
    seed(&t.state.pool, t.account.id, id, 0, 1).await;
    let reason = safety::check_account(&t.state.pool, t.account.id)
        .await
        .unwrap();
    assert!(reason.unwrap().contains("bounce rate"));
    assert!(
        safety::check_account(&t.state.pool, t.account.id)
            .await
            .unwrap()
            .is_none(),
        "already paused"
    );

    assert_eq!(
        accounts::get(&t.state.pool, t.account.id)
            .await
            .unwrap()
            .unwrap()
            .status,
        AccountStatus::Paused
    );
    assert_eq!(
        accounts::get(&t.state.pool, other.id)
            .await
            .unwrap()
            .unwrap()
            .status,
        AccountStatus::Active
    );

    let (s, b) = common::call(
        &t.router,
        common::req(
            "POST",
            "/emails",
            Some(&t.key),
            Some(json!({ "from": "hi@shop.com", "to": "a@x.com", "subject": "Hi", "text": "x" })),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(b["error"]["code"], "account_paused");
}

#[sqlx::test]
async fn simulated_bounces_trip_the_pause_through_the_worker(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    common::verified_domain(&t, "shop.com").await;
    let id = common::send_email(
        &t,
        json!({ "from": "hi@shop.com", "to": "a@x.com", "subject": "Hi", "text": "x" }),
    )
    .await;
    seed(&t.state.pool, t.account.id, id, 120, 0).await;
    for _ in 0..3 {
        common::send_email(&t, json!({ "from": "hi@shop.com", "to": events::SIM_BOUNCE, "subject": "Hi", "text": "x" })).await;
    }
    worker::tick(&t.state, &SendLimiter::new(10_000, 1))
        .await
        .unwrap();
    assert_eq!(
        accounts::get(&t.state.pool, t.account.id)
            .await
            .unwrap()
            .unwrap()
            .status,
        AccountStatus::Paused
    );
}

#[sqlx::test]
async fn ses_bounce_events_trip_the_pause(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    common::verified_domain(&t, "shop.com").await;
    let id = common::send_email(
        &t,
        json!({ "from": "hi@shop.com", "to": "a@x.com", "subject": "Hi", "text": "x" }),
    )
    .await;
    seed(&t.state.pool, t.account.id, id, 100, 2).await;
    let ev: events::SesEvent = serde_json::from_value(json!({
        "eventType": "Bounce",
        "mail": { "messageId": "ses-1", "tags": { "mailrust_email_id": [id.to_string()] } },
        "bounce": { "bounceType": "Permanent", "bouncedRecipients": [{ "emailAddress": "u9@x.com" }] }
    }))
    .unwrap();
    events::ingest(&t.state.pool, &ev, None).await.unwrap();
    let acc = accounts::get(&t.state.pool, t.account.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        acc.status,
        AccountStatus::Paused,
        "3 of 100 bounced must pause the account"
    );
}
