#![allow(dead_code)]
use std::{collections::HashMap, path::PathBuf, sync::Arc};

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
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
        (
            "TRACKING_SECRET",
            "kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk",
        ),
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
    let key = keys::create(&pool, account.id, "test", KeyAccess::Full)
        .await
        .unwrap()
        .token;
    let state = AppState::new(pool, test_config(outbox.clone()), sender);
    let router = api::router(state.clone());
    TestApp {
        state,
        router,
        account,
        key,
        outbox,
    }
}

pub fn req(method: &str, uri: &str, key: Option<&str>, body: Option<Value>) -> Request<Body> {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(k) = key {
        b = b.header("authorization", format!("Bearer {k}"));
    }
    match body {
        Some(v) => b
            .header("content-type", "application/json")
            .body(Body::from(v.to_string()))
            .unwrap(),
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

/// Like `call`, for non-JSON answers (CSV): status, headers and the body as text.
pub async fn call_raw(
    router: &Router,
    req: Request<Body>,
) -> (StatusCode, axum::http::HeaderMap, String) {
    let res = router.clone().oneshot(req).await.unwrap();
    let (status, headers) = (res.status(), res.headers().clone());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        headers,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

pub async fn verified_domain(t: &TestApp, name: &str) -> mailrust::domains::Domain {
    let d = mailrust::domains::add(
        &t.state,
        &t.account,
        mailrust::domains::AddDomain {
            name: name.into(),
            tracking_subdomain: None,
            open_tracking: false,
            click_tracking: false,
        },
    )
    .await
    .unwrap();
    assert_eq!(d.status, mailrust::domains::DomainStatus::Verified);
    d
}

use mailrust::sender::{DnsRecord, DomainState, OutgoingEmail, SendQuota, SenderError, TenantRef};

/// Records what it was asked to send. `fail_with` makes every send fail with that error.
pub struct RecordingSender {
    pub sent: std::sync::Mutex<Vec<OutgoingEmail>>,
    pub fail_with: Option<fn() -> SenderError>,
    pub quota: std::sync::Mutex<Option<Result<SendQuota, String>>>,
    /// Every call to `send`, failed ones too.
    pub calls: std::sync::atomic::AtomicUsize,
}

impl RecordingSender {
    pub fn ok() -> Arc<Self> {
        Arc::new(Self {
            sent: Default::default(),
            fail_with: None,
            quota: Default::default(),
            calls: Default::default(),
        })
    }
    pub fn failing(f: fn() -> SenderError) -> Arc<Self> {
        Arc::new(Self {
            sent: Default::default(),
            fail_with: Some(f),
            quota: Default::default(),
            calls: Default::default(),
        })
    }
    pub fn sent(&self) -> Vec<OutgoingEmail> {
        self.sent.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl Sender for RecordingSender {
    async fn provision_account(&self, _: &TenantRef) -> Result<(), SenderError> {
        Ok(())
    }
    async fn create_domain(&self, _: &TenantRef, _: &str) -> Result<Vec<DnsRecord>, SenderError> {
        Ok(vec![])
    }
    async fn domain_state(&self, _: &TenantRef, _: &str) -> Result<DomainState, SenderError> {
        Ok(DomainState::Verified)
    }
    async fn send(&self, email: &OutgoingEmail) -> Result<String, SenderError> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if let Some(f) = self.fail_with {
            return Err(f());
        }
        self.sent.lock().unwrap().push(email.clone());
        Ok(format!("rec-{}", email.email_id))
    }
    async fn quota(&self) -> Result<Option<SendQuota>, SenderError> {
        match self.quota.lock().unwrap().clone() {
            None => Ok(None),
            Some(Ok(q)) => Ok(Some(q)),
            Some(Err(m)) => Err(SenderError::Retryable(m)),
        }
    }
}

pub async fn send_email(t: &TestApp, body: serde_json::Value) -> uuid::Uuid {
    let (s, b) = call(&t.router, req("POST", "/emails", Some(&t.key), Some(body))).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    b["id"].as_str().unwrap().parse().unwrap()
}

pub async fn status_of(pool: &PgPool, id: uuid::Uuid) -> String {
    sqlx::query_scalar("SELECT status::text FROM emails WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Inserts `n` bare emails straight into the table (no domain). `created_at` and `send_at` are `hours_ago` back.
pub async fn seed_emails(
    pool: &PgPool,
    account_id: Uuid,
    priority: &str,
    status: &str,
    n: i64,
    hours_ago: f64,
) -> Vec<Uuid> {
    sqlx::query_scalar(
        "INSERT INTO emails (id, account_id, from_address, to_addresses, subject, text_body, priority, status, created_at, send_at)
         SELECT gen_random_uuid(), $1, 'hi@shop.com', ARRAY['to@x.com'], 's', 't', $2::email_priority, $3::email_status,
                now() - make_interval(secs => $5), now() - make_interval(secs => $5)
         FROM generate_series(1, $4) RETURNING id",
    )
    .bind(account_id).bind(priority).bind(status).bind(n).bind(hours_ago * 3600.0)
    .fetch_all(pool).await.unwrap()
}

/// Makes `t.account` the system sender with `from`, its domain verified.
pub async fn with_system_sender(t: &mut TestApp, from: &str) {
    let domain = from
        .rsplit('@')
        .next()
        .unwrap()
        .trim_end_matches('>')
        .to_string();
    verified_domain(t, &domain).await;
    let mut config = (*t.state.config).clone();
    config.system_account_id = Some(t.account.id);
    config.system_from = Some(from.to_string());
    config.web_url = "http://app.test".parse().unwrap();
    t.state.config = std::sync::Arc::new(config);
    t.router = api::router(t.state.clone());
}

/// Makes `email` a member of `account_id` with `role` and returns a fresh session token.
pub async fn session_for(
    pool: &PgPool,
    account_id: Uuid,
    email: &str,
    role: mailrust::users::MemberRole,
) -> String {
    let (user, _) = mailrust::users::invite_direct(pool, email, account_id, role)
        .await
        .unwrap();
    let mut conn = pool.acquire().await.unwrap();
    mailrust::sessions::create(&mut conn, user.id)
        .await
        .unwrap()
        .token
}

/// A request with a session token and, if given, the `x-mailrust-account` header.
pub fn sreq(
    method: &str,
    uri: &str,
    session: &str,
    account: Option<Uuid>,
    body: Option<Value>,
) -> Request<Body> {
    let mut r = req(method, uri, Some(session), body);
    if let Some(a) = account {
        r.headers_mut()
            .insert("x-mailrust-account", a.to_string().parse().unwrap());
    }
    r
}

pub const WEB_SECRET: &str = "web-secret-for-tests-0123456789abcdef";

/// Lets the router trust `X-Mailrust-Web` (the Next.js app) with `WEB_SECRET`.
pub fn with_web_secret(t: &mut TestApp) {
    let mut config = (*t.state.config).clone();
    config.web_secret = Some(WEB_SECRET.to_string());
    t.state.config = std::sync::Arc::new(config);
    t.router = api::router(t.state.clone());
}

/// A request as the Next.js app sends it for a browser at `ip`.
pub fn web_req(method: &str, uri: &str, ip: &str, body: Option<Value>) -> Request<Body> {
    let mut r = req(method, uri, None, body);
    r.headers_mut()
        .insert("x-mailrust-web", WEB_SECRET.parse().unwrap());
    r.headers_mut()
        .insert("x-mailrust-client-ip", ip.parse().unwrap());
    r
}

/// Polls `f` every 20 ms for up to 5 s (for work done after the response, like the join email).
pub async fn eventually<F, Fut>(mut f: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for _ in 0..250 {
        if f().await {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("condition not met within 5 s");
}

/// A 1x1 PNG.
pub const PNG_1X1: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

pub fn png_1x1() -> Vec<u8> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(PNG_1X1)
        .unwrap()
}

/// `POST uri` with one multipart field `file` holding `bytes`, declared as `declared_type`.
pub fn multipart_file(uri: &str, key: &str, bytes: &[u8], declared_type: &str) -> Request<Body> {
    let boundary = "mailrust-test-boundary";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"upload\"\r\nContent-Type: {declared_type}\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("authorization", format!("Bearer {key}"))
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .unwrap()
}
