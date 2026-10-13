#[sqlx::test]
async fn migrations_create_every_table(pool: sqlx::PgPool) {
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name::text FROM information_schema.tables WHERE table_schema = 'public'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    for t in [
        "account_members",
        "accounts",
        "api_keys",
        "attachments",
        "contact_confirmations",
        "contact_imports",
        "contact_tags",
        "contacts",
        "domains",
        "email_events",
        "emails",
        "invites",
        "join_requests",
        "login_requests",
        "login_tokens",
        "sender_quota",
        "sessions",
        "sns_messages",
        "suppressions",
        "tags",
        "users",
        "webhook_deliveries",
        "webhooks",
    ] {
        assert!(tables.iter().any(|x| x == t), "missing table {t}");
    }
}

#[sqlx::test]
async fn the_unused_urgent_index_is_gone(pool: sqlx::PgPool) {
    let found: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'emails_account_urgent')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        !found,
        "emails_account_urgent is never used and slows inserts"
    );
}
