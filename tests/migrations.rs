#[sqlx::test]
async fn migrations_create_every_table(pool: sqlx::PgPool) {
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name::text FROM information_schema.tables WHERE table_schema = 'public'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    for t in [
        "accounts",
        "api_keys",
        "attachments",
        "domains",
        "email_events",
        "emails",
        "sender_quota",
        "sns_messages",
        "suppressions",
        "webhook_deliveries",
        "webhooks",
    ] {
        assert!(tables.iter().any(|x| x == t), "missing table {t}");
    }
}
