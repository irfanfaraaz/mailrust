use mailrust::{accounts, accounts::AccountStatus, keys, keys::KeyAccess};

#[sqlx::test]
async fn account_gets_a_valid_tenant_name(pool: sqlx::PgPool) {
    let a = accounts::create(&pool, "Shop").await.unwrap();
    assert_eq!(a.status, AccountStatus::Active);
    assert!(a.ses_tenant_name.starts_with("mailrust-"));
    assert!(a.ses_tenant_name.len() <= 64);
    assert!(
        a.ses_tenant_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    );
}

#[sqlx::test]
async fn pause_and_resume_report_real_changes(pool: sqlx::PgPool) {
    let a = accounts::create(&pool, "Shop").await.unwrap();
    assert!(accounts::pause(&pool, a.id, "test").await.unwrap());
    assert!(!accounts::pause(&pool, a.id, "again").await.unwrap());
    let paused = accounts::get(&pool, a.id).await.unwrap().unwrap();
    assert_eq!(paused.status, AccountStatus::Paused);
    assert_eq!(paused.paused_reason.as_deref(), Some("test"));
    assert!(accounts::resume(&pool, a.id).await.unwrap());
    assert!(!accounts::resume(&pool, a.id).await.unwrap());
}

#[sqlx::test]
async fn key_lookup_and_revoke(pool: sqlx::PgPool) {
    let a = accounts::create(&pool, "Shop").await.unwrap();
    let k = keys::create(&pool, a.id, "main", KeyAccess::Sending)
        .await
        .unwrap();
    let ctx = keys::find_by_token(&pool, &k.token).await.unwrap().unwrap();
    assert_eq!(ctx.account_id, a.id);
    assert_eq!(ctx.access, KeyAccess::Sending);
    assert!(
        keys::find_by_token(&pool, "mr_live_wrong")
            .await
            .unwrap()
            .is_none()
    );

    let other = accounts::create(&pool, "Other").await.unwrap();
    assert!(
        !keys::revoke(&pool, Some(other.id), k.id).await.unwrap(),
        "other account cannot revoke"
    );
    assert!(keys::revoke(&pool, Some(a.id), k.id).await.unwrap());
    assert!(
        keys::find_by_token(&pool, &k.token)
            .await
            .unwrap()
            .is_none()
    );
    assert!(keys::list(&pool, a.id).await.unwrap().is_empty());
}
