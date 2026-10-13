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

fn doc_with(text: &str) -> serde_json::Value {
    json!({"type": "doc", "content": [{"type": "paragraph", "content": [{"type": "text", "text": text}]}]})
}

async fn api(
    t: &common::TestApp,
    method: &str,
    uri: &str,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value) {
    limited(|| common::req(method, uri, Some(&t.key), body.clone()), t).await
}

/// Calls again after a 429: the 10/s key limit answers before any handler runs.
async fn limited(
    mk: impl Fn() -> axum::http::Request<axum::body::Body>,
    t: &common::TestApp,
) -> (StatusCode, serde_json::Value) {
    loop {
        let (s, b) = common::call(&t.router, mk()).await;
        if s != StatusCode::TOO_MANY_REQUESTS {
            return (s, b);
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

#[sqlx::test]
async fn a_campaign_is_created_edited_listed_and_deleted(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let (s, tag) = api(&t, "POST", "/tags", Some(json!({"name": "vip"}))).await;
    assert_eq!(s, StatusCode::CREATED);
    let (s, c) = api(
        &t,
        "POST",
        "/campaigns",
        Some(json!({"subject": "  Hello  ", "blocks": doc_with("Hi")})),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{c}");
    assert_eq!(
        (c["subject"].as_str(), c["status"].as_str()),
        (Some("Hello"), Some("draft"))
    );
    assert!(c.get("account_id").is_none(), "never in the API");
    let id = c["id"].as_str().unwrap().to_string();

    let (s, c) = api(
        &t,
        "PATCH",
        &format!("/campaigns/{id}"),
        Some(json!({
            "preview_text": "Short", "include_tags": [tag["id"]], "blocks": doc_with("Bye")
        })),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{c}");
    assert_eq!(c["preview_text"], "Short");
    assert_eq!(c["include_tags"], json!([tag["id"]]));
    assert_eq!(c["blocks"]["content"][0]["content"][0]["text"], "Bye");
    let (_, c) = api(
        &t,
        "PATCH",
        &format!("/campaigns/{id}"),
        Some(json!({"preview_text": ""})),
    )
    .await;
    assert!(c["preview_text"].is_null());
    assert_eq!(c["subject"], "Hello", "absent fields stay");

    let (s, l) = api(&t, "GET", "/campaigns", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(l["data"][0]["id"], id.as_str());
    assert_eq!(l["data"][0]["recipients"], 0);

    let (s, _) = api(&t, "DELETE", &format!("/campaigns/{id}"), None).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let (s, _) = api(&t, "GET", &format!("/campaigns/{id}"), None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn bad_input_is_refused(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let (s, _) = api(
        &t,
        "POST",
        "/campaigns",
        Some(json!({"subject": "x".repeat(201)})),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = api(
        &t,
        "POST",
        "/campaigns",
        Some(json!({"subject": "two\nlines"})),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = api(
        &t,
        "POST",
        "/campaigns",
        Some(json!({"preview_text": "x".repeat(201)})),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, b) = api(
        &t,
        "POST",
        "/campaigns",
        Some(json!({"blocks": {"type": "doc", "content": [{"type": "table"}]}})),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{b}");
    let huge = doc_with(&"x".repeat(600 * 1024));
    let (s, b) = api(&t, "POST", "/campaigns", Some(json!({"blocks": huge}))).await;
    assert_eq!(
        (s, b["error"]["code"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("blocks_too_large"))
    );
    // A half-written merge tag (no fallback yet) still saves: the rules apply at send.
    let draft = json!({"type": "doc", "content": [{"type": "paragraph", "content": [{"type": "mergeTag", "attrs": {"key": "first_name"}}]}]});
    let (s, _) = api(&t, "POST", "/campaigns", Some(json!({"blocks": draft}))).await;
    assert_eq!(s, StatusCode::CREATED);

    let other = mailrust::accounts::create(&pool, "Other").await.unwrap();
    let foreign: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO tags (id, account_id, name) VALUES (gen_random_uuid(), $1, 'x') RETURNING id",
    )
    .bind(other.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let (s, _) = api(
        &t,
        "POST",
        "/campaigns",
        Some(json!({"exclude_tags": [foreign]})),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn only_drafts_and_scheduled_issues_can_change(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let (_, c) = api(&t, "POST", "/campaigns", Some(json!({"subject": "Hi"}))).await;
    let id: uuid::Uuid = c["id"].as_str().unwrap().parse().unwrap();
    let uri = format!("/campaigns/{id}");

    sqlx::query("UPDATE campaigns SET status = 'scheduled', send_at = now() + interval '1 day' WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let (s, c) = api(&t, "PATCH", &uri, Some(json!({"subject": "Changed"}))).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(
        (c["status"].as_str(), c["send_at"].is_null()),
        (Some("draft"), true),
        "an edit unschedules it"
    );

    sqlx::query("UPDATE campaigns SET status = 'sent' WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let (s, b) = api(&t, "PATCH", &uri, Some(json!({"subject": "Late"}))).await;
    assert_eq!(
        (s, b["error"]["code"].as_str()),
        (StatusCode::CONFLICT, Some("campaign_locked"))
    );
    let (s, c) = api(&t, "PATCH", &uri, Some(json!({"archive_hidden": true}))).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(c["archive_hidden"], true);
    let (s, b) = api(&t, "DELETE", &uri, None).await;
    assert_eq!(
        (s, b["error"]["code"].as_str()),
        (StatusCode::CONFLICT, Some("campaign_not_draft"))
    );
}

#[sqlx::test]
async fn campaigns_are_private_to_their_account_and_need_a_full_key(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let (_, c) = api(&t, "POST", "/campaigns", Some(json!({"subject": "Mine"}))).await;
    let id = c["id"].as_str().unwrap().to_string();
    let u = common::app(pool.clone()).await;
    for (m, body) in [
        ("GET", None),
        ("PATCH", Some(json!({"subject": "x"}))),
        ("DELETE", None),
    ] {
        let (s, _) = api(&u, m, &format!("/campaigns/{id}"), body).await;
        assert_eq!(s, StatusCode::NOT_FOUND, "{m}");
    }
    let (_, l) = api(&u, "GET", "/campaigns", None).await;
    assert_eq!(l["data"], json!([]));
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
        common::req("GET", "/campaigns", Some(&key), None),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn the_list_pages_newest_first(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    for i in 0..3 {
        api(
            &t,
            "POST",
            "/campaigns",
            Some(json!({"subject": format!("n{i}")})),
        )
        .await;
    }
    let (_, a) = api(&t, "GET", "/campaigns?limit=2", None).await;
    assert_eq!(
        (
            a["data"][0]["subject"].as_str(),
            a["data"][1]["subject"].as_str()
        ),
        (Some("n2"), Some("n1"))
    );
    let cursor = a["next_cursor"].as_str().unwrap();
    let (_, b) = api(
        &t,
        "GET",
        &format!("/campaigns?limit=2&after={cursor}"),
        None,
    )
    .await;
    assert_eq!(b["data"].as_array().unwrap().len(), 1);
    assert!(b["next_cursor"].is_null());
}

/// Verified news.shop.com, from + name + postal address set.
async fn ready(t: &common::TestApp) {
    common::verified_domain(t, "news.shop.com").await;
    let (s, b) = api(t, "PATCH", "/newsletter", Some(json!({
        "newsletter_name": "Shop Weekly", "newsletter_from": "news@news.shop.com", "postal_address": "1 Main St"
    }))).await;
    assert_eq!(s, StatusCode::OK, "{b}");
}

async fn contact(
    t: &common::TestApp,
    email: &str,
    first: Option<&str>,
    tags: &[&str],
) -> uuid::Uuid {
    let (s, b) = api(
        t,
        "POST",
        "/contacts",
        Some(json!({"email": email, "first_name": first, "tags": tags})),
    )
    .await;
    assert!(s.is_success(), "{b}");
    b["id"].as_str().unwrap().parse().unwrap()
}

async fn preview(t: &common::TestApp, id: &str) -> serde_json::Value {
    let (s, p) = api(t, "POST", &format!("/campaigns/{id}/preview"), None).await;
    assert_eq!(s, StatusCode::OK, "{p}");
    p
}

fn failed(p: &serde_json::Value) -> Vec<String> {
    p["checks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["ok"] == false)
        .map(|c| c["code"].as_str().unwrap().to_string())
        .collect()
}

fn greeting(fallback: Option<&str>) -> serde_json::Value {
    json!({"type": "doc", "content": [{"type": "paragraph", "content": [
        {"type": "text", "text": "Hi "}, {"type": "mergeTag", "attrs": {"key": "first_name", "fallback": fallback}}
    ]}]})
}

#[sqlx::test]
async fn a_fresh_campaign_fails_every_check_that_applies(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let (_, c) = api(&t, "POST", "/campaigns", Some(json!({}))).await;
    let p = preview(&t, c["id"].as_str().unwrap()).await;
    assert_eq!(
        failed(&p),
        vec!["from_address", "subject", "postal_address", "readers"]
    );
    assert_eq!(p["readers"], 0);
    assert_eq!(p["waves"], json!([]));
    assert_eq!(
        p["warnings"].as_array().unwrap().len(),
        1,
        "preview text is empty"
    );
}

#[sqlx::test]
async fn a_ready_campaign_passes_and_previews_with_fallbacks(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    ready(&t).await;
    contact(&t, "ann@x.com", Some("Ann"), &[]).await;
    contact(&t, "bob@x.com", None, &[]).await;
    let (_, c) = api(
        &t,
        "POST",
        "/campaigns",
        Some(
            json!({"subject": "Hello", "preview_text": "News", "blocks": greeting(Some("there"))}),
        ),
    )
    .await;
    let p = preview(&t, c["id"].as_str().unwrap()).await;
    assert_eq!(failed(&p), Vec::<String>::new(), "{p}");
    assert_eq!(
        (p["readers"].as_i64(), p["missing_first_name"].as_i64()),
        (Some(2), Some(1))
    );
    assert_eq!(p["first_name_fallback"], "there");
    assert_eq!(p["waves"], json!([2]));
    assert!(p["html"].as_str().unwrap().contains("Hi there"));
    assert!(p["text"].as_str().unwrap().contains("Hi there"));
    assert!(p["size_bytes"].as_u64().unwrap() > 1000);
    assert_eq!(p["warnings"], json!([]));
}

#[sqlx::test]
async fn block_rules_name_what_is_wrong(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    ready(&t).await;
    contact(&t, "ann@x.com", None, &[]).await;
    let check = |blocks: serde_json::Value| {
        let t = &t;
        async move {
            let (_, c) = api(
                t,
                "POST",
                "/campaigns",
                Some(json!({"subject": "S", "blocks": blocks})),
            )
            .await;
            failed(&preview(t, c["id"].as_str().unwrap()).await)
        }
    };
    assert_eq!(check(greeting(None)).await, vec!["merge_tag_fallback"]);
    let js = json!({"type": "doc", "content": [{"type": "button", "attrs": {"label": "Go", "href": "javascript:alert(1)"}}]});
    assert_eq!(check(js).await, vec!["bad_blocks"]);

    let u = common::app(pool).await;
    let (_, img) = common::call(
        &u.router,
        common::multipart_file("/images", &u.key, &common::png_1x1(), "image/png"),
    )
    .await;
    let theirs = json!({"type": "doc", "content": [{"type": "image", "attrs": {"id": img["id"], "alt": "x"}}]});
    assert_eq!(
        check(theirs).await,
        vec!["image_not_found"],
        "another account's image"
    );
    let (_, mine) = limited(
        || common::multipart_file("/images", &t.key, &common::png_1x1(), "image/png"),
        &t,
    )
    .await;
    let ok = json!({"type": "doc", "content": [{"type": "image", "attrs": {"id": mine["id"], "alt": "x"}}]});
    assert_eq!(check(ok).await, Vec::<String>::new());
}

#[sqlx::test]
async fn a_huge_issue_and_a_paused_account_fail(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    ready(&t).await;
    contact(&t, "ann@x.com", None, &[]).await;
    let paras: Vec<_> = (0..1200)
        .map(
            |_| json!({"type": "paragraph", "content": [{"type": "text", "text": "x".repeat(90)}]}),
        )
        .collect();
    let (_, c) = api(
        &t,
        "POST",
        "/campaigns",
        Some(json!({"subject": "Big", "blocks": {"type": "doc", "content": paras}})),
    )
    .await;
    let id = c["id"].as_str().unwrap().to_string();
    assert_eq!(failed(&preview(&t, &id).await), vec!["size"]);
    mailrust::accounts::pause(&pool, t.account.id, "test")
        .await
        .unwrap();
    assert_eq!(
        failed(&preview(&t, &id).await),
        vec!["size", "account_active"]
    );
}

#[sqlx::test]
async fn the_reader_count_follows_the_tags(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    ready(&t).await;
    contact(&t, "a@x.com", None, &["news"]).await;
    contact(&t, "b@x.com", None, &["news", "skip"]).await;
    contact(&t, "c@x.com", None, &[]).await;
    let d = contact(&t, "d@x.com", None, &["news"]).await;
    api(
        &t,
        "PATCH",
        &format!("/contacts/{d}"),
        Some(json!({"status": "unsubscribed"})),
    )
    .await;
    contact(&t, "e@x.com", None, &["news"]).await;
    api(
        &t,
        "POST",
        "/suppressions",
        Some(json!({"address": "e@x.com"})),
    )
    .await;
    sqlx::query("INSERT INTO contacts (id, account_id, email, status, source) VALUES (gen_random_uuid(), $1, 'p@x.com', 'pending', 'form')")
        .bind(t.account.id).execute(&pool).await.unwrap();

    let (_, tags) = api(&t, "GET", "/tags", None).await;
    let tag = |n: &str| {
        tags["data"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["name"] == n)
            .unwrap()["id"]
            .clone()
    };
    let (_, c) = api(&t, "POST", "/campaigns", Some(json!({"subject": "S"}))).await;
    let id = c["id"].as_str().unwrap().to_string();
    assert_eq!(
        preview(&t, &id).await["readers"],
        3,
        "a, b, c: not unsubscribed, suppressed or pending"
    );
    api(
        &t,
        "PATCH",
        &format!("/campaigns/{id}"),
        Some(json!({"include_tags": [tag("news")]})),
    )
    .await;
    assert_eq!(preview(&t, &id).await["readers"], 2);
    api(
        &t,
        "PATCH",
        &format!("/campaigns/{id}"),
        Some(json!({"exclude_tags": [tag("skip")]})),
    )
    .await;
    assert_eq!(
        preview(&t, &id).await["readers"],
        1,
        "skip wins over include"
    );
}

async fn member(t: &common::TestApp, email: &str) {
    common::session_for(
        &t.state.pool,
        t.account.id,
        email,
        mailrust::users::MemberRole::Member,
    )
    .await;
}

#[sqlx::test]
async fn a_test_goes_to_members_with_their_own_merge_data(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    ready(&t).await;
    member(&t, "ann@shop.com").await;
    member(&t, "bob@shop.com").await;
    contact(&t, "ann@shop.com", Some("Ann"), &[]).await;
    let (_, c) = api(
        &t,
        "POST",
        "/campaigns",
        Some(json!({"subject": "Hello", "blocks": greeting(Some("there"))})),
    )
    .await;
    let id = c["id"].as_str().unwrap();
    let (s, b) = api(
        &t,
        "POST",
        &format!("/campaigns/{id}/test"),
        Some(json!({"to": ["Ann@Shop.com", "bob@shop.com"]})),
    )
    .await;
    assert_eq!(s, StatusCode::ACCEPTED, "{b}");
    assert_eq!(b["queued"], 2);
    let rows: Vec<(String, String, Option<String>, String, String)> = sqlx::query_as(
        "SELECT to_addresses[1], subject, html, priority::text, from_address FROM emails WHERE tags->>'campaign_test' = $1 ORDER BY to_addresses[1]",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        (rows[0].0.as_str(), rows[0].1.as_str(), rows[0].3.as_str()),
        ("ann@shop.com", "[Test] Hello", "normal")
    );
    assert!(
        rows[0].2.as_deref().unwrap().contains("Hi Ann"),
        "the member's own contact row"
    );
    assert!(
        rows[1].2.as_deref().unwrap().contains("Hi there"),
        "no contact row: fallback"
    );
    assert!(
        rows[0]
            .2
            .as_deref()
            .unwrap()
            .contains("This is a test email")
    );
    assert!(rows[0].4.contains("Shop Weekly") && rows[0].4.contains("<news@news.shop.com>"));
    let campaign_rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM emails WHERE campaign_id IS NOT NULL")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(campaign_rows, 0, "tests never count in the report");
}

#[sqlx::test]
async fn test_sends_are_limited(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    ready(&t).await;
    member(&t, "ann@shop.com").await;
    let (_, c) = api(&t, "POST", "/campaigns", Some(json!({"subject": "Hello"}))).await;
    let uri = format!("/campaigns/{}/test", c["id"].as_str().unwrap());
    let (s, _) = api(&t, "POST", &uri, Some(json!({"to": []}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let six: Vec<String> = (0..6).map(|i| format!("m{i}@shop.com")).collect();
    let (s, _) = api(&t, "POST", &uri, Some(json!({"to": six}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, b) = api(&t, "POST", &uri, Some(json!({"to": ["stranger@x.com"]}))).await;
    assert_eq!(
        (s, b["error"]["code"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("not_a_member"))
    );
    for _ in 0..20 {
        let (s, _) = api(&t, "POST", &uri, Some(json!({"to": ["ann@shop.com"]}))).await;
        assert_eq!(s, StatusCode::ACCEPTED);
    }
    // `api` would retry a 429 forever; wait out the 10/s key limit, then take the answer as it is.
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let (s, _) = common::call(
        &t.router,
        common::req(
            "POST",
            &uri,
            Some(&t.key),
            Some(json!({"to": ["ann@shop.com"]})),
        ),
    )
    .await;
    assert_eq!(
        s,
        StatusCode::TOO_MANY_REQUESTS,
        "21st test email this hour"
    );
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM emails WHERE tags ? 'campaign_test'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 20, "the refused request queued nothing");
}

#[sqlx::test]
async fn a_test_needs_a_from_address_a_subject_and_valid_blocks(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    member(&t, "ann@shop.com").await;
    let (_, c) = api(&t, "POST", "/campaigns", Some(json!({"subject": "Hello"}))).await;
    let uri = format!("/campaigns/{}/test", c["id"].as_str().unwrap());
    let (s, b) = api(&t, "POST", &uri, Some(json!({"to": ["ann@shop.com"]}))).await;
    assert_eq!(
        (s, b["error"]["code"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("from_address"))
    );
    ready(&t).await;
    api(
        &t,
        "PATCH",
        &format!("/campaigns/{}", c["id"].as_str().unwrap()),
        Some(json!({"blocks": greeting(None)})),
    )
    .await;
    let (s, b) = api(&t, "POST", &uri, Some(json!({"to": ["ann@shop.com"]}))).await;
    assert_eq!(
        (s, b["error"]["code"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("merge_tag_fallback"))
    );
}

/// A sent issue with `subject`, without going through the send path (the archive only reads rows).
async fn sent_issue(t: &common::TestApp, subject: &str, slug: &str, hidden: bool) -> uuid::Uuid {
    let (_, c) = api(
        t,
        "POST",
        "/campaigns",
        Some(json!({"subject": subject, "blocks": greeting(Some("reader"))})),
    )
    .await;
    let id: uuid::Uuid = c["id"].as_str().unwrap().parse().unwrap();
    sqlx::query("UPDATE campaigns SET status = 'sent', archive_slug = $2, archive_hidden = $3, started_at = now(), finished_at = now() WHERE id = $1")
        .bind(id)
        .bind(slug)
        .bind(hidden)
        .execute(&t.state.pool)
        .await
        .unwrap();
    id
}

#[sqlx::test]
async fn the_archive_lists_and_shows_sent_issues_only(pool: sqlx::PgPool) {
    let mut t = common::app(pool.clone()).await;
    common::with_web_secret(&mut t);
    api(
        &t,
        "PATCH",
        "/newsletter",
        Some(json!({"public_slug": "weekly", "newsletter_name": "Shop Weekly"})),
    )
    .await;
    sent_issue(&t, "First <issue>", "first-issue", false).await;
    sent_issue(&t, "Secret", "secret", true).await;
    let (_, draft) = api(&t, "POST", "/campaigns", Some(json!({"subject": "Draft"}))).await;
    sqlx::query("UPDATE campaigns SET archive_slug = 'draft' WHERE id = $1")
        .bind(uuid::Uuid::parse_str(draft["id"].as_str().unwrap()).unwrap())
        .execute(&pool)
        .await
        .unwrap();

    let (s, idx) = common::call(
        &t.router,
        common::web_req("GET", "/public/archive/weekly", "10.0.0.3", None),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{idx}");
    assert_eq!(idx["newsletter_name"], "Shop Weekly");
    assert_eq!(idx["signup_open"], false);
    let issues = idx["issues"].as_array().unwrap();
    assert_eq!(issues.len(), 1);
    assert_eq!(
        (issues[0]["title"].as_str(), issues[0]["slug"].as_str()),
        (Some("First <issue>"), Some("first-issue"))
    );

    let (s, h, page) = common::call_raw(
        &t.router,
        common::web_req(
            "GET",
            "/public/archive/weekly/first-issue",
            "10.0.0.3",
            None,
        ),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(h["content-type"], "text/html; charset=utf-8");
    assert_eq!(
        h["content-security-policy"],
        mailrust::render::campaign::ARCHIVE_CSP
    );
    assert_eq!(h["x-content-type-options"], "nosniff");
    assert!(page.contains("First &lt;issue&gt;"));
    assert!(page.contains("Hi reader"), "merge tags show their fallback");
    assert!(!page.contains("/o/") && !page.contains("Unsubscribe"));

    for path in [
        "/public/archive/weekly/secret",
        "/public/archive/weekly/draft",
        "/public/archive/weekly/nope",
        "/public/archive/nobody/first-issue",
    ] {
        let (s, _, _) =
            common::call_raw(&t.router, common::web_req("GET", path, "10.0.0.3", None)).await;
        assert_eq!(s, StatusCode::NOT_FOUND, "{path}");
    }
    let (s, _) = common::call(
        &t.router,
        common::req("GET", "/public/archive/weekly", None, None),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND, "web secret required");
}
