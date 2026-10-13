mod common;

use mailrust::campaigns::{self, CampaignStatus};
use serde_json::json;

#[sqlx::test]
async fn a_draft_starts_empty_and_belongs_to_its_account(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let mut conn = pool.acquire().await.unwrap();
    let c = campaigns::create_draft(&mut conn, t.account.id, None).await.unwrap();
    assert_eq!(c.status, CampaignStatus::Draft);
    assert_eq!(c.subject, "");
    assert_eq!(c.blocks.0, json!({"type": "doc", "content": []}));
    assert_eq!((c.wave, c.archive_hidden), (0, false));
    assert!(campaigns::get(&pool, Some(t.account.id), c.id).await.unwrap().is_some());
    assert!(campaigns::get(&pool, None, c.id).await.unwrap().is_some());
    let other = mailrust::accounts::create(&pool, "Other").await.unwrap();
    assert!(campaigns::get(&pool, Some(other.id), c.id).await.unwrap().is_none());
}

#[sqlx::test]
async fn campaign_emails_need_no_body_and_one_issue_sends_at_a_time(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let mut conn = pool.acquire().await.unwrap();
    let a = campaigns::create_draft(&mut conn, t.account.id, None).await.unwrap();
    let b = campaigns::create_draft(&mut conn, t.account.id, None).await.unwrap();

    let insert = "INSERT INTO emails (id, account_id, from_address, to_addresses, subject, campaign_id)
                  VALUES (gen_random_uuid(), $1, 'a@x.com', ARRAY['b@x.com'], 's', $2)";
    sqlx::query(insert).bind(t.account.id).bind(a.id).execute(&pool).await.expect("a campaign row needs no body");
    let no_campaign: Option<uuid::Uuid> = None;
    assert!(
        sqlx::query(insert).bind(t.account.id).bind(no_campaign).execute(&pool).await.is_err(),
        "a normal email still needs html or text"
    );

    let sending = "UPDATE campaigns SET status = 'sending' WHERE id = $1";
    sqlx::query(sending).bind(a.id).execute(&pool).await.unwrap();
    assert!(sqlx::query(sending).bind(b.id).execute(&pool).await.is_err(), "campaigns_one_sending");
}

#[sqlx::test]
async fn lock_skip_skips_a_row_another_transaction_holds(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let mut conn = pool.acquire().await.unwrap();
    let c = campaigns::create_draft(&mut conn, t.account.id, None).await.unwrap();
    let mut first = pool.begin().await.unwrap();
    assert!(campaigns::lock_skip(&mut first, c.id).await.unwrap().is_some());
    let mut second = pool.begin().await.unwrap();
    assert!(campaigns::lock_skip(&mut second, c.id).await.unwrap().is_none());
}
