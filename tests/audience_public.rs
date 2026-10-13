mod common;

use axum::http::StatusCode;
use serde_json::{Value, json};

async fn settings(t: &common::TestApp, key: &str, body: Option<Value>) -> (StatusCode, Value) {
    let m = if body.is_some() { "PATCH" } else { "GET" };
    common::call(&t.router, common::req(m, "/newsletter", Some(key), body)).await
}

#[sqlx::test]
async fn the_form_opens_only_when_everything_is_set(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let (s, b) = settings(&t, &t.key, None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(b["signup_open"], false);
    assert!(b["closed_reason"].is_string());

    common::verified_domain(&t, "news.shop.com").await;
    let (s, b) = settings(
        &t,
        &t.key,
        Some(json!({
            "public_slug": "  Weekly ", "newsletter_name": "Shop Weekly",
            "newsletter_from": "Hello@News.Shop.com", "signup_enabled": true
        })),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["public_slug"], "weekly");
    assert_eq!(b["newsletter_from"], "hello@news.shop.com");
    assert_eq!(b["signup_open"], true);
    assert!(b["closed_reason"].is_null());

    mailrust::accounts::pause(&pool, t.account.id, "test")
        .await
        .unwrap();
    let (_, b) = settings(&t, &t.key, None).await;
    assert_eq!(b["signup_open"], false);
    assert!(b["closed_reason"].as_str().unwrap().contains("paused"));
}

#[sqlx::test]
async fn the_from_address_must_be_on_a_verified_domain(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let (s, b) = settings(
        &t,
        &t.key,
        Some(json!({"newsletter_from": "hi@elsewhere.com"})),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{b}");
    assert_eq!(b["error"]["code"], "from_not_verified");
    let (s, _) = settings(&t, &t.key, Some(json!({"newsletter_from": "nope"}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn slugs_are_checked_and_unique(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    for bad in ["ab", "a_b", &"x".repeat(41)] {
        let (s, _) = settings(&t, &t.key, Some(json!({"public_slug": bad}))).await;
        assert_eq!(s, StatusCode::BAD_REQUEST, "{bad}");
    }
    let (s, _) = settings(&t, &t.key, Some(json!({"signup_enabled": true}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "no slug yet");
    let (s, _) = settings(&t, &t.key, Some(json!({"public_slug": "weekly"}))).await;
    assert_eq!(s, StatusCode::OK);

    let other = mailrust::accounts::create(&pool, "Other").await.unwrap();
    let key = mailrust::keys::create(&pool, other.id, "k", mailrust::keys::KeyAccess::Full)
        .await
        .unwrap()
        .token;
    for taken in ["weekly", " WEEKLY "] {
        let (s, b) = settings(&t, &key, Some(json!({"public_slug": taken}))).await;
        assert_eq!(s, StatusCode::CONFLICT, "{b}");
        assert_eq!(b["error"]["code"], "slug_taken");
    }
    let (s, b) = settings(&t, &t.key, Some(json!({"public_slug": ""}))).await;
    assert_eq!(s, StatusCode::OK);
    assert!(b["public_slug"].is_null());
    let (s, _) = settings(&t, &key, Some(json!({"public_slug": "weekly"}))).await;
    assert_eq!(s, StatusCode::OK, "a freed slug can be taken");
}

use mailrust::tokens;

async fn contact(t: &common::TestApp, email: &str) -> uuid::Uuid {
    let (s, b) = common::call(
        &t.router,
        common::req(
            "POST",
            "/contacts",
            Some(&t.key),
            Some(json!({"email": email})),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{b}");
    b["id"].as_str().unwrap().parse().unwrap()
}

async fn status_of(pool: &sqlx::PgPool, id: uuid::Uuid) -> String {
    sqlx::query_scalar("SELECT status::text FROM contacts WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test]
async fn public_routes_answer_only_the_web_app(pool: sqlx::PgPool) {
    let mut t = common::app(pool).await;
    // WEB_SECRET unset: nobody gets in.
    let (s, _) = common::call(
        &t.router,
        common::web_req("GET", "/public/newsletter/weekly", "1.2.3.4", None),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    common::with_web_secret(&mut t);
    settings(&t, &t.key, Some(json!({"public_slug": "weekly"}))).await;
    let (s, _) = common::call(
        &t.router,
        common::req("GET", "/public/newsletter/weekly", None, None),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND, "no header");
    let mut wrong = common::web_req("GET", "/public/newsletter/weekly", "1.2.3.4", None);
    wrong
        .headers_mut()
        .insert("x-mailrust-web", "x".repeat(37).parse().unwrap());
    let (s, _) = common::call(&t.router, wrong).await;
    assert_eq!(s, StatusCode::NOT_FOUND, "wrong secret");
    let (s, b) = common::call(
        &t.router,
        common::web_req("GET", "/public/newsletter/Weekly", "1.2.3.4", None),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b, json!({"name": "Test", "open": false}));
    let (s, _) = common::call(
        &t.router,
        common::web_req("GET", "/public/newsletter/nobody", "1.2.3.4", None),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn public_routes_are_rate_limited_per_ip(pool: sqlx::PgPool) {
    let mut t = common::app(pool).await;
    common::with_web_secret(&mut t);
    let mut limited = false;
    for _ in 0..40 {
        let (s, _) = common::call(
            &t.router,
            common::web_req("GET", "/public/newsletter/x", "9.9.9.9", None),
        )
        .await;
        limited |= s == StatusCode::TOO_MANY_REQUESTS;
    }
    assert!(limited);
    let (s, _) = common::call(
        &t.router,
        common::web_req("GET", "/public/newsletter/x", "8.8.8.8", None),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND, "another IP is not limited");
}

#[sqlx::test]
async fn the_link_unsubscribes_and_can_resubscribe(pool: sqlx::PgPool) {
    let mut t = common::app(pool.clone()).await;
    common::with_web_secret(&mut t);
    settings(&t, &t.key, Some(json!({"newsletter_name": "Shop Weekly"}))).await;
    common::call(
        &t.router,
        common::req(
            "POST",
            "/suppressions",
            Some(&t.key),
            Some(json!({"address": "other@x.com"})),
        ),
    )
    .await;
    let id = contact(&t, "ana@x.com").await;
    let token = tokens::sign_unsubscribe(&t.state.config.tracking_secret, id);
    let web = |m: &str, uri: &str, body: Option<Value>| common::web_req(m, uri, "1.2.3.4", body);

    let (s, b) = common::call(
        &t.router,
        web("GET", &format!("/public/unsubscribe/{token}"), None),
    )
    .await;
    assert_eq!(
        (s, b),
        (StatusCode::OK, json!({"newsletter_name": "Shop Weekly"}))
    );
    assert_eq!(
        status_of(&pool, id).await,
        "subscribed",
        "looking does not unsubscribe"
    );

    for _ in 0..2 {
        let (s, b) = common::call(
            &t.router,
            web("POST", "/public/unsubscribe", Some(json!({"token": token}))),
        )
        .await;
        assert_eq!(
            (s, b),
            (StatusCode::OK, json!({"newsletter_name": "Shop Weekly"}))
        );
    }
    assert_eq!(status_of(&pool, id).await, "unsubscribed");
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM suppressions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 1, "the suppression list is unchanged");

    let (s, _) = common::call(
        &t.router,
        web("POST", "/public/resubscribe", Some(json!({"token": token}))),
    )
    .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert_eq!(status_of(&pool, id).await, "subscribed");
    let (s, b) = common::call(
        &t.router,
        web("POST", "/public/resubscribe", Some(json!({"token": token}))),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_eq!(b["error"]["code"], "resubscribe_not_allowed");
}

#[sqlx::test]
async fn the_link_cannot_undo_an_unsubscribe_made_elsewhere(pool: sqlx::PgPool) {
    let mut t = common::app(pool.clone()).await;
    common::with_web_secret(&mut t);
    let id = contact(&t, "ana@x.com").await;
    common::call(
        &t.router,
        common::req(
            "PATCH",
            &format!("/contacts/{id}"),
            Some(&t.key),
            Some(json!({"status": "unsubscribed"})),
        ),
    )
    .await;
    let token = tokens::sign_unsubscribe(&t.state.config.tracking_secret, id);
    let (s, _) = common::call(
        &t.router,
        common::web_req(
            "POST",
            "/public/resubscribe",
            "1.2.3.4",
            Some(json!({"token": token})),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_eq!(status_of(&pool, id).await, "unsubscribed");
}

#[sqlx::test]
async fn bad_or_stale_tokens_change_nothing(pool: sqlx::PgPool) {
    let mut t = common::app(pool.clone()).await;
    common::with_web_secret(&mut t);
    let id = contact(&t, "ana@x.com").await;
    let secret = t.state.config.tracking_secret.clone();
    let good = tokens::sign_unsubscribe(&secret, id);
    let mut flipped = good.clone().into_bytes();
    flipped[30] = if flipped[30] == b'A' { b'B' } else { b'A' };
    let bad = [
        String::from_utf8(flipped).unwrap(),
        good[..40].to_string(),
        tokens::sign(&secret, id, tokens::TokenKind::Open),
        tokens::sign_unsubscribe(b"another-secret-another-secret-xx", id),
        "garbage".into(),
    ];
    for (i, token) in bad.iter().enumerate() {
        let ip = format!("10.0.0.{i}"); // one IP per token keeps the per-IP rate limit out of this test
        let (s, b) = common::call(
            &t.router,
            common::web_req(
                "POST",
                "/public/unsubscribe",
                &ip,
                Some(json!({"token": token})),
            ),
        )
        .await;
        assert_eq!(
            (s, b),
            (StatusCode::OK, json!({"newsletter_name": null})),
            "{token}"
        );
        let (s, _) = common::call(
            &t.router,
            common::web_req(
                "POST",
                "/public/resubscribe",
                &ip,
                Some(json!({"token": token})),
            ),
        )
        .await;
        assert_eq!(s, StatusCode::CONFLICT);
    }
    assert_eq!(status_of(&pool, id).await, "subscribed");

    common::call(
        &t.router,
        common::req("DELETE", &format!("/contacts/{id}"), Some(&t.key), None),
    )
    .await;
    let (s, b) = common::call(
        &t.router,
        common::web_req(
            "POST",
            "/public/unsubscribe",
            "1.2.3.4",
            Some(json!({"token": good})),
        ),
    )
    .await;
    assert_eq!(
        (s, b),
        (StatusCode::OK, json!({"newsletter_name": null})),
        "a deleted contact still gets the plain answer"
    );
}

/// A fresh account with an open form at /join/<slug> ("Shop Weekly" from hello@<slug>.shop.com).
async fn open_form_at(pool: &sqlx::PgPool, slug: &str) -> common::TestApp {
    let mut t = common::app(pool.clone()).await;
    common::with_web_secret(&mut t);
    common::verified_domain(&t, &format!("{slug}.shop.com")).await;
    let (s, b) = settings(
        &t,
        &t.key,
        Some(json!({
            "public_slug": slug, "newsletter_name": "Shop Weekly",
            "newsletter_from": format!("hello@{slug}.shop.com"), "signup_enabled": true
        })),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    t
}

async fn open_form(pool: &sqlx::PgPool) -> common::TestApp {
    open_form_at(pool, "weekly").await
}

async fn join_at(t: &common::TestApp, slug: &str, email: &str, ip: &str) -> (StatusCode, Value) {
    common::call(
        &t.router,
        common::web_req(
            "POST",
            "/public/join",
            ip,
            Some(json!({"slug": slug, "email": email, "first_name": "Ana"})),
        ),
    )
    .await
}

async fn join(t: &common::TestApp, email: &str, ip: &str) -> (StatusCode, Value) {
    join_at(t, "weekly", email, ip).await
}

async fn mails_to(pool: &sqlx::PgPool, to: &str) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT text_body FROM emails WHERE $1 = ANY(to_addresses) ORDER BY created_at",
    )
    .bind(to)
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn wait_for_mails(pool: &sqlx::PgPool, to: &str, n: usize) -> Vec<String> {
    common::eventually(|| async { mails_to(pool, to).await.len() >= n }).await;
    mails_to(pool, to).await
}

fn confirm_token(text: &str) -> String {
    let start = text.find("/confirm/").expect("confirm link") + "/confirm/".len();
    text[start..].split_whitespace().next().unwrap().to_string()
}

async fn contact_status(pool: &sqlx::PgPool, email: &str) -> Option<String> {
    sqlx::query_scalar("SELECT status::text FROM contacts WHERE email = $1")
        .bind(email)
        .fetch_optional(pool)
        .await
        .unwrap()
}

#[sqlx::test]
async fn join_then_confirm_subscribes(pool: sqlx::PgPool) {
    let t = open_form(&pool).await;
    let (s, b) = join(&t, " Reader@X.com ", "1.1.1.1").await;
    assert_eq!((s, b), (StatusCode::ACCEPTED, Value::Null));
    let mails = wait_for_mails(&pool, "reader@x.com", 1).await;
    assert!(
        mails[0].contains("http://localhost:3000/confirm/mr_conf_"),
        "{}",
        mails[0]
    );
    assert_eq!(
        contact_status(&pool, "reader@x.com").await.as_deref(),
        Some("pending")
    );
    let (from, account_id): (String, uuid::Uuid) = sqlx::query_as(
        "SELECT from_address, account_id FROM emails WHERE 'reader@x.com' = ANY(to_addresses)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        from.contains("Shop Weekly") && from.contains("hello@weekly.shop.com"),
        "{from}"
    );
    assert_eq!(
        account_id, t.account.id,
        "sent through the newsletter's own account"
    );

    let token = confirm_token(&mails[0]);
    let confirm = |tok: String| {
        common::web_req(
            "POST",
            "/public/confirm",
            "1.1.1.1",
            Some(json!({"token": tok})),
        )
    };
    let (s, b) = common::call(&t.router, confirm(token.clone())).await;
    assert_eq!(
        (s, b),
        (StatusCode::OK, json!({"newsletter_name": "Shop Weekly"}))
    );
    assert_eq!(
        contact_status(&pool, "reader@x.com").await.as_deref(),
        Some("subscribed")
    );
    let (s, b) = common::call(&t.router, confirm(token)).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED, "single use");
    assert_eq!(b["error"]["code"], "invalid_token");
}

#[sqlx::test]
async fn every_join_answers_the_same(pool: sqlx::PgPool) {
    let t = open_form(&pool).await;
    common::call(
        &t.router,
        common::req(
            "POST",
            "/contacts",
            Some(&t.key),
            Some(json!({"email": "sub@x.com"})),
        ),
    )
    .await;
    let answers = [
        join(&t, "new@x.com", "1.1.1.1").await,
        join(&t, "SUB@x.com", "1.1.1.2").await,
    ];
    assert!(
        answers
            .iter()
            .all(|a| *a == (StatusCode::ACCEPTED, Value::Null))
    );
    wait_for_mails(&pool, "new@x.com", 1).await;
    assert!(
        mails_to(&pool, "sub@x.com").await.is_empty(),
        "already subscribed: nothing sent"
    );

    // Pending again within 10 minutes: no second email. After 10 minutes: a new one.
    assert_eq!(
        join(&t, "new@x.com", "1.1.1.3").await.0,
        StatusCode::ACCEPTED
    );
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_eq!(mails_to(&pool, "new@x.com").await.len(), 1);
    sqlx::query("UPDATE contact_confirmations SET created_at = now() - interval '11 minutes'")
        .execute(&pool)
        .await
        .unwrap();
    join(&t, "new@x.com", "1.1.1.4").await;
    wait_for_mails(&pool, "new@x.com", 2).await;
}

#[sqlx::test]
async fn an_unsubscribed_reader_can_come_back_through_the_form(pool: sqlx::PgPool) {
    let t = open_form(&pool).await;
    let (_, c) = common::call(
        &t.router,
        common::req(
            "POST",
            "/contacts",
            Some(&t.key),
            Some(json!({"email": "back@x.com"})),
        ),
    )
    .await;
    let uri = format!("/contacts/{}", c["id"].as_str().unwrap());
    common::call(
        &t.router,
        common::req(
            "PATCH",
            &uri,
            Some(&t.key),
            Some(json!({"status": "unsubscribed"})),
        ),
    )
    .await;
    join(&t, "back@x.com", "1.1.1.1").await;
    let mails = wait_for_mails(&pool, "back@x.com", 1).await;
    assert_eq!(
        contact_status(&pool, "back@x.com").await.as_deref(),
        Some("pending")
    );
    let (s, _) = common::call(
        &t.router,
        common::web_req(
            "POST",
            "/public/confirm",
            "1.1.1.1",
            Some(json!({"token": confirm_token(&mails[0])})),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(
        contact_status(&pool, "back@x.com").await.as_deref(),
        Some("subscribed")
    );
}

#[sqlx::test]
async fn the_sixth_join_from_one_ip_in_an_hour_sends_nothing(pool: sqlx::PgPool) {
    let t = open_form(&pool).await;
    for i in 0..6 {
        assert_eq!(
            join(&t, &format!("r{i}@x.com"), "7.7.7.7").await.0,
            StatusCode::ACCEPTED
        );
    }
    common::eventually(|| async {
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM emails")
            .fetch_one(&pool)
            .await
            .unwrap()
            == 5
    })
    .await;
    assert!(mails_to(&pool, "r5@x.com").await.is_empty());
    assert_eq!(contact_status(&pool, "r5@x.com").await, None);
}

#[sqlx::test]
async fn the_honeypot_saves_and_sends_nothing(pool: sqlx::PgPool) {
    let t = open_form(&pool).await;
    let (s, b) = common::call(
        &t.router,
        common::web_req(
            "POST",
            "/public/join",
            "1.1.1.1",
            Some(json!({"slug": "weekly", "email": "bot@x.com", "website": "http://spam"})),
        ),
    )
    .await;
    assert_eq!((s, b), (StatusCode::ACCEPTED, Value::Null));
    for table in ["contacts", "join_requests", "emails"] {
        let n: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n, 0, "{table}");
    }
}

#[sqlx::test]
async fn a_closed_or_unknown_form_refuses(pool: sqlx::PgPool) {
    let t = open_form(&pool).await;
    let (s, _) = common::call(
        &t.router,
        common::web_req(
            "POST",
            "/public/join",
            "1.1.1.1",
            Some(json!({"slug": "nobody", "email": "a@x.com"})),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (s, _) = join(&t, "not an email", "1.1.1.1").await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // One form per reason (§6.1): turned off, from-domain gone, account paused.
    let closers = [
        (
            "off",
            "UPDATE accounts SET signup_enabled = false WHERE id = $1",
        ),
        ("nodomain", "DELETE FROM domains WHERE account_id = $1"),
        (
            "paused",
            "UPDATE accounts SET status = 'paused', paused_reason = 'test' WHERE id = $1",
        ),
    ];
    for (slug, sql) in closers {
        let t = open_form_at(&pool, slug).await;
        sqlx::query(sql)
            .bind(t.account.id)
            .execute(&pool)
            .await
            .unwrap();
        let (s, b) = join_at(&t, slug, "a@x.com", "1.1.1.1").await;
        assert_eq!(s, StatusCode::CONFLICT, "{slug}: {b}");
        assert_eq!(b["error"]["code"], "signup_closed");
    }
}

#[sqlx::test]
async fn an_expired_confirm_link_is_refused(pool: sqlx::PgPool) {
    let t = open_form(&pool).await;
    join(&t, "late@x.com", "1.1.1.1").await;
    let mails = wait_for_mails(&pool, "late@x.com", 1).await;
    sqlx::query("UPDATE contact_confirmations SET expires_at = now() - interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    let (s, _) = common::call(
        &t.router,
        common::web_req(
            "POST",
            "/public/confirm",
            "1.1.1.1",
            Some(json!({"token": confirm_token(&mails[0])})),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    assert_eq!(
        contact_status(&pool, "late@x.com").await.as_deref(),
        Some("pending")
    );
}

#[sqlx::test]
async fn cleanup_forgets_old_pending_but_remembers_who_left(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let a = t.account.id;
    sqlx::query(
        "INSERT INTO contacts (id, account_id, email, status, source, unsubscribed_at, updated_at) VALUES
           (gen_random_uuid(), $1, 'old@x.com',   'pending',    'form', NULL,  now() - interval '31 days'),
           (gen_random_uuid(), $1, 'back@x.com',  'pending',    'form', now() - interval '40 days', now() - interval '31 days'),
           (gen_random_uuid(), $1, 'fresh@x.com', 'pending',    'form', NULL,  now() - interval '29 days'),
           (gen_random_uuid(), $1, 'sub@x.com',   'subscribed', 'api',  NULL,  now() - interval '90 days')",
    )
    .bind(a)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO contact_confirmations (token_hash, contact_id, expires_at)
         SELECT '\\x01', id, now() - interval '2 days' FROM contacts WHERE email = 'fresh@x.com'",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO join_requests (ip, email, created_at) VALUES ('1.1.1.1', 'a@x.com', now() - interval '2 hours'), ('1.1.1.1', 'b@x.com', now())")
        .execute(&pool)
        .await
        .unwrap();

    mailrust::background::cleanup_audience(&pool).await.unwrap();

    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT email, status::text FROM contacts ORDER BY email")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        rows,
        [
            ("back@x.com".into(), "unsubscribed".into()),
            ("fresh@x.com".into(), "pending".into()),
            ("sub@x.com".into(), "subscribed".into())
        ]
    );
    let conf: i64 = sqlx::query_scalar("SELECT count(*) FROM contact_confirmations")
        .fetch_one(&pool)
        .await
        .unwrap();
    let reqs: i64 = sqlx::query_scalar("SELECT count(*) FROM join_requests")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!((conf, reqs), (0, 1));
}

#[sqlx::test]
async fn a_newsletter_name_cannot_hold_line_breaks(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    for name in ["Shop\nWeekly", "Shop\r\nWeekly", "Shop\u{0}"] {
        let (s, b) = settings(&t, &t.key, Some(json!({"newsletter_name": name}))).await;
        assert_eq!(s, StatusCode::BAD_REQUEST, "{name:?}: {b}");
    }
}
