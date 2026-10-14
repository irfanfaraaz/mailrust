mod common;

use axum::http::StatusCode;

#[sqlx::test]
async fn overview_counts_events_in_both_windows(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let ids = common::seed_emails(&pool, t.account.id, "normal", "sent", 3, 0.0).await;
    for (i, id) in ids.iter().enumerate() {
        let at = if i == 0 {
            "now() - interval '3 days'"
        } else {
            "now()"
        };
        sqlx::query(&format!(
            "INSERT INTO email_events (id, email_id, account_id, type, data, created_at)
             VALUES (gen_random_uuid(), $1, $2, 'sent', '{{\"recipients\":[\"to@x.com\"]}}', {at}),
                    (gen_random_uuid(), $1, $2, 'delivered', '{{}}', {at})"
        ))
        .bind(id)
        .bind(t.account.id)
        .execute(&pool)
        .await
        .unwrap();
    }
    sqlx::query("INSERT INTO email_events (id, email_id, account_id, type, data) VALUES (gen_random_uuid(), $1, $2, 'bounced', '{}')")
        .bind(ids[1]).bind(t.account.id).execute(&pool).await.unwrap();

    let (s, b) = common::call(
        &t.router,
        common::req("GET", "/stats/overview", Some(&t.key), None),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["last_24h"]["sent"], 2);
    assert_eq!(b["last_7d"]["sent"], 3);
    assert_eq!(b["last_7d"]["delivered"], 3);
    assert_eq!(b["last_7d"]["bounced"], 1);
    assert!((b["bounce_rate"].as_f64().unwrap() - 1.0 / 3.0).abs() < 1e-9);
    assert_eq!(b["bounce_limit"], 0.02);
    assert_eq!(b["bounce_min_sent"], 100);
    assert_eq!(b["complaint_min_sent"], 1250);
    assert_eq!(b["status"], "active");
}
