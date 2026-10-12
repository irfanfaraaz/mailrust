mod common;

use axum::http::StatusCode;
use mailrust::{
    accounts,
    keys::{self, KeyAccess},
};
use serde_json::json;

#[sqlx::test]
async fn create_list_get_patch_delete(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let k = Some(t.key.as_str());

    let (s, d) = common::call(
        &t.router,
        common::req("POST", "/domains", k, Some(json!({ "name": "Shop.com" }))),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{d}");
    assert_eq!(d["name"], "shop.com");
    assert_eq!(d["status"], "verified");
    assert!(
        d["dns_records"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["name"] == "_dmarc.shop.com")
    );
    let id = d["id"].as_str().unwrap().to_string();

    let (s, l) = common::call(&t.router, common::req("GET", "/domains", k, None)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(l["data"].as_array().unwrap().len(), 1);

    let (s, p) = common::call(
        &t.router,
        common::req(
            "PATCH",
            &format!("/domains/{id}"),
            k,
            Some(json!({ "click_tracking": true })),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(p["click_tracking"], true);
    assert_eq!(p["open_tracking"], false);

    let (s, _) = common::call(
        &t.router,
        common::req("POST", &format!("/domains/{id}/verify"), k, None),
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    let (s, _) = common::call(
        &t.router,
        common::req("DELETE", &format!("/domains/{id}"), k, None),
    )
    .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let (s, _) = common::call(
        &t.router,
        common::req("GET", &format!("/domains/{id}"), k, None),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn duplicates_bad_names_and_bad_tracking_hosts(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let k = Some(t.key.as_str());
    common::call(
        &t.router,
        common::req("POST", "/domains", k, Some(json!({ "name": "shop.com" }))),
    )
    .await;
    let (s, e) = common::call(
        &t.router,
        common::req("POST", "/domains", k, Some(json!({ "name": "shop.com" }))),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_eq!(e["error"]["code"], "domain_exists");

    let (s, _) = common::call(
        &t.router,
        common::req("POST", "/domains", k, Some(json!({ "name": "nodot" }))),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    let (s, _) = common::call(
        &t.router,
        common::req(
            "POST",
            "/domains",
            k,
            Some(json!({ "name": "a.com", "tracking_subdomain": "track.other.com" })),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    let (s, d) = common::call(
        &t.router,
        common::req(
            "POST",
            "/domains",
            k,
            Some(json!({ "name": "b.com", "tracking_subdomain": "track.b.com" })),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);
    assert!(
        d["dns_records"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["name"] == "track.b.com" && r["value"] == "mail.test")
    );
    assert_eq!(d["tracking_verified"], false);
}

#[sqlx::test]
async fn sending_keys_and_other_accounts_are_kept_out(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let sending = keys::create(&t.state.pool, t.account.id, "s", KeyAccess::Sending)
        .await
        .unwrap()
        .token;
    let (s, _) = common::call(
        &t.router,
        common::req("GET", "/domains", Some(&sending), None),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);

    let d = common::verified_domain(&t, "shop.com").await;
    let other = accounts::create(&t.state.pool, "Other").await.unwrap();
    let other_key = keys::create(&t.state.pool, other.id, "o", KeyAccess::Full)
        .await
        .unwrap()
        .token;
    let (s, _) = common::call(
        &t.router,
        common::req("GET", &format!("/domains/{}", d.id), Some(&other_key), None),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

struct PendingSender;

#[async_trait::async_trait]
impl mailrust::sender::Sender for PendingSender {
    async fn provision_account(
        &self,
        _: &mailrust::sender::TenantRef,
    ) -> Result<(), mailrust::sender::SenderError> {
        Ok(())
    }
    async fn create_domain(
        &self,
        _: &mailrust::sender::TenantRef,
        _: &str,
    ) -> Result<Vec<mailrust::sender::DnsRecord>, mailrust::sender::SenderError> {
        Ok(vec![])
    }
    async fn domain_state(
        &self,
        _: &mailrust::sender::TenantRef,
        _: &str,
    ) -> Result<mailrust::sender::DomainState, mailrust::sender::SenderError> {
        Ok(mailrust::sender::DomainState::Pending)
    }
    async fn send(
        &self,
        _: &mailrust::sender::OutgoingEmail,
    ) -> Result<String, mailrust::sender::SenderError> {
        unreachable!()
    }
}

#[sqlx::test]
async fn a_domain_that_never_verifies_fails_after_its_window(pool: sqlx::PgPool) {
    use mailrust::domains::{self, AddDomain, DomainStatus};
    let t = common::app_with_sender(pool, std::sync::Arc::new(PendingSender)).await;
    let add = |name: &str| AddDomain {
        name: name.into(),
        tracking_subdomain: None,
        open_tracking: false,
        click_tracking: false,
    };
    let fresh = domains::add(&t.state, &t.account, add("fresh.com"))
        .await
        .unwrap();
    assert_eq!(
        domains::check(&t.state, &fresh).await.unwrap().status,
        DomainStatus::Pending
    );

    let old = domains::add(&t.state, &t.account, add("old.com"))
        .await
        .unwrap();
    sqlx::query("UPDATE domains SET verify_until = now() - interval '1 minute' WHERE id = $1")
        .bind(old.id)
        .execute(&t.state.pool)
        .await
        .unwrap();
    let old = domains::get_by_id(&t.state.pool, old.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        domains::check(&t.state, &old).await.unwrap().status,
        DomainStatus::Failed
    );
}
