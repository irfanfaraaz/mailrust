mod common;

use axum::http::StatusCode;
use mailrust::campaigns::{self, CampaignStatus};
use serde_json::json;

#[sqlx::test]
async fn a_draft_starts_empty_and_belongs_to_its_account(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let mut conn = pool.acquire().await.unwrap();
    let c = campaigns::create_draft(&mut conn, t.account.id, None)
        .await
        .unwrap();
    assert_eq!(c.status, CampaignStatus::Draft);
    assert_eq!(c.subject, "");
    assert_eq!(c.blocks.0, json!({"type": "doc", "content": []}));
    assert_eq!((c.wave, c.archive_hidden), (0, false));
    assert!(
        campaigns::get(&pool, Some(t.account.id), c.id)
            .await
            .unwrap()
            .is_some()
    );
    assert!(campaigns::get(&pool, None, c.id).await.unwrap().is_some());
    let other = mailrust::accounts::create(&pool, "Other").await.unwrap();
    assert!(
        campaigns::get(&pool, Some(other.id), c.id)
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test]
async fn campaign_emails_need_no_body_and_one_issue_sends_at_a_time(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let mut conn = pool.acquire().await.unwrap();
    let a = campaigns::create_draft(&mut conn, t.account.id, None)
        .await
        .unwrap();
    let b = campaigns::create_draft(&mut conn, t.account.id, None)
        .await
        .unwrap();

    let insert =
        "INSERT INTO emails (id, account_id, from_address, to_addresses, subject, campaign_id)
                  VALUES (gen_random_uuid(), $1, 'a@x.com', ARRAY['b@x.com'], 's', $2)";
    sqlx::query(insert)
        .bind(t.account.id)
        .bind(a.id)
        .execute(&pool)
        .await
        .expect("a campaign row needs no body");
    let no_campaign: Option<uuid::Uuid> = None;
    assert!(
        sqlx::query(insert)
            .bind(t.account.id)
            .bind(no_campaign)
            .execute(&pool)
            .await
            .is_err(),
        "a normal email still needs html or text"
    );

    let sending = "UPDATE campaigns SET status = 'sending' WHERE id = $1";
    sqlx::query(sending)
        .bind(a.id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        sqlx::query(sending)
            .bind(b.id)
            .execute(&pool)
            .await
            .is_err(),
        "campaigns_one_sending"
    );
}

#[sqlx::test]
async fn lock_skip_skips_a_row_another_transaction_holds(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let mut conn = pool.acquire().await.unwrap();
    let c = campaigns::create_draft(&mut conn, t.account.id, None)
        .await
        .unwrap();
    let mut first = pool.begin().await.unwrap();
    assert!(
        campaigns::lock_skip(&mut first, c.id)
            .await
            .unwrap()
            .is_some()
    );
    let mut second = pool.begin().await.unwrap();
    assert!(
        campaigns::lock_skip(&mut second, c.id)
            .await
            .unwrap()
            .is_none()
    );
}

async fn upload(
    t: &common::TestApp,
    bytes: &[u8],
    declared: &str,
) -> (StatusCode, serde_json::Value) {
    common::call(
        &t.router,
        common::multipart_file("/images", &t.key, bytes, declared),
    )
    .await
}

#[sqlx::test]
async fn an_uploaded_image_is_served_with_safe_headers(pool: sqlx::PgPool) {
    let mut t = common::app(pool).await;
    common::with_web_secret(&mut t);
    // The declared type is ignored: the bytes decide.
    let (s, b) = upload(&t, &common::png_1x1(), "image/svg+xml").await;
    assert_eq!(s, StatusCode::CREATED, "{b}");
    let id = b["id"].as_str().unwrap().to_string();
    assert_eq!(b["url"], format!("http://localhost:3000/img/{id}"));
    assert_eq!(
        (b["width"].as_u64(), b["height"].as_u64()),
        (Some(1), Some(1))
    );

    let (s, h, _) = common::call_raw(
        &t.router,
        common::web_req("GET", &format!("/public/img/{id}"), "10.0.0.1", None),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(h["content-type"], "image/png");
    assert_eq!(h["x-content-type-options"], "nosniff");
    assert_eq!(h["cache-control"], "public, max-age=31536000, immutable");

    let (s, _, _) = common::call_raw(
        &t.router,
        common::req("GET", &format!("/public/img/{id}"), None, None),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND, "web secret required");
    let (s, _, _) = common::call_raw(
        &t.router,
        common::web_req(
            "GET",
            &format!("/public/img/{}", uuid::Uuid::now_v7()),
            "10.0.0.1",
            None,
        ),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn svg_unknown_and_huge_uploads_are_refused(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let (s, b) = upload(
        &t,
        b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script>x</script></svg>",
        "image/png",
    )
    .await;
    assert_eq!(s, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(b["error"]["code"], "unsupported_media_type");
    let (s, _) = upload(&t, b"%PDF-1.7", "image/png").await;
    assert_eq!(s, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    let mut big = common::png_1x1();
    big.resize(mailrust::images::MAX_IMAGE_BYTES + 1, 0);
    let (s, _) = upload(&t, &big, "image/png").await;
    assert_eq!(s, StatusCode::PAYLOAD_TOO_LARGE);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM images")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0, "nothing stored");
}

#[sqlx::test]
async fn a_sending_key_cannot_upload(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let key = mailrust::keys::create(
        &pool,
        t.account.id,
        "send",
        mailrust::keys::KeyAccess::Sending,
    )
    .await
    .unwrap()
    .token;
    let (s, _) = common::call(
        &t.router,
        common::multipart_file("/images", &key, &common::png_1x1(), "image/png"),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn the_store_knows_the_owner(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let (_, b) = upload(&t, &common::png_1x1(), "image/png").await;
    let id: uuid::Uuid = b["id"].as_str().unwrap().parse().unwrap();
    assert_eq!(t.state.images.owner(id).await.unwrap(), Some(t.account.id));
    assert_eq!(
        t.state.images.owner(uuid::Uuid::now_v7()).await.unwrap(),
        None
    );
}
