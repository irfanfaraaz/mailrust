mod common;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use mailrust::campaigns::send;
use mailrust::email_events::{self, EventKind, NewEvent};
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

async fn post(t: &common::TestApp, id: Uuid, action: &str) -> (StatusCode, Value) {
    api(t, "POST", &format!("/campaigns/{id}/{action}"), None).await
}

/// Marks `n` queued emails of the campaign sent (a worker part-way through a wave).
async fn send_some(pool: &PgPool, id: Uuid, n: i64) {
    sqlx::query("UPDATE emails SET status = 'sent', sent_at = now() WHERE id IN (SELECT id FROM emails WHERE campaign_id = $1 AND status = 'queued' LIMIT $2)")
        .bind(id).bind(n).execute(pool).await.unwrap();
}

async fn once_each(pool: &PgPool, id: Uuid) -> (i64, i64) {
    sqlx::query_as("SELECT count(*), count(DISTINCT contact_id) FROM emails WHERE campaign_id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test]
async fn pause_takes_back_queued_emails_and_resume_finishes_the_wave_first(pool: PgPool) {
    let t = common::app(pool.clone()).await;
    ready(&t).await;
    readers(&pool, t.account.id, 600).await; // waves 100, 200, 300
    let id = new_campaign(&t, json!({"subject": "Hello"})).await;
    send_now(&t, id).await;
    send::tick(&t.state, Utc::now()).await.unwrap();
    send_some(&pool, id, 40).await;

    let (s, c) = post(&t, id, "pause").await;
    assert_eq!((s, c["status"].as_str()), (StatusCode::OK, Some("paused")));
    assert_eq!(queued(&pool, id).await, 0);
    assert_eq!(once_each(&pool, id).await, (40, 40));
    let linked: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM campaign_recipients WHERE campaign_id = $1 AND email_id IS NOT NULL",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(linked, 40);
    let (s, b) = post(&t, id, "pause").await;
    assert_eq!(
        (s, b["error"]["code"].as_str()),
        (StatusCode::CONFLICT, Some("campaign_not_sending"))
    );

    let (s, c) = post(&t, id, "resume").await;
    assert_eq!((s, c["status"].as_str()), (StatusCode::OK, Some("sending")));
    let mut now = Utc::now();
    send::tick(&t.state, now).await.unwrap();
    assert_eq!(
        queued(&pool, id).await,
        60,
        "the rest of wave 1, not wave 2"
    );
    for expected in [200, 300] {
        mark_sent(&pool, id).await;
        send::tick(&t.state, now).await.unwrap();
        now += Duration::minutes(10) + Duration::seconds(1);
        send::tick(&t.state, now).await.unwrap();
        assert_eq!(queued(&pool, id).await, expected);
    }
    mark_sent(&pool, id).await;
    send::tick(&t.state, now).await.unwrap();
    assert_eq!(status(&pool, id).await, "sent");
    assert_eq!(once_each(&pool, id).await, (600, 600));
}

#[sqlx::test]
async fn auto_pause_mid_wave_takes_back_queued_emails(pool: PgPool) {
    let rec = common::RecordingSender::ok();
    let t = common::app_with_sender(pool.clone(), rec.clone()).await;
    ready(&t).await;
    readers(&pool, t.account.id, 10).await;
    let id = new_campaign(&t, json!({"subject": "Hello"})).await;
    send_now(&t, id).await;
    send::tick(&t.state, Utc::now()).await.unwrap();
    // A worker claimed three before the pause: they come back to `queued` after it (requeue_paused).
    let claimed: Vec<Uuid> = sqlx::query_scalar("UPDATE emails SET status = 'sending', lease_until = now() + interval '5 minutes', attempts = 1
                                                  WHERE id IN (SELECT id FROM emails WHERE campaign_id = $1 LIMIT 3) RETURNING id")
        .bind(id).fetch_all(&pool).await.unwrap();
    mailrust::accounts::pause(&pool, t.account.id, "bounce rate 5% is over 2%")
        .await
        .unwrap();
    send::tick(&t.state, Utc::now()).await.unwrap();
    let (st, reason): (String, String) =
        sqlx::query_as("SELECT status::text, paused_reason FROM campaigns WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(st, "paused");
    assert!(reason.contains("bounce rate"));
    assert_eq!(
        once_each(&pool, id).await.0,
        3,
        "only the claimed three are left"
    );

    // The claimed three come back to the queue; the worker takes them back instead of sending.
    sqlx::query("UPDATE emails SET status = 'queued', lease_until = NULL WHERE id = ANY($1)")
        .bind(&claimed)
        .execute(&pool)
        .await
        .unwrap();
    mailrust::accounts::resume(&pool, t.account.id)
        .await
        .unwrap();
    drain(&t).await;
    assert!(
        rec.sent().is_empty(),
        "nothing goes out while the issue is paused"
    );
    assert_eq!(once_each(&pool, id).await.0, 0);

    let (s, _) = post(&t, id, "resume").await;
    assert_eq!(s, StatusCode::OK);
    send::tick(&t.state, Utc::now()).await.unwrap();
    drain(&t).await;
    assert_eq!(rec.sent().len(), 10);
    assert_eq!(once_each(&pool, id).await, (10, 10));
}

#[sqlx::test]
async fn a_restart_mid_send_sends_each_reader_once(pool: PgPool) {
    let rec = common::RecordingSender::ok();
    let t = common::app_with_sender(pool.clone(), rec.clone()).await;
    ready(&t).await;
    readers(&pool, t.account.id, 700).await; // waves 100, 200, 400
    let id = new_campaign(&t, json!({"subject": "Hello"})).await;
    send_now(&t, id).await;
    let mut now = Utc::now();
    send::tick(&t.state, now).await.unwrap();
    drain(&t).await;
    send::tick(&t.state, now).await.unwrap();
    now += Duration::minutes(11);
    send::tick(&t.state, now).await.unwrap(); // wave 2 queued, then the engine "restarts"

    let again = mailrust::api::AppState::new(
        pool.clone(),
        common::test_config(common::temp_outbox()),
        rec.clone(),
    );
    let limiter = mailrust::limiter::SendLimiter::new(10_000, 1);
    for _ in 0..3 {
        send::tick(&again, now).await.unwrap();
        while worker::tick(&again, &limiter).await.unwrap() > 0 {}
        send::tick(&again, now).await.unwrap();
        now += Duration::minutes(11);
    }
    assert_eq!(status(&pool, id).await, "sent");
    let mut to: Vec<String> = rec.sent().into_iter().flat_map(|e| e.recipients).collect();
    to.sort();
    let n = to.len();
    to.dedup();
    assert_eq!((n, to.len()), (700, 700), "everyone once");
}

#[sqlx::test]
async fn cancel_stops_for_good(pool: PgPool) {
    let t = common::app(pool.clone()).await;
    ready(&t).await;
    readers(&pool, t.account.id, 5).await;
    let draft = new_campaign(&t, json!({"subject": "Draft"})).await;
    let (s, b) = post(&t, draft, "cancel").await;
    assert_eq!(
        (s, b["error"]["code"].as_str()),
        (StatusCode::CONFLICT, Some("campaign_not_cancellable"))
    );

    let id = new_campaign(&t, json!({"subject": "Hello"})).await;
    send_now(&t, id).await;
    send::tick(&t.state, Utc::now()).await.unwrap();
    let (s, c) = post(&t, id, "cancel").await;
    assert_eq!(
        (s, c["status"].as_str()),
        (StatusCode::OK, Some("cancelled"))
    );
    assert_eq!(queued(&pool, id).await, 0);
    send::tick(&t.state, Utc::now() + Duration::hours(1))
        .await
        .unwrap();
    assert_eq!(once_each(&pool, id).await.0, 0);
    let (s, _) = post(&t, id, "resume").await;
    assert_eq!(s, StatusCode::CONFLICT);
}

#[sqlx::test]
async fn resume_waits_for_the_other_issue_and_reruns_the_checks(pool: PgPool) {
    let t = common::app(pool.clone()).await;
    ready(&t).await;
    readers(&pool, t.account.id, 3).await;
    let a = new_campaign(&t, json!({"subject": "A"})).await;
    send_now(&t, a).await;
    post(&t, a, "pause").await;
    let b = new_campaign(&t, json!({"subject": "B"})).await;
    assert_eq!(
        send_now(&t, b).await["status"],
        "sending",
        "a paused issue does not block another"
    );
    let (s, e) = post(&t, a, "resume").await;
    assert_eq!(
        (s, e["error"]["code"].as_str()),
        (StatusCode::CONFLICT, Some("another_sending"))
    );
    post(&t, b, "cancel").await;
    api(
        &t,
        "PATCH",
        "/newsletter",
        Some(json!({"postal_address": ""})),
    )
    .await;
    let (s, e) = post(&t, a, "resume").await;
    assert_eq!(
        (s, e["error"]["code"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("postal_address"))
    );
}

#[sqlx::test]
async fn resuming_an_issue_that_never_started_starts_it(pool: PgPool) {
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
    assert_eq!(status(&pool, id).await, "paused");
    api(
        &t,
        "PATCH",
        "/newsletter",
        Some(json!({"postal_address": "1 Main St"})),
    )
    .await;
    let (s, c) = post(&t, id, "resume").await;
    assert_eq!(
        (s, c["status"].as_str(), c["archive_slug"].as_str()),
        (StatusCode::OK, Some("sending"), Some("later"))
    );
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM campaign_recipients WHERE campaign_id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(n, 3);
}

async fn event(
    pool: &PgPool,
    account: Uuid,
    email_id: Uuid,
    kind: EventKind,
    data: Value,
    bot: bool,
) {
    let mut conn = pool.acquire().await.unwrap();
    email_events::record(
        &mut conn,
        NewEvent {
            account_id: account,
            email_id,
            kind,
            recipient: None,
            data,
            likely_bot: bot,
        },
    )
    .await
    .unwrap();
}

async fn email_to(pool: &PgPool, campaign: Uuid, to: &str) -> Uuid {
    sqlx::query_scalar("SELECT id FROM emails WHERE campaign_id = $1 AND to_addresses[1] = $2")
        .bind(campaign)
        .bind(to)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test]
async fn the_report_counts_unique_people_per_link_and_provider(pool: PgPool) {
    let mut t = common::app(pool.clone()).await;
    common::with_web_secret(&mut t);
    ready(&t).await;
    for e in ["a@gmail.com", "b@gmail.com", "c@hotmail.com", "d@shop.com"] {
        api(&t, "POST", "/contacts", Some(json!({"email": e}))).await;
    }
    let id = new_campaign(&t, json!({"subject": "Hello"})).await;
    send_now(&t, id).await;
    send::tick(&t.state, Utc::now()).await.unwrap();
    mark_sent(&pool, id).await;
    let acc = t.account.id;
    let (a, b, c, d) = (
        email_to(&pool, id, "a@gmail.com").await,
        email_to(&pool, id, "b@gmail.com").await,
        email_to(&pool, id, "c@hotmail.com").await,
        email_to(&pool, id, "d@shop.com").await,
    );
    for e in [a, b, c] {
        event(&pool, acc, e, EventKind::Delivered, json!({}), false).await;
    }
    event(&pool, acc, a, EventKind::Opened, json!({}), false).await;
    event(&pool, acc, a, EventKind::Opened, json!({}), false).await;
    event(&pool, acc, b, EventKind::Opened, json!({}), true).await; // a bot: left out
    event(
        &pool,
        acc,
        a,
        EventKind::Clicked,
        json!({"link": "https://shop.com/x"}),
        false,
    )
    .await;
    event(
        &pool,
        acc,
        a,
        EventKind::Clicked,
        json!({"link": "https://shop.com/x"}),
        false,
    )
    .await;
    event(
        &pool,
        acc,
        c,
        EventKind::Clicked,
        json!({"link": "https://shop.com/y"}),
        false,
    )
    .await;
    event(
        &pool,
        acc,
        b,
        EventKind::Clicked,
        json!({"link": "https://shop.com/y"}),
        true,
    )
    .await; // a scanner
    event(&pool, acc, d, EventKind::Bounced, json!({}), false).await;
    sqlx::query("UPDATE emails SET status = 'bounced' WHERE id = $1")
        .bind(d)
        .execute(&pool)
        .await
        .unwrap();

    // b unsubscribes from this issue's link, twice; it counts once.
    let b_contact: Uuid = sqlx::query_scalar("SELECT id FROM contacts WHERE email = 'b@gmail.com'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let token = tokens::sign_campaign_unsubscribe(&t.state.config.tracking_secret, b_contact, id);
    for _ in 0..2 {
        let (s, _) = common::call(
            &t.router,
            common::web_req(
                "POST",
                "/public/unsubscribe",
                "10.0.0.9",
                Some(json!({"token": token})),
            ),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
    }
    // An old-form link changes the contact but names no issue.
    let c_contact: Uuid =
        sqlx::query_scalar("SELECT id FROM contacts WHERE email = 'c@hotmail.com'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let old = tokens::sign_unsubscribe(&t.state.config.tracking_secret, c_contact);
    common::call(
        &t.router,
        common::web_req(
            "POST",
            "/public/unsubscribe",
            "10.0.0.9",
            Some(json!({"token": old})),
        ),
    )
    .await;

    let (s, r) = api(&t, "GET", &format!("/campaigns/{id}/report"), None).await;
    assert_eq!(s, StatusCode::OK, "{r}");
    let tot = &r["totals"];
    assert_eq!(tot["recipients"], 4);
    assert_eq!(tot["sent"], 4);
    assert_eq!(tot["delivered"], 3);
    assert_eq!(tot["bounced"], 1);
    assert_eq!(tot["opened"], 1);
    assert_eq!(tot["clicked"], 2);
    assert_eq!(tot["unsubscribed"], 1);
    assert_eq!(r["rates"]["open_rate"], 0.25);
    assert_eq!(r["rates"]["click_rate"], 0.5);
    assert_eq!(
        r["links"],
        json!([
            {"url": "https://shop.com/x", "clicks": 2, "unique_clicks": 1},
            {"url": "https://shop.com/y", "clicks": 1, "unique_clicks": 1}
        ])
    );
    let gmail = r["providers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["provider"] == "Gmail")
        .unwrap()
        .clone();
    assert_eq!(
        (
            gmail["sent"].as_i64(),
            gmail["opened"].as_i64(),
            gmail["clicked"].as_i64()
        ),
        (Some(2), Some(1), Some(1))
    );
    let other = r["providers"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(
        (other["provider"].as_str(), other["bounced"].as_i64()),
        (Some("Other"), Some(1))
    );
    assert_eq!(r["waves"], json!([{"wave": 1, "readers": 4, "done": 4}]));
}

#[sqlx::test]
async fn another_accounts_report_is_not_found(pool: PgPool) {
    let t = common::app(pool.clone()).await;
    let id = new_campaign(&t, json!({"subject": "Mine"})).await;
    let u = common::app(pool).await;
    let (s, _) = api(&u, "GET", &format!("/campaigns/{id}/report"), None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}
