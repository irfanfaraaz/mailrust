mod common;

use std::collections::HashSet;

use mailrust::{
    limiter::SendLimiter,
    queue, quota,
    sender::{SendQuota, SenderError},
    worker,
};

#[sqlx::test]
async fn urgent_jumps_a_long_bulk_line_and_normal_beats_bulk(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    common::seed_emails(&pool, t.account.id, "bulk", "queued", 1000, 1.0).await;
    let normal = common::seed_emails(&pool, t.account.id, "normal", "queued", 1, 0.5).await;
    let urgent = common::seed_emails(&pool, t.account.id, "urgent", "queued", 1, 0.0).await;
    let got: HashSet<_> = queue::claim(&pool, 2)
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(got, HashSet::from([urgent[0], normal[0]]));
}

fn q(rate: f64, max: f64, sent: f64) -> SendQuota {
    SendQuota {
        max_send_rate: rate,
        max_24h_send: max,
        sent_last_24h: sent,
    }
}

#[sqlx::test]
async fn sync_saves_the_numbers_and_keeps_them_on_error(pool: sqlx::PgPool) {
    let s = common::RecordingSender::ok();
    let t = common::app_with_sender(pool.clone(), s.clone()).await;
    *s.quota.lock().unwrap() = Some(Ok(q(14.0, 50_000.0, 10.0)));
    quota::sync_once(&t.state).await.unwrap();
    let row = quota::load(&pool).await.unwrap().unwrap();
    assert_eq!(
        (row.max_send_rate, row.max_24h_send, row.sent_last_24h),
        (Some(14.0), Some(50_000.0), Some(10.0))
    );
    assert!(row.synced_at.is_some());
    *s.quota.lock().unwrap() = Some(Err("aws down".into()));
    assert!(quota::sync_once(&t.state).await.is_err());
    assert_eq!(
        quota::load(&pool).await.unwrap().unwrap().max_send_rate,
        Some(14.0)
    );
}

#[sqlx::test]
async fn a_sender_without_limits_writes_nothing(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    quota::sync_once(&t.state).await.unwrap();
    assert!(quota::load(&pool).await.unwrap().is_none());
}

#[sqlx::test]
async fn a_daily_cap_answer_pauses_everyone_and_uses_no_try(pool: sqlx::PgPool) {
    let s = common::RecordingSender::failing(|| {
        SenderError::DailyQuota("Daily message quota exceeded".into())
    });
    let t = common::app_with_sender(pool.clone(), s).await;
    common::verified_domain(&t, "shop.com").await;
    let id = common::send_email(
        &t,
        serde_json::json!({ "from": "hi@shop.com", "to": "a@x.com", "subject": "Hi", "text": "x" }),
    )
    .await;
    worker::tick(&t.state, &SendLimiter::new(10_000, 1))
        .await
        .unwrap();
    assert_eq!(common::status_of(&pool, id).await, "queued");
    let attempts: i32 = sqlx::query_scalar("SELECT attempts FROM emails WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(attempts, 0);
    let row = quota::load(&pool).await.unwrap().unwrap(); // no row before: the pause creates it
    assert!(row.paused_until.unwrap() > chrono::Utc::now() + chrono::Duration::minutes(14));
    assert!(row.pause_reason.unwrap().contains("Daily message quota"));
}

#[sqlx::test]
async fn sync_ends_a_pause_only_when_there_is_room(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    quota::pause(&pool, "cap").await.unwrap();
    quota::save(&pool, &q(14.0, 200.0, 200.0)).await.unwrap();
    assert!(quota::load(&pool).await.unwrap().unwrap().paused_until.is_some());
    quota::save(&pool, &q(14.0, 200.0, 150.0)).await.unwrap();
    assert!(quota::load(&pool).await.unwrap().unwrap().paused_until.is_none());
    drop(t);
}
