mod common;

use mailrust::{
    contacts::{self, ContactInput, ContactSource, ContactStatus},
    safety::{self, SuppressionReason},
};

fn input(email: &str) -> ContactInput {
    ContactInput {
        email: email.into(),
        ..Default::default()
    }
}

#[sqlx::test]
async fn upsert_adds_once_then_merges(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let mut conn = pool.acquire().await.unwrap();
    let a = input(" Ana@Shop.COM ").validate().unwrap();
    let (id1, new1) = contacts::upsert(&mut conn, t.account.id, &a, ContactSource::Api)
        .await
        .unwrap();

    let mut b = input("ana@shop.com");
    b.first_name = Some("Ana".into());
    b.fields.insert("plan".into(), "pro".into());
    let (id2, new2) = contacts::upsert(
        &mut conn,
        t.account.id,
        &b.validate().unwrap(),
        ContactSource::Import,
    )
    .await
    .unwrap();
    assert!(new1 && !new2);
    assert_eq!(id1, id2);

    let v = contacts::view(&pool, t.account.id, id1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(v.email, "ana@shop.com");
    assert_eq!(v.first_name.as_deref(), Some("Ana"));
    assert_eq!(v.fields.0["plan"], "pro");
    assert_eq!(v.status, ContactStatus::Subscribed);
    assert_eq!(v.source, ContactSource::Api, "the first source stays");
    assert!(v.subscribed_at.is_some());
    assert!(!v.blocked);
}

#[sqlx::test]
async fn upsert_never_resubscribes(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let mut conn = pool.acquire().await.unwrap();
    let c = input("ana@shop.com").validate().unwrap();
    let (id, _) = contacts::upsert(&mut conn, t.account.id, &c, ContactSource::Api)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE contacts SET status = 'unsubscribed', unsubscribed_at = now() WHERE id = $1",
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();
    contacts::upsert(&mut conn, t.account.id, &c, ContactSource::Import)
        .await
        .unwrap();
    let v = contacts::view(&pool, t.account.id, id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(v.status, ContactStatus::Unsubscribed);
}

#[sqlx::test]
async fn tags_ignore_case_and_a_suppressed_address_is_blocked(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let mut conn = pool.acquire().await.unwrap();
    let mut c = input("ana@shop.com");
    c.tags = vec!["VIP".into(), "News".into()];
    let (id, _) = contacts::save(
        &mut conn,
        t.account.id,
        &c.validate().unwrap(),
        ContactSource::Api,
        None,
    )
    .await
    .unwrap();
    let again = contacts::ensure_tags(&mut conn, t.account.id, &["news".into()])
        .await
        .unwrap();
    let v = contacts::view(&pool, t.account.id, id)
        .await
        .unwrap()
        .unwrap();
    let names: Vec<&str> = v.tags.0.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["News", "VIP"], "sorted, original spelling kept");
    assert_eq!(again[0], v.tags.0[0].id);

    safety::add_suppression(
        &mut conn,
        t.account.id,
        "Ana@Shop.com",
        SuppressionReason::Manual,
        None,
    )
    .await
    .unwrap();
    assert!(
        contacts::view(&pool, t.account.id, id)
            .await
            .unwrap()
            .unwrap()
            .blocked
    );
}

#[sqlx::test]
async fn another_accounts_contact_is_invisible(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let other = mailrust::accounts::create(&pool, "Other").await.unwrap();
    let mut conn = pool.acquire().await.unwrap();
    let (id, _) = contacts::upsert(
        &mut conn,
        other.id,
        &input("x@y.com").validate().unwrap(),
        ContactSource::Api,
    )
    .await
    .unwrap();
    assert!(
        contacts::view(&pool, t.account.id, id)
            .await
            .unwrap()
            .is_none()
    );
}

use axum::http::StatusCode;
use serde_json::{Value, json};

async fn add(t: &common::TestApp, body: Value) -> (StatusCode, Value) {
    common::call(
        &t.router,
        common::req("POST", "/contacts", Some(&t.key), Some(body)),
    )
    .await
}

#[sqlx::test]
async fn adding_twice_updates_and_adds_tags(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let (s, a) = add(&t, json!({"email": "Ana@Shop.com", "first_name": "Ana", "tags": ["VIP"], "fields": {"plan": "free"}})).await;
    assert_eq!(s, StatusCode::CREATED, "{a}");
    assert_eq!(a["status"], "subscribed");
    assert_eq!(a["source"], "api");
    let (s, b) = add(&t, json!({"email": " ana@shop.com ", "last_name": "Lopez", "tags": ["News"], "fields": {"plan": "pro"}})).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["id"], a["id"]);
    assert_eq!(b["first_name"], "Ana");
    assert_eq!(b["last_name"], "Lopez");
    assert_eq!(b["fields"]["plan"], "pro");
    let tags: Vec<&str> = b["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(tags, ["News", "VIP"]);
}

#[sqlx::test]
async fn bad_contacts_are_refused(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let many: serde_json::Map<String, Value> =
        (0..51).map(|i| (format!("k{i}"), json!("v"))).collect();
    for body in [
        json!({"email": "nope"}),
        json!({"email": "a@b.com", "fields": many}),
        json!({"email": "a@b.com", "fields": {"Bad Key": "v"}}),
        json!({"email": "a@b.com", "fields": {"k": "x".repeat(501)}}),
    ] {
        let (s, b) = add(&t, body).await;
        assert_eq!(s, StatusCode::BAD_REQUEST, "{b}");
    }
}

#[sqlx::test]
async fn a_dashboard_add_is_marked_dashboard(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let s = common::session_for(
        &pool,
        t.account.id,
        "me@shop.com",
        mailrust::users::MemberRole::Member,
    )
    .await;
    let (st, b) = common::call(
        &t.router,
        common::sreq(
            "POST",
            "/contacts",
            &s,
            Some(t.account.id),
            Some(json!({"email": "x@y.com"})),
        ),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    assert_eq!(b["source"], "dashboard");
}

#[sqlx::test]
async fn list_filters_and_pages(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    add(
        &t,
        json!({"email": "ana@x.com", "first_name": "Ana", "tags": ["VIP"]}),
    )
    .await;
    add(&t, json!({"email": "bo_b@x.com", "first_name": "Bob"})).await;
    let (_, c) = add(&t, json!({"email": "cy@x.com", "last_name": "Lopez"})).await;
    sqlx::query("UPDATE contacts SET status = 'unsubscribed' WHERE id = $1::uuid")
        .bind(c["id"].as_str().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    let get = |q: &str| {
        common::call(
            &t.router,
            common::req("GET", &format!("/contacts?{q}"), Some(&t.key), None),
        )
    };

    let (_, all) = get("limit=2").await;
    assert_eq!(all["data"].as_array().unwrap().len(), 2);
    assert_eq!(all["data"][0]["email"], "cy@x.com", "newest first");
    let cursor = all["next_cursor"].as_str().unwrap();
    let (_, rest) = get(&format!("limit=2&after={cursor}")).await;
    assert_eq!(rest["data"].as_array().unwrap().len(), 1);
    assert!(rest["next_cursor"].is_null());

    let (_, unsub) = get("status=unsubscribed").await;
    assert_eq!(unsub["data"].as_array().unwrap().len(), 1);
    let (_, by_name) = get("q=LOPEZ").await;
    assert_eq!(by_name["data"][0]["email"], "cy@x.com");
    let (_, literal) = get("q=o_b").await;
    assert_eq!(
        literal["data"].as_array().unwrap().len(),
        1,
        "_ is not a wildcard"
    );
    let (_, ana) = get("q=ana").await;
    let tag = ana["data"][0]["tags"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let (_, by_tag) = get(&format!("tag={tag}")).await;
    assert_eq!(by_tag["data"].as_array().unwrap().len(), 1);
}

#[sqlx::test]
async fn patch_edits_but_never_resubscribes(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let (_, a) = add(&t, json!({"email": "ana@x.com", "first_name": "Ana", "tags": ["A", "B"], "fields": {"x": "1"}})).await;
    let uri = format!("/contacts/{}", a["id"].as_str().unwrap());
    let patch = |body: Value| {
        common::call(
            &t.router,
            common::req("PATCH", &uri, Some(&t.key), Some(body)),
        )
    };

    let (s, b) = patch(json!({"first_name": "", "fields": {"y": "2"}, "tags": ["b", "C"]})).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert!(b["first_name"].is_null());
    assert_eq!(b["fields"], json!({"y": "2"}));
    let tags: Vec<&str> = b["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(tags, ["B", "C"]);

    let (s, b) = patch(json!({"status": "unsubscribed"})).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(b["status"], "unsubscribed");
    assert!(b["unsubscribed_at"].is_string());
    let (s, b) = patch(json!({"status": "subscribed"})).await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_eq!(b["error"]["code"], "resubscribe_not_allowed");
    let (s, _) = patch(json!({"status": "pending"})).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn another_account_gets_404(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let other = mailrust::accounts::create(&pool, "Other").await.unwrap();
    let key = mailrust::keys::create(&pool, other.id, "k", mailrust::keys::KeyAccess::Full)
        .await
        .unwrap()
        .token;
    let (_, a) = add(&t, json!({"email": "ana@x.com"})).await;
    let uri = format!("/contacts/{}", a["id"].as_str().unwrap());
    for (m, body) in [
        ("GET", None),
        ("PATCH", Some(json!({"first_name": "X"}))),
        ("DELETE", None),
    ] {
        let (s, _) = common::call(&t.router, common::req(m, &uri, Some(&key), body)).await;
        assert_eq!(s, StatusCode::NOT_FOUND, "{m}");
    }
}

#[sqlx::test]
async fn delete_erases_tag_links_and_confirmations(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let (_, a) = add(&t, json!({"email": "ana@x.com", "tags": ["A"]})).await;
    let id: uuid::Uuid = a["id"].as_str().unwrap().parse().unwrap();
    sqlx::query("INSERT INTO contact_confirmations (token_hash, contact_id, expires_at) VALUES ('\\x01', $1, now() + interval '1 day')")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let (s, _) = common::call(
        &t.router,
        common::req("DELETE", &format!("/contacts/{id}"), Some(&t.key), None),
    )
    .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    for table in ["contacts", "contact_tags", "contact_confirmations"] {
        let n: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n, 0, "{table}");
    }
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM tags")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 1, "the tag itself stays");
}

#[sqlx::test]
async fn tags_create_rename_count_and_delete(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let call = |m: &'static str, uri: String, body: Option<Value>| {
        let r = common::req(m, &uri, Some(&t.key), body);
        let router = t.router.clone();
        async move { common::call(&router, r).await }
    };
    let (s, vip) = call("POST", "/tags".into(), Some(json!({"name": "  VIP "}))).await;
    assert_eq!(s, StatusCode::CREATED, "{vip}");
    assert_eq!(vip["name"], "VIP");
    let (s, b) = call("POST", "/tags".into(), Some(json!({"name": "vip"}))).await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_eq!(b["error"]["code"], "tag_exists");
    let (s, _) = call("POST", "/tags".into(), Some(json!({"name": " "}))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    add(&t, json!({"email": "a@x.com", "tags": ["vip"]})).await;
    let (_, list) = call("GET", "/tags".into(), None).await;
    assert_eq!(list["data"][0]["contacts"], 1);

    let (_, news) = call("POST", "/tags".into(), Some(json!({"name": "News"}))).await;
    let vip_uri = format!("/tags/{}", vip["id"].as_str().unwrap());
    let (s, b) = call("PATCH", vip_uri.clone(), Some(json!({"name": "news"}))).await;
    assert_eq!(s, StatusCode::CONFLICT, "{b}");
    let (s, b) = call("PATCH", vip_uri.clone(), Some(json!({"name": "Members"}))).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(b["name"], "Members");
    let (s, b) = call("PATCH", vip_uri.clone(), Some(json!({"name": "members"}))).await;
    assert_eq!(
        s,
        StatusCode::OK,
        "renaming a tag to its own name in another case is fine: {b}"
    );

    let (s, _) = call("DELETE", vip_uri.clone(), None).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM contact_tags")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0);
    // The test key has used its 10 requests a second; a second key of the same account asks again.
    let key2 = mailrust::keys::create(&pool, t.account.id, "k2", mailrust::keys::KeyAccess::Full)
        .await
        .unwrap()
        .token;
    let (s, _) = common::call(
        &t.router,
        common::req("DELETE", &vip_uri, Some(&key2), None),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert!(news["id"].is_string());
}

#[sqlx::test]
async fn another_accounts_tag_is_not_found(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let other = mailrust::accounts::create(&pool, "Other").await.unwrap();
    let key = mailrust::keys::create(&pool, other.id, "k", mailrust::keys::KeyAccess::Full)
        .await
        .unwrap()
        .token;
    let (_, tag) = common::call(
        &t.router,
        common::req("POST", "/tags", Some(&t.key), Some(json!({"name": "A"}))),
    )
    .await;
    let uri = format!("/tags/{}", tag["id"].as_str().unwrap());
    let (s, _) = common::call(
        &t.router,
        common::req("PATCH", &uri, Some(&key), Some(json!({"name": "B"}))),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (s, _) = common::call(&t.router, common::req("DELETE", &uri, Some(&key), None)).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn export_is_a_csv_of_every_contact(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    add(&t, json!({"email": "a@x.com", "first_name": "Lopez, Ana", "tags": ["VIP", "news"], "fields": {"plan": "pro"}})).await;
    add(&t, json!({"email": "b@x.com", "fields": {"city": "Pune"}})).await;
    let (s, h, body) = common::call_raw(
        &t.router,
        common::req("GET", "/contacts/export", Some(&t.key), None),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert!(h["content-type"].to_str().unwrap().starts_with("text/csv"));
    assert!(
        h["content-disposition"]
            .to_str()
            .unwrap()
            .contains("contacts.csv")
    );
    let lines: Vec<&str> = body.lines().collect();
    assert_eq!(lines.len(), 3, "{body}");
    assert_eq!(
        lines[0],
        "email,first_name,last_name,status,tags,created_at,city,plan"
    );
    assert!(
        lines[1].starts_with("a@x.com,\"Lopez, Ana\",,subscribed,news;VIP,"),
        "{}",
        lines[1]
    );
    assert!(lines[1].ends_with(",,pro"), "{}", lines[1]);
    assert!(
        lines[2].starts_with("b@x.com,,,subscribed,,"),
        "{}",
        lines[2]
    );
    assert!(lines[2].ends_with(",Pune,"), "{}", lines[2]);
}

#[sqlx::test]
async fn export_walks_past_one_page(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    sqlx::query(
        "INSERT INTO contacts (id, account_id, email, status, source)
         SELECT gen_random_uuid(), $1, 'c' || g || '@x.com', 'subscribed', 'api' FROM generate_series(1, 1001) g",
    )
    .bind(t.account.id)
    .execute(&pool)
    .await
    .unwrap();
    let (s, _, body) = common::call_raw(
        &t.router,
        common::req("GET", "/contacts/export", Some(&t.key), None),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body.lines().count(), 1002);
}

#[sqlx::test]
async fn patch_cannot_subscribe_a_pending_contact(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let (_, a) = add(&t, json!({"email": "ana@x.com"})).await;
    sqlx::query("UPDATE contacts SET status = 'pending', subscribed_at = NULL")
        .execute(&pool)
        .await
        .unwrap();
    let uri = format!("/contacts/{}", a["id"].as_str().unwrap());
    let (s, b) = common::call(
        &t.router,
        common::req(
            "PATCH",
            &uri,
            Some(&t.key),
            Some(json!({"status": "subscribed"})),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT, "{b}");
    assert_eq!(b["error"]["code"], "resubscribe_not_allowed");
    let (s, b) = common::call(
        &t.router,
        common::req(
            "PATCH",
            &uri,
            Some(&t.key),
            Some(json!({"status": "unsubscribed"})),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{b}");
}

#[sqlx::test]
async fn control_characters_are_refused_not_a_500(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    for body in [
        json!({"email": "a@b.com", "first_name": "A\u{0}na"}),
        json!({"email": "a@b.com", "last_name": "Lo\u{7}pez"}),
        json!({"email": "a@b.com", "fields": {"k": "v\u{0}"}}),
        json!({"email": "a@b.com", "tags": ["V\u{0}IP"]}),
        json!({"email": "a\u{0}@b.com"}),
    ] {
        let (s, b) = add(&t, body.clone()).await;
        assert_eq!(s, StatusCode::BAD_REQUEST, "{body} -> {b}");
    }
    let (s, b) = add(
        &t,
        json!({"email": "a@b.com", "fields": {"note": "line one\nline two\ttab"}}),
    )
    .await;
    assert_eq!(
        s,
        StatusCode::CREATED,
        "line breaks and tabs are fine in field values: {b}"
    );
}

#[sqlx::test]
async fn reserved_field_keys_are_refused(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    for key in [
        "email",
        "first_name",
        "last_name",
        "status",
        "tags",
        "created_at",
    ] {
        let (s, b) = add(&t, json!({"email": "a@b.com", "fields": {key: "v"}})).await;
        assert_eq!(s, StatusCode::BAD_REQUEST, "{key}: {b}");
    }
    let (_, a) = add(&t, json!({"email": "a@b.com"})).await;
    let uri = format!("/contacts/{}", a["id"].as_str().unwrap());
    let (s, _) = common::call(
        &t.router,
        common::req(
            "PATCH",
            &uri,
            Some(&t.key),
            Some(json!({"fields": {"status": "x"}})),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

/// Two writers that name the same existing tags in opposite order must not lock each other out.
#[sqlx::test]
async fn tag_upserts_in_opposite_order_do_not_deadlock(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let acc = t.account.id;
    let (a, b) = ("Alpha".to_string(), "Beta".to_string());
    {
        let mut c = pool.acquire().await.unwrap();
        contacts::ensure_tags(&mut c, acc, &[a.clone(), b.clone()])
            .await
            .unwrap();
    }
    let mut t1 = pool.begin().await.unwrap();
    let mut t2 = pool.begin().await.unwrap();
    contacts::ensure_tags(&mut t1, acc, std::slice::from_ref(&a))
        .await
        .unwrap();
    contacts::ensure_tags(&mut t2, acc, std::slice::from_ref(&b))
        .await
        .unwrap();
    let r = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let one = contacts::ensure_tags(&mut t1, acc, std::slice::from_ref(&b));
        let two = contacts::ensure_tags(&mut t2, acc, std::slice::from_ref(&a));
        tokio::join!(one, two)
    })
    .await
    .expect("the two transactions wait on each other");
    r.0.unwrap();
    r.1.unwrap();
}

#[sqlx::test]
async fn ensure_tags_dedupes_and_keeps_input_order(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let mut c = pool.acquire().await.unwrap();
    let ids = contacts::ensure_tags(&mut c, t.account.id, &["b".into(), "A".into(), "B".into()])
        .await
        .unwrap();
    assert_eq!(ids.len(), 3);
    assert_eq!(ids[0], ids[2], "same tag, ignoring case");
    assert_ne!(ids[0], ids[1]);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM tags")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 2);
}
