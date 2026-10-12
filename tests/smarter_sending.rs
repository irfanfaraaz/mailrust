mod common;

use std::collections::HashSet;

use mailrust::{queue, quota, sender::SendQuota};

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
