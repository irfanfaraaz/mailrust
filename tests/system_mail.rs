mod common;

use mailrust::system_mail;

#[sqlx::test]
async fn queues_a_text_only_urgent_email_from_the_system_address(pool: sqlx::PgPool) {
    let mut t = common::app(pool.clone()).await;
    common::with_system_sender(&mut t, "Mailrust <login@mail.shop.com>").await;
    let id = system_mail::queue(&t.state, "ana@x.com", "Hi", "link: http://x/y".into())
        .await
        .unwrap()
        .expect("queued");
    let row = sqlx::query!(
        r#"SELECT from_address, to_addresses, html, text_body, priority::text AS "p!" FROM emails WHERE id = $1"#,
        id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.from_address, "Mailrust <login@mail.shop.com>");
    assert_eq!(row.to_addresses, vec!["ana@x.com".to_string()]);
    assert!(
        row.html.is_none(),
        "text only: no click tracking on one-time links"
    );
    assert_eq!(row.p, "urgent");
}

#[sqlx::test]
async fn not_set_up_sends_nothing(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    assert!(
        system_mail::queue(&t.state, "ana@x.com", "Hi", "x".into())
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        system_mail::setup_problem(&t.state)
            .await
            .unwrap()
            .is_some()
    );
}

#[sqlx::test]
async fn unverified_system_domain_sends_nothing(pool: sqlx::PgPool) {
    let mut t = common::app(pool.clone()).await;
    common::with_system_sender(&mut t, "login@mail.shop.com").await;
    sqlx::query("UPDATE domains SET status = 'pending'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        system_mail::queue(&t.state, "ana@x.com", "Hi", "x".into())
            .await
            .unwrap()
            .is_none()
    );
    let p = system_mail::setup_problem(&t.state).await.unwrap().unwrap();
    assert!(p.contains("not verified"), "{p}");
}

#[sqlx::test]
async fn system_account_members_cannot_read_login_links_through_the_api(pool: sqlx::PgPool) {
    let mut t = common::app(pool.clone()).await;
    common::with_system_sender(&mut t, "login@mail.shop.com").await;
    let member = common::session_for(
        &pool,
        t.account.id,
        "m@shop.com",
        mailrust::users::MemberRole::Member,
    )
    .await;
    let (s, _) = common::call(
        &t.router,
        common::req(
            "POST",
            "/auth/login",
            None,
            Some(serde_json::json!({"email": "m@shop.com"})),
        ),
    )
    .await;
    assert_eq!(s, axum::http::StatusCode::ACCEPTED);
    let (id, stored): (uuid::Uuid, String) =
        sqlx::query_as("SELECT id, text_body FROM emails WHERE 'm@shop.com' = ANY(to_addresses)")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        stored.contains("mr_login_"),
        "the worker still sends the link"
    );

    let by_session = common::call(
        &t.router,
        common::sreq(
            "GET",
            &format!("/emails/{id}"),
            &member,
            Some(t.account.id),
            None,
        ),
    )
    .await;
    let by_key = common::call(
        &t.router,
        common::req("GET", &format!("/emails/{id}"), Some(&t.key), None),
    )
    .await;
    for (s, b) in [by_session, by_key] {
        assert_eq!(s, axum::http::StatusCode::OK, "{b}");
        assert!(!b.to_string().contains("mr_login_"), "{b}");
        assert!(b["text"].is_null() && b["html"].is_null(), "{b}");
    }
}

#[sqlx::test]
async fn customers_cannot_mark_their_own_email_as_system(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    common::verified_domain(&t, "shop.com").await;
    let (s, b) = common::call(
        &t.router,
        common::req(
            "POST",
            "/emails",
            Some(&t.key),
            Some(serde_json::json!({
                "from": "hi@shop.com", "to": "a@x.com", "subject": "Hi", "text": "x",
                "tags": { system_mail::SYSTEM_TAG: "true" }
            })),
        ),
    )
    .await;
    assert_eq!(s, axum::http::StatusCode::BAD_REQUEST, "{b}");
}

#[sqlx::test]
async fn a_paused_system_account_is_a_setup_problem(pool: sqlx::PgPool) {
    let mut t = common::app(pool.clone()).await;
    common::with_system_sender(&mut t, "login@mail.shop.com").await;
    mailrust::accounts::pause(&pool, t.account.id, "bounce rate")
        .await
        .unwrap();
    let p = system_mail::setup_problem(&t.state).await.unwrap().unwrap();
    assert!(p.contains("paused"), "{p}");
    assert!(
        system_mail::queue(&t.state, "ana@x.com", "Hi", "x".into())
            .await
            .unwrap()
            .is_none()
    );
}
