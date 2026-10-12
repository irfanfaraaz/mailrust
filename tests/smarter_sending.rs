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
    assert!(
        quota::load(&pool)
            .await
            .unwrap()
            .unwrap()
            .paused_until
            .is_some()
    );
    // 190 of 200 is still inside the urgent reserve: SES's count lags, so a near-full answer must not end the pause.
    quota::save(&pool, &q(14.0, 200.0, 190.0)).await.unwrap();
    assert!(
        quota::load(&pool)
            .await
            .unwrap()
            .unwrap()
            .paused_until
            .is_some()
    );
    quota::save(&pool, &q(14.0, 200.0, 150.0)).await.unwrap();
    assert!(
        quota::load(&pool)
            .await
            .unwrap()
            .unwrap()
            .paused_until
            .is_none()
    );
    drop(t);
}

#[sqlx::test]
async fn nothing_is_claimed_while_paused(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    common::seed_emails(&pool, t.account.id, "urgent", "queued", 3, 0.0).await;
    quota::pause(&pool, "cap").await.unwrap();
    assert!(queue::claim(&pool, 10).await.unwrap().is_empty());
}

#[sqlx::test]
async fn near_the_daily_quota_only_urgent_goes(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    common::seed_emails(&pool, t.account.id, "bulk", "queued", 3, 1.0).await;
    let urgent = common::seed_emails(&pool, t.account.id, "urgent", "queued", 1, 0.0).await;
    quota::save(&pool, &q(14.0, 1000.0, 900.0)).await.unwrap();
    let got: Vec<_> = queue::claim(&pool, 10)
        .await
        .unwrap()
        .into_iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(got, urgent);
    quota::save(&pool, &q(14.0, 1000.0, 100.0)).await.unwrap();
    assert_eq!(queue::claim(&pool, 10).await.unwrap().len(), 3);
}

#[sqlx::test]
async fn the_reserve_counts_mail_sent_since_the_last_sync(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    quota::save(&pool, &q(14.0, 1000.0, 850.0)).await.unwrap();
    // 60 recipients sent after the sync: 850 + 60 = 910, over 90% of 1,000.
    let sent = common::seed_emails(&pool, t.account.id, "normal", "sent", 60, 0.0).await;
    sqlx::query("UPDATE emails SET sent_at = now() WHERE id = ANY($1)")
        .bind(sent)
        .execute(&pool)
        .await
        .unwrap();
    common::seed_emails(&pool, t.account.id, "bulk", "queued", 1, 0.0).await;
    assert!(queue::claim(&pool, 10).await.unwrap().is_empty());
}

#[sqlx::test]
async fn after_a_daily_cap_hit_the_next_round_claims_nothing(pool: sqlx::PgPool) {
    let s = common::RecordingSender::failing(|| {
        SenderError::DailyQuota("Daily message quota exceeded".into())
    });
    let t = common::app_with_sender(pool.clone(), s).await;
    common::verified_domain(&t, "shop.com").await;
    common::send_email(
        &t,
        serde_json::json!({ "from": "hi@shop.com", "to": "a@x.com", "subject": "Hi", "text": "x" }),
    )
    .await;
    let limiter = SendLimiter::new(10_000, 1);
    assert_eq!(worker::tick(&t.state, &limiter).await.unwrap(), 1);
    assert_eq!(worker::tick(&t.state, &limiter).await.unwrap(), 0);
}

#[sqlx::test]
async fn the_worker_paces_itself_by_the_synced_rate(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let limiter = SendLimiter::new(t.state.config.ses_max_send_rate, 1); // 14
    quota::save(&pool, &q(2.0, 1000.0, 0.0)).await.unwrap();
    worker::tick(&t.state, &limiter).await.unwrap();
    assert_eq!(limiter.global_rate(), 2);
    quota::save(&pool, &q(500.0, 1000.0, 0.0)).await.unwrap();
    worker::tick(&t.state, &limiter).await.unwrap();
    assert_eq!(limiter.global_rate(), 14);
}

#[sqlx::test]
async fn stale_quota_numbers_do_not_hold_back_normal_mail(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    common::seed_emails(&pool, t.account.id, "bulk", "queued", 1, 0.0).await;
    quota::save(&pool, &q(14.0, 1000.0, 950.0)).await.unwrap();
    sqlx::query("UPDATE sender_quota SET synced_at = now() - interval '1 hour'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(queue::claim(&pool, 10).await.unwrap().len(), 1);
}

#[sqlx::test]
async fn non_positive_ses_numbers_are_ignored(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    common::seed_emails(&pool, t.account.id, "bulk", "queued", 1, 0.0).await;
    quota::pause(&pool, "cap").await.unwrap();
    quota::save(&pool, &q(-1.0, -1.0, 5.0)).await.unwrap(); // -1: no limit
    assert!(
        quota::load(&pool)
            .await
            .unwrap()
            .unwrap()
            .paused_until
            .is_none()
    );
    assert_eq!(queue::claim(&pool, 10).await.unwrap().len(), 1);
}

#[sqlx::test]
async fn emails_already_claimed_wait_once_the_pause_starts(pool: sqlx::PgPool) {
    let s = common::RecordingSender::failing(|| {
        SenderError::DailyQuota("Daily message quota exceeded".into())
    });
    let mut t = common::app_with_sender(pool.clone(), s.clone()).await;
    let mut cfg = (*t.state.config).clone();
    cfg.worker_concurrency = 1; // one at a time, so the second email sees the pause
    t.state.config = std::sync::Arc::new(cfg);
    common::verified_domain(&t, "shop.com").await;
    let mut ids = vec![];
    for to in ["a@x.com", "b@x.com"] {
        ids.push(
            common::send_email(
                &t,
                serde_json::json!({ "from": "hi@shop.com", "to": to, "subject": "Hi", "text": "x" }),
            )
            .await,
        );
    }
    worker::tick(&t.state, &SendLimiter::new(10_000, 1))
        .await
        .unwrap();
    assert_eq!(s.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    for id in ids {
        assert_eq!(common::status_of(&pool, id).await, "queued");
        let attempts: i32 = sqlx::query_scalar("SELECT attempts FROM emails WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(attempts, 0);
    }
}

#[sqlx::test]
async fn one_gate_read_gives_the_rate_too(pool: sqlx::PgPool) {
    assert_eq!(quota::gate(&pool).await.unwrap().max_send_rate, None);
    quota::save(&pool, &q(2.0, 1000.0, 0.0)).await.unwrap();
    assert_eq!(quota::gate(&pool).await.unwrap().max_send_rate, Some(2.0));
}
