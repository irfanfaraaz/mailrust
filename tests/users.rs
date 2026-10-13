mod common;

use mailrust::{accounts, users::{self, MemberRole}};

#[sqlx::test]
async fn invite_direct_is_safe_to_run_again(pool: sqlx::PgPool) {
    let acc = accounts::create(&pool, "Shop").await.unwrap();
    let (u1, r1) = users::invite_direct(&pool, "Ana@Shop.com", acc.id, MemberRole::Owner).await.unwrap();
    let (u2, r2) = users::invite_direct(&pool, "ana@shop.com", acc.id, MemberRole::Owner).await.unwrap();
    assert_eq!(u1.id, u2.id);
    assert_eq!(u1.email, "ana@shop.com");
    assert_eq!((r1, r2), (MemberRole::Owner, MemberRole::Owner));
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM account_members").fetch_one(&pool).await.unwrap();
    assert_eq!(n, 1);
}

#[sqlx::test]
async fn adding_a_member_never_lowers_an_owner(pool: sqlx::PgPool) {
    let acc = accounts::create(&pool, "Shop").await.unwrap();
    users::invite_direct(&pool, "ana@shop.com", acc.id, MemberRole::Owner).await.unwrap();
    let (_, role) = users::invite_direct(&pool, "ana@shop.com", acc.id, MemberRole::Member).await.unwrap();
    assert_eq!(role, MemberRole::Owner);
}

#[sqlx::test]
async fn invite_direct_rejects_a_bad_email(pool: sqlx::PgPool) {
    let acc = accounts::create(&pool, "Shop").await.unwrap();
    assert!(users::invite_direct(&pool, "nope", acc.id, MemberRole::Owner).await.is_err());
}
