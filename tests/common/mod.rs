#![allow(dead_code)]
use std::{collections::HashMap, path::PathBuf, sync::Arc};

use axum::{Router, body::Body, http::{Request, StatusCode}};
use http_body_util::BodyExt;
use mailrust::{
    accounts::{self, Account},
    api::{self, AppState},
    config::Config,
    keys::{self, KeyAccess},
    sender::{Sender, fake::FakeSender},
};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

pub struct TestApp {
    pub state: AppState,
    pub router: Router,
    pub account: Account,
    pub key: String,
    pub outbox: PathBuf,
}

pub fn test_config(outbox: PathBuf) -> Config {
    let env: HashMap<String, String> = [
        ("DATABASE_URL", "postgres://unused"),
        ("PUBLIC_URL", "http://mail.test"),
        ("TRACKING_SECRET", "kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk"),
        ("MAILRUST_ALLOW_HTTP_WEBHOOKS", "true"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .chain([("OUTBOX_DIR".to_string(), outbox.display().to_string())])
    .collect();
    Config::from_map(&env).expect("test config")
}

pub fn temp_outbox() -> PathBuf {
    std::env::temp_dir().join(format!("mailrust-test-{}", Uuid::now_v7()))
}

pub async fn app(pool: PgPool) -> TestApp {
    let outbox = temp_outbox();
    let sender: Arc<dyn Sender> = Arc::new(FakeSender::new(outbox.clone()));
    build(pool, sender, outbox).await
}

pub async fn app_with_sender(pool: PgPool, sender: Arc<dyn Sender>) -> TestApp {
    build(pool, sender, temp_outbox()).await
}

async fn build(pool: PgPool, sender: Arc<dyn Sender>, outbox: PathBuf) -> TestApp {
    let account = accounts::create(&pool, "Test").await.unwrap();
    let key = keys::create(&pool, account.id, "test", KeyAccess::Full).await.unwrap().token;
    let state = AppState::new(pool, test_config(outbox.clone()), sender);
    let router = api::router(state.clone());
    TestApp { state, router, account, key, outbox }
}

pub fn req(method: &str, uri: &str, key: Option<&str>, body: Option<Value>) -> Request<Body> {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(k) = key {
        b = b.header("authorization", format!("Bearer {k}"));
    }
    match body {
        Some(v) => b.header("content-type", "application/json").body(Body::from(v.to_string())).unwrap(),
        None => b.body(Body::empty()).unwrap(),
    }
}

pub async fn call(router: &Router, req: Request<Body>) -> (StatusCode, Value) {
    let res = router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

pub async fn verified_domain(t: &TestApp, name: &str) -> mailrust::domains::Domain {
    let d = mailrust::domains::add(&t.state, &t.account, mailrust::domains::AddDomain {
        name: name.into(),
        tracking_subdomain: None,
        open_tracking: false,
        click_tracking: false,
    })
    .await
    .unwrap();
    assert_eq!(d.status, mailrust::domains::DomainStatus::Verified);
    d
}
