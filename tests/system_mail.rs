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
    assert!(system_mail::setup_problem(&t.state).await.unwrap().is_some());
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
