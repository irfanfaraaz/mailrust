mod common;

use mailrust::{
    contacts::{self, ContactInput, ContactSource, ContactStatus},
    safety::{self, SuppressionReason},
};

fn input(email: &str) -> ContactInput {
    ContactInput { email: email.into(), ..Default::default() }
}

#[sqlx::test]
async fn upsert_adds_once_then_merges(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let mut conn = pool.acquire().await.unwrap();
    let a = input(" Ana@Shop.COM ").validate().unwrap();
    let (id1, new1) = contacts::upsert(&mut conn, t.account.id, &a, ContactSource::Api).await.unwrap();

    let mut b = input("ana@shop.com");
    b.first_name = Some("Ana".into());
    b.fields.insert("plan".into(), "pro".into());
    let (id2, new2) = contacts::upsert(&mut conn, t.account.id, &b.validate().unwrap(), ContactSource::Import)
        .await
        .unwrap();
    assert!(new1 && !new2);
    assert_eq!(id1, id2);

    let v = contacts::view(&pool, t.account.id, id1).await.unwrap().unwrap();
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
    let (id, _) = contacts::upsert(&mut conn, t.account.id, &c, ContactSource::Api).await.unwrap();
    sqlx::query("UPDATE contacts SET status = 'unsubscribed', unsubscribed_at = now() WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    contacts::upsert(&mut conn, t.account.id, &c, ContactSource::Import).await.unwrap();
    let v = contacts::view(&pool, t.account.id, id).await.unwrap().unwrap();
    assert_eq!(v.status, ContactStatus::Unsubscribed);
}

#[sqlx::test]
async fn tags_ignore_case_and_a_suppressed_address_is_blocked(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let mut conn = pool.acquire().await.unwrap();
    let mut c = input("ana@shop.com");
    c.tags = vec!["VIP".into(), "News".into()];
    let (id, _) = contacts::save(&mut conn, t.account.id, &c.validate().unwrap(), ContactSource::Api, None)
        .await
        .unwrap();
    let again = contacts::ensure_tags(&mut conn, t.account.id, &["news".into()]).await.unwrap();
    let v = contacts::view(&pool, t.account.id, id).await.unwrap().unwrap();
    let names: Vec<&str> = v.tags.0.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["News", "VIP"], "sorted, original spelling kept");
    assert_eq!(again[0], v.tags.0[0].id);

    safety::add_suppression(&mut conn, t.account.id, "Ana@Shop.com", SuppressionReason::Manual, None)
        .await
        .unwrap();
    assert!(contacts::view(&pool, t.account.id, id).await.unwrap().unwrap().blocked);
}

#[sqlx::test]
async fn another_accounts_contact_is_invisible(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let other = mailrust::accounts::create(&pool, "Other").await.unwrap();
    let mut conn = pool.acquire().await.unwrap();
    let (id, _) = contacts::upsert(&mut conn, other.id, &input("x@y.com").validate().unwrap(), ContactSource::Api)
        .await
        .unwrap();
    assert!(contacts::view(&pool, t.account.id, id).await.unwrap().is_none());
}
