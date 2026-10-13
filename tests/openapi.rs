mod common;

use axum::http::StatusCode;

#[sqlx::test]
async fn openapi_lists_every_public_route(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let (s, doc) = common::call(&t.router, common::req("GET", "/openapi.json", None, None)).await;
    assert_eq!(s, StatusCode::OK);
    let paths = doc["paths"].as_object().unwrap();
    for p in [
        "/emails",
        "/emails/batch",
        "/emails/{id}",
        "/emails/{id}/cancel",
        "/domains",
        "/domains/{id}",
        "/domains/{id}/verify",
        "/api-keys",
        "/api-keys/{id}",
        "/webhooks",
        "/webhooks/{id}",
        "/suppressions",
        "/suppressions/{address}",
        "/health",
        "/public/confirm/{token}",
        "/images",
        "/public/img/{id}",
        "/campaigns",
        "/campaigns/{id}",
        "/campaigns/{id}/preview",
        "/campaigns/{id}/test",
        "/campaigns/{id}/send",
        "/campaigns/{id}/pause",
        "/campaigns/{id}/report",
        "/campaigns/{id}/resume",
        "/campaigns/{id}/cancel",
    ] {
        assert!(paths.contains_key(p), "missing {p}");
    }
    assert!(doc["components"]["schemas"]["SendEmailRequest"].is_object());
    assert!(doc["components"]["securitySchemes"]["bearer"].is_object());
}
