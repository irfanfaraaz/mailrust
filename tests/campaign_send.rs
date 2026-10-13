mod common;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use mailrust::campaigns::send;
use mailrust::{limiter::SendLimiter, tokens, worker};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

/// Calls again after a 429: the 10/s key limit answers before any handler runs.
async fn api(
    t: &common::TestApp,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    loop {
        let (s, b) = common::call(
            &t.router,
            common::req(method, uri, Some(&t.key), body.clone()),
        )
        .await;
        if s != StatusCode::TOO_MANY_REQUESTS {
            return (s, b);
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

/// Verified news.shop.com, from + name + postal address set.
async fn ready(t: &common::TestApp) {
    common::verified_domain(t, "news.shop.com").await;
    let (s, b) = api(t, "PATCH", "/newsletter", Some(json!({
        "newsletter_name": "Shop Weekly", "newsletter_from": "news@news.shop.com", "postal_address": "1 Main St"
    }))).await;
    assert_eq!(s, StatusCode::OK, "{b}");
}

/// `n` subscribed readers r1@readers.test … rn@readers.test, subscribed 60 days ago (tier 3).
async fn readers(pool: &PgPool, account: Uuid, n: i64) {
    sqlx::query(
        "INSERT INTO contacts (id, account_id, email, status, source, subscribed_at)
         SELECT gen_random_uuid(), $1, 'r' || g || '@readers.test', 'subscribed', 'api', now() - interval '60 days'
         FROM generate_series(1, $2) g",
    )
    .bind(account).bind(n).execute(pool).await.unwrap();
}

async fn ids_of(pool: &PgPool, account: Uuid, emails: &[String]) -> Vec<Uuid> {
    sqlx::query_scalar("SELECT id FROM contacts WHERE account_id = $1 AND email = ANY($2)")
        .bind(account)
        .bind(emails)
        .fetch_all(pool)
        .await
        .unwrap()
}

/// A past email to each address with `opens` open events and `clicks` click events.
async fn engage(pool: &PgPool, account: Uuid, emails: &[String], opens: i32, clicks: i32) {
    sqlx::query(
        "WITH e AS (
           INSERT INTO emails (id, account_id, from_address, to_addresses, subject, text_body, status)
           SELECT gen_random_uuid(), $1, 'a@shop.com', ARRAY[x], 's', 't', 'delivered' FROM unnest($2::text[]) x RETURNING id)
         INSERT INTO email_events (id, account_id, email_id, type)
         SELECT gen_random_uuid(), $1, e.id, k FROM e,
           LATERAL (SELECT 'opened' FROM generate_series(1, $3) UNION ALL SELECT 'clicked' FROM generate_series(1, $4)) AS t(k)",
    )
    .bind(account).bind(emails).bind(opens).bind(clicks).execute(pool).await.unwrap();
}

async fn new_campaign(t: &common::TestApp, body: Value) -> Uuid {
    let (s, c) = api(t, "POST", "/campaigns", Some(body)).await;
    assert_eq!(s, StatusCode::CREATED, "{c}");
    c["id"].as_str().unwrap().parse().unwrap()
}

async fn send_now(t: &common::TestApp, id: Uuid) -> Value {
    let (s, c) = api(t, "POST", &format!("/campaigns/{id}/send"), Some(json!({}))).await;
    assert_eq!(s, StatusCode::OK, "{c}");
    c
}

async fn status(pool: &PgPool, id: Uuid) -> String {
    sqlx::query_scalar("SELECT status::text FROM campaigns WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn queued(pool: &PgPool, id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM emails WHERE campaign_id = $1 AND status = 'queued'")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Stands in for the worker in this task: every queued email of the campaign is now sent.
async fn mark_sent(pool: &PgPool, id: Uuid) {
    sqlx::query("UPDATE emails SET status = 'sent', sent_at = now() WHERE campaign_id = $1 AND status = 'queued'")
        .bind(id).execute(pool).await.unwrap();
}

fn emails(range: std::ops::RangeInclusive<i32>) -> Vec<String> {
    range.map(|i| format!("r{i}@readers.test")).collect()
}

#[sqlx::test]
async fn twelve_hundred_readers_go_out_in_four_waves_engaged_first(pool: PgPool) {
    let t = common::app(pool.clone()).await;
    ready(&t).await;
    readers(&pool, t.account.id, 1200).await;
    let engaged = emails(1101..=1150);
    engage(&pool, t.account.id, &engaged, 1, 0).await;
    let id = new_campaign(&t, json!({"subject": "Hello"})).await;
    let c = send_now(&t, id).await;
    assert_eq!(c["status"], "sending");
    assert_eq!(c["archive_slug"], "hello");

    let mut now = Utc::now();
    send::tick(&t.state, now).await.unwrap();
    let wave1: Vec<Uuid> = sqlx::query_scalar(
        "SELECT contact_id FROM campaign_recipients WHERE campaign_id = $1 AND wave = 1",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .unwrap();
    for e in ids_of(&pool, t.account.id, &engaged).await {
        assert!(wave1.contains(&e), "engaged readers are in wave 1");
    }
    let mut sizes = vec![queued(&pool, id).await];
    loop {
        mark_sent(&pool, id).await;
        send::tick(&t.state, now).await.unwrap(); // the wave is done: gap starts, or the issue is sent
        if status(&pool, id).await == "sent" {
            break;
        }
        send::tick(&t.state, now + Duration::minutes(5))
            .await
            .unwrap();
        assert_eq!(
            queued(&pool, id).await,
            0,
            "nothing new inside the 10-minute gap"
        );
        now += Duration::minutes(10) + Duration::seconds(1);
        send::tick(&t.state, now).await.unwrap();
        sizes.push(queued(&pool, id).await);
    }
    assert_eq!(sizes, vec![100, 200, 400, 500]);
    let (total, distinct): (i64, i64) = sqlx::query_as(
        "SELECT count(*), count(DISTINCT contact_id) FROM emails WHERE campaign_id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((total, distinct), (1200, 1200));
    let finished: Option<chrono::DateTime<Utc>> =
        sqlx::query_scalar("SELECT finished_at FROM campaigns WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(finished.is_some());
}

#[sqlx::test]
async fn tiers_put_clickers_then_openers_then_new_readers_first(pool: PgPool) {
    let t = common::app(pool.clone()).await;
    ready(&t).await;
    readers(&pool, t.account.id, 600).await;
    engage(&pool, t.account.id, &emails(10..=10), 1, 0).await; // opened once
    engage(&pool, t.account.id, &emails(20..=20), 0, 3).await; // clicked 3 times
    let new = emails(30..=32);
    sqlx::query(
        "UPDATE contacts SET subscribed_at = now() - interval '2 days' WHERE email = ANY($1)",
    )
    .bind(&new)
    .execute(&pool)
    .await
    .unwrap();
    let id = new_campaign(&t, json!({"subject": "Tiers"})).await;
    send_now(&t, id).await;
    let order: Vec<(String, i16)> = sqlx::query_as(
        "SELECT c.email, r.tier FROM campaign_recipients r JOIN contacts c ON c.id = r.contact_id WHERE r.campaign_id = $1 ORDER BY r.position LIMIT 5",
    )
    .bind(id).fetch_all(&pool).await.unwrap();
    assert_eq!(order[0], ("r20@readers.test".into(), 1), "clicks first");
    assert_eq!(order[1], ("r10@readers.test".into(), 1));
    let next: std::collections::BTreeSet<String> = order[2..]
        .iter()
        .map(|(e, tier)| {
            assert_eq!(*tier, 2);
            e.clone()
        })
        .collect();
    assert_eq!(next, new.into_iter().collect());
    let waves: Vec<i32> = sqlx::query_scalar("SELECT count(*)::int FROM campaign_recipients WHERE campaign_id = $1 GROUP BY wave ORDER BY wave")
        .bind(id).fetch_all(&pool).await.unwrap();
    assert_eq!(waves, vec![100, 200, 300]);
}

#[sqlx::test]
async fn checks_block_a_send(pool: PgPool) {
    let t = common::app(pool.clone()).await;
    ready(&t).await;
    readers(&pool, t.account.id, 3).await;
    api(
        &t,
        "PATCH",
        "/newsletter",
        Some(json!({"postal_address": ""})),
    )
    .await;
    let id = new_campaign(&t, json!({"subject": "Hi"})).await;
    let (s, b) = api(
        &t,
        "POST",
        &format!("/campaigns/{id}/send"),
        Some(json!({})),
    )
    .await;
    assert_eq!(
        (s, b["error"]["code"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("postal_address"))
    );
    assert!(
        b["error"]["message"]
            .as_str()
            .unwrap()
            .contains("postal address")
    );
    assert_eq!(status(&pool, id).await, "draft");
    let (s, _) = api(
        &t,
        "POST",
        &format!("/campaigns/{id}/send"),
        Some(json!({"send_at": Utc::now() + Duration::days(31)})),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn a_scheduled_issue_starts_at_its_time(pool: PgPool) {
    let t = common::app(pool.clone()).await;
    ready(&t).await;
    readers(&pool, t.account.id, 3).await;
    let id = new_campaign(&t, json!({"subject": "Later"})).await;
    let at = Utc::now() + Duration::hours(1);
    let (s, c) = api(
        &t,
        "POST",
        &format!("/campaigns/{id}/send"),
        Some(json!({"send_at": at})),
    )
    .await;
    assert_eq!(
        (s, c["status"].as_str()),
        (StatusCode::OK, Some("scheduled"))
    );
    send::tick(&t.state, Utc::now()).await.unwrap();
    assert_eq!(status(&pool, id).await, "scheduled");
    send::tick(&t.state, at + Duration::seconds(1))
        .await
        .unwrap();
    assert_eq!(status(&pool, id).await, "sending");
    send::tick(&t.state, at + Duration::seconds(2))
        .await
        .unwrap();
    assert_eq!(queued(&pool, id).await, 3);
    let (s, b) = api(
        &t,
        "POST",
        &format!("/campaigns/{id}/send"),
        Some(json!({})),
    )
    .await;
    assert_eq!(
        (s, b["error"]["code"].as_str()),
        (StatusCode::CONFLICT, Some("campaign_not_sendable"))
    );
}

#[sqlx::test]
async fn a_scheduled_issue_that_fails_its_checks_at_start_is_paused(pool: PgPool) {
    let t = common::app(pool.clone()).await;
    ready(&t).await;
    readers(&pool, t.account.id, 3).await;
    let id = new_campaign(&t, json!({"subject": "Later"})).await;
    let at = Utc::now() + Duration::hours(1);
    api(
        &t,
        "POST",
        &format!("/campaigns/{id}/send"),
        Some(json!({"send_at": at})),
    )
    .await;
    api(
        &t,
        "PATCH",
        "/newsletter",
        Some(json!({"postal_address": ""})),
    )
    .await;
    send::tick(&t.state, at + Duration::seconds(1))
        .await
        .unwrap();
    let (st, reason, started): (String, Option<String>, Option<chrono::DateTime<Utc>>) =
        sqlx::query_as(
            "SELECT status::text, paused_reason, started_at FROM campaigns WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(st, "paused");
    assert!(reason.unwrap().contains("postal address"));
    assert!(started.is_none());
}

#[sqlx::test]
async fn one_issue_sends_at_a_time(pool: PgPool) {
    let t = common::app(pool.clone()).await;
    ready(&t).await;
    readers(&pool, t.account.id, 3).await;
    let a = new_campaign(&t, json!({"subject": "First"})).await;
    let b = new_campaign(&t, json!({"subject": "Second"})).await;
    send_now(&t, a).await;
    let c = send_now(&t, b).await;
    assert_eq!(c["status"], "scheduled", "waits for the first");
    let now = Utc::now();
    send::tick(&t.state, now).await.unwrap();
    assert_eq!(status(&pool, b).await, "scheduled");
    mark_sent(&pool, a).await;
    send::tick(&t.state, now).await.unwrap();
    assert_eq!(status(&pool, a).await, "sent");
    send::tick(&t.state, now).await.unwrap();
    assert_eq!(status(&pool, b).await, "sending");
}

#[sqlx::test]
async fn the_archive_slug_gets_a_suffix_on_a_clash(pool: PgPool) {
    let t = common::app(pool.clone()).await;
    ready(&t).await;
    readers(&pool, t.account.id, 1).await;
    let mut slugs = vec![];
    for _ in 0..3 {
        let id = new_campaign(&t, json!({"subject": "Hello World!"})).await;
        slugs.push(send_now(&t, id).await["archive_slug"].clone());
        sqlx::query("UPDATE campaigns SET status = 'sent' WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }
    assert_eq!(
        slugs,
        vec![
            json!("hello-world"),
            json!("hello-world-2"),
            json!("hello-world-3")
        ]
    );
}

#[sqlx::test]
async fn tag_filters_pick_the_readers(pool: PgPool) {
    let t = common::app(pool.clone()).await;
    ready(&t).await;
    for (email, tags) in [
        ("in@x.com", vec!["news"]),
        ("both@x.com", vec!["news", "skip"]),
        ("none@x.com", vec![]),
    ] {
        api(
            &t,
            "POST",
            "/contacts",
            Some(json!({"email": email, "tags": tags})),
        )
        .await;
    }
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
    let id = new_campaign(
        &t,
        json!({"subject": "Tagged", "include_tags": [tag("news")], "exclude_tags": [tag("skip")]}),
    )
    .await;
    send_now(&t, id).await;
    send::tick(&t.state, Utc::now()).await.unwrap();
    let to: Vec<String> =
        sqlx::query_scalar("SELECT to_addresses[1] FROM emails WHERE campaign_id = $1")
            .bind(id)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(to, vec!["in@x.com"]);
    let (from, priority, html): (String, String, Option<String>) = sqlx::query_as(
        "SELECT from_address, priority::text, html FROM emails WHERE campaign_id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(from.contains("<news@news.shop.com>"));
    assert_eq!((priority.as_str(), html), ("bulk", None));
}

/// Runs the worker until nothing is left to claim.
async fn drain(t: &common::TestApp) {
    let limiter = SendLimiter::new(10_000, 1);
    while worker::tick(&t.state, &limiter).await.unwrap() > 0 {}
}

fn greeting_doc() -> Value {
    json!({"type": "doc", "content": [{"type": "paragraph", "content": [
        {"type": "text", "text": "Hi "}, {"type": "mergeTag", "attrs": {"key": "first_name", "fallback": "there"}},
        {"type": "text", "text": ", read "},
        {"type": "text", "text": "this", "marks": [{"type": "link", "attrs": {"href": "https://shop.com/post"}}]}
    ]}]})
}

fn raw_of(sent: &[mailrust::sender::OutgoingEmail], to: &str) -> String {
    let e = sent
        .iter()
        .find(|e| e.recipients == vec![to.to_string()])
        .unwrap_or_else(|| panic!("nothing sent to {to}"));
    String::from_utf8_lossy(&e.raw).to_string()
}

#[sqlx::test]
async fn each_reader_gets_their_own_copy_with_one_click_headers(pool: PgPool) {
    let rec = common::RecordingSender::ok();
    let t = common::app_with_sender(pool.clone(), rec.clone()).await;
    ready(&t).await;
    sqlx::query(
        "UPDATE domains SET click_tracking = true, open_tracking = true WHERE account_id = $1",
    )
    .bind(t.account.id)
    .execute(&pool)
    .await
    .unwrap();
    api(
        &t,
        "POST",
        "/contacts",
        Some(json!({"email": "ann@x.com", "first_name": "Ann"})),
    )
    .await;
    api(&t, "POST", "/contacts", Some(json!({"email": "bob@x.com"}))).await;
    let id = new_campaign(&t, json!({"subject": "Hello", "blocks": greeting_doc()})).await;
    send_now(&t, id).await;
    send::tick(&t.state, Utc::now()).await.unwrap();
    drain(&t).await;

    let sent = rec.sent();
    assert_eq!(sent.len(), 2);
    let ann = raw_of(&sent, "ann@x.com");
    // lettre sends the text part quoted-printable; a short ASCII line stays verbatim. The "(url)" form is only in the text part.
    assert!(
        ann.contains("Hi Ann, read this (https://shop.com/post)"),
        "merge value in the text part"
    );
    assert!(raw_of(&sent, "bob@x.com").contains("Hi there, read this (https://shop.com/post)"));
    assert!(ann.contains("List-Unsubscribe-Post: List-Unsubscribe=One-Click\r\n"));
    let token = ann
        .split("List-Unsubscribe: <http://localhost:3000/u/")
        .nth(1)
        .unwrap()
        .split('>')
        .next()
        .unwrap();
    let ann_id: Uuid = sqlx::query_scalar("SELECT id FROM contacts WHERE email = 'ann@x.com'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        tokens::verify_unsubscribe_any(&t.state.config.tracking_secret, token),
        Some((ann_id, Some(id)))
    );

    // The body link is click-tracked; the unsubscribe link is not.
    let links: Vec<Value> = sqlx::query_scalar("SELECT links FROM emails WHERE campaign_id = $1")
        .bind(id)
        .fetch_all(&pool)
        .await
        .unwrap();
    for l in links {
        assert_eq!(l, json!(["https://shop.com/post"]));
    }
    let statuses: Vec<String> =
        sqlx::query_scalar("SELECT status::text FROM emails WHERE campaign_id = $1")
            .bind(id)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(
        statuses.iter().all(|s| s == "sent" || s == "delivered"),
        "{statuses:?}"
    );
    send::tick(&t.state, Utc::now()).await.unwrap();
    assert_eq!(status(&pool, id).await, "sent");
}

#[sqlx::test]
async fn readers_who_left_after_the_snapshot_are_skipped_and_the_issue_finishes(pool: PgPool) {
    let rec = common::RecordingSender::ok();
    let t = common::app_with_sender(pool.clone(), rec.clone()).await;
    ready(&t).await;
    readers(&pool, t.account.id, 4).await;
    let id = new_campaign(&t, json!({"subject": "Hello"})).await;
    send_now(&t, id).await; // snapshot taken
    sqlx::query("UPDATE contacts SET status = 'unsubscribed', unsubscribed_at = now() WHERE email = 'r1@readers.test'").execute(&pool).await.unwrap();
    send::tick(&t.state, Utc::now()).await.unwrap(); // queued for all 4
    sqlx::query("DELETE FROM contacts WHERE email = 'r2@readers.test'")
        .execute(&pool)
        .await
        .unwrap();
    api(
        &t,
        "POST",
        "/suppressions",
        Some(json!({"address": "r3@readers.test"})),
    )
    .await;
    drain(&t).await;

    let to: Vec<Vec<String>> = rec.sent().into_iter().map(|e| e.recipients).collect();
    assert_eq!(to, vec![vec!["r4@readers.test".to_string()]]);
    let rows: Vec<(Option<Uuid>, String)> = sqlx::query_as("SELECT contact_id, status::text FROM emails WHERE campaign_id = $1 ORDER BY to_addresses[1]")
        .bind(id).fetch_all(&pool).await.unwrap();
    assert_eq!(rows.len(), 4, "the deleted reader's email row stays");
    assert_eq!(rows[1].0, None, "r2: contact_id set to null");
    assert_eq!(rows.iter().filter(|r| r.1 == "suppressed").count(), 3);
    let reason: Value = sqlx::query_scalar(
        "SELECT v.data FROM email_events v JOIN emails e ON e.id = v.email_id WHERE e.campaign_id = $1 AND e.to_addresses[1] = 'r1@readers.test' AND v.type = 'suppressed'",
    )
    .bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!(reason["reason"], "unsubscribed");
    send::tick(&t.state, Utc::now()).await.unwrap();
    assert_eq!(status(&pool, id).await, "sent");
}

#[sqlx::test]
async fn campaign_rows_stay_out_of_the_email_log(pool: PgPool) {
    let t = common::app(pool.clone()).await;
    ready(&t).await;
    readers(&pool, t.account.id, 2).await;
    let id = new_campaign(&t, json!({"subject": "Hello"})).await;
    send_now(&t, id).await;
    send::tick(&t.state, Utc::now()).await.unwrap();
    let (_, l) = api(&t, "GET", "/emails", None).await;
    assert_eq!(l["data"], json!([]));
    let one: Uuid = sqlx::query_scalar("SELECT id FROM emails WHERE campaign_id = $1 LIMIT 1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let (s, _) = api(&t, "GET", &format!("/emails/{one}"), None).await;
    assert_eq!(s, StatusCode::OK, "still readable by id");
}
