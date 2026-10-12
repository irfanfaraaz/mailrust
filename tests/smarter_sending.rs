mod common;

use std::collections::HashSet;

use mailrust::queue;

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
