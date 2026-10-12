mod common;

use axum::http::StatusCode;
use base64::{Engine, engine::general_purpose::STANDARD};
use mailrust::{
    api::{self, AppState},
    config::SesConfig,
    events::{self, Applied, SesEvent, sns},
    limiter::SendLimiter,
    queue, worker,
};
use rsa::signature::{SignatureEncoding, Signer};
use serde_json::{Value, json};
use uuid::Uuid;

fn ses(kind: &str, email_id: Uuid, extra: Value) -> SesEvent {
    let mut v = json!({
        "eventType": kind,
        "mail": { "messageId": "ses-1", "destination": ["a@x.com"], "tags": { "mailrust_email_id": [email_id.to_string()] } }
    });
    if let (Some(o), Some(e)) = (v.as_object_mut(), extra.as_object()) {
        o.extend(e.clone());
    }
    serde_json::from_value(v).unwrap()
}

async fn queued(t: &common::TestApp) -> Uuid {
    common::verified_domain(t, "shop.com").await;
    common::send_email(
        t,
        json!({ "from": "hi@shop.com", "to": "a@x.com", "subject": "Hi", "text": "x" }),
    )
    .await
}

async fn types(pool: &sqlx::PgPool, id: Uuid) -> Vec<String> {
    sqlx::query_scalar("SELECT type FROM email_events WHERE email_id = $1 ORDER BY id")
        .bind(id)
        .fetch_all(pool)
        .await
        .unwrap()
}

#[sqlx::test]
async fn delivery_then_delay_keeps_delivered(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let id = queued(&t).await;
    events::ingest(
        &t.state.pool,
        &ses(
            "Delivery",
            id,
            json!({ "delivery": { "recipients": ["a@x.com"] } }),
        ),
        None,
    )
    .await
    .unwrap();
    events::ingest(
        &t.state.pool,
        &ses(
            "DeliveryDelay",
            id,
            json!({ "deliveryDelay": { "delayedRecipients": [{ "emailAddress": "a@x.com" }] } }),
        ),
        None,
    )
    .await
    .unwrap();
    assert_eq!(common::status_of(&t.state.pool, id).await, "delivered");
    assert_eq!(
        types(&t.state.pool, id).await,
        vec!["delivered", "delivery_delayed"]
    );
}

#[sqlx::test]
async fn hard_bounce_blocks_soft_bounce_does_not(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let id = queued(&t).await;
    let soft = json!({ "bounce": { "bounceType": "Transient", "bouncedRecipients": [{ "emailAddress": "a@x.com" }] } });
    events::ingest(&t.state.pool, &ses("Bounce", id, soft), None)
        .await
        .unwrap();
    assert_eq!(common::status_of(&t.state.pool, id).await, "queued");
    let hard = json!({ "bounce": { "bounceType": "Permanent", "bouncedRecipients": [{ "emailAddress": "A@X.com", "diagnosticCode": "550" }] } });
    let applied = events::ingest(&t.state.pool, &ses("Bounce", id, hard), None)
        .await
        .unwrap();
    assert_eq!(
        applied,
        Applied::Done {
            account_id: t.account.id,
            affects_safety: true
        }
    );
    assert_eq!(common::status_of(&t.state.pool, id).await, "bounced");
    let blocked: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM suppressions WHERE address = 'a@x.com' AND reason = 'bounce'",
    )
    .fetch_one(&t.state.pool)
    .await
    .unwrap();
    assert_eq!(blocked, 1);
}

#[sqlx::test]
async fn complaint_and_reject(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let id = queued(&t).await;
    events::ingest(
        &t.state.pool,
        &ses(
            "Complaint",
            id,
            json!({ "complaint": { "complainedRecipients": [{ "emailAddress": "a@x.com" }] } }),
        ),
        None,
    )
    .await
    .unwrap();
    assert_eq!(common::status_of(&t.state.pool, id).await, "complained");
    events::ingest(
        &t.state.pool,
        &ses(
            "Reject",
            id,
            json!({ "reject": { "reason": "Bad content" } }),
        ),
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        common::status_of(&t.state.pool, id).await,
        "complained",
        "worse news is not replaced by milder news"
    );
}

#[sqlx::test]
async fn unmatched_and_duplicate(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let id = queued(&t).await;
    let stranger = ses("Delivery", Uuid::now_v7(), json!({}));
    assert_eq!(
        events::ingest(&t.state.pool, &stranger, None)
            .await
            .unwrap(),
        Applied::Unmatched
    );
    let ev = ses("Delivery", id, json!({}));
    events::ingest(&t.state.pool, &ev, Some("sns-1"))
        .await
        .unwrap();
    assert_eq!(
        events::ingest(&t.state.pool, &ev, Some("sns-1"))
            .await
            .unwrap(),
        Applied::Duplicate
    );
    assert_eq!(types(&t.state.pool, id).await.len(), 1);
}

#[sqlx::test]
async fn delivery_before_mark_sent_keeps_delivered_and_saves_id(pool: sqlx::PgPool) {
    let rec = common::RecordingSender::ok();
    let t = common::app_with_sender(pool, rec).await;
    let id = queued(&t).await;
    let claimed = queue::claim(&t.state.pool, 10)
        .await
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(claimed.id, id);
    events::ingest(&t.state.pool, &ses("Delivery", id, json!({})), None)
        .await
        .unwrap();
    queue::mark_sent(&t.state.pool, &claimed, "ses-1", &["a@x.com".to_string()])
        .await
        .unwrap();
    let (status, pid): (String, Option<String>) =
        sqlx::query_as("SELECT status::text, provider_message_id FROM emails WHERE id = $1")
            .bind(id)
            .fetch_one(&t.state.pool)
            .await
            .unwrap();
    assert_eq!(status, "delivered");
    assert_eq!(pid.as_deref(), Some("ses-1"));
}

#[sqlx::test]
async fn send_event_reconciles_unknown(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    let id = queued(&t).await;
    sqlx::query("UPDATE emails SET status = 'unknown', unknown_since = now() WHERE id = $1")
        .bind(id)
        .execute(&t.state.pool)
        .await
        .unwrap();
    events::ingest(&t.state.pool, &ses("Send", id, json!({})), None)
        .await
        .unwrap();
    assert_eq!(common::status_of(&t.state.pool, id).await, "sent");
    assert_eq!(queue::requeue_unknown(&t.state.pool).await.unwrap(), 0);
}

#[sqlx::test]
async fn fake_sender_simulates_bounce_and_complaint(pool: sqlx::PgPool) {
    let t = common::app(pool).await;
    common::verified_domain(&t, "shop.com").await;
    let b = common::send_email(
        &t,
        json!({ "from": "hi@shop.com", "to": events::SIM_BOUNCE, "subject": "Hi", "text": "x" }),
    )
    .await;
    let c = common::send_email(
        &t,
        json!({ "from": "hi@shop.com", "to": events::SIM_COMPLAINT, "subject": "Hi", "text": "x" }),
    )
    .await;
    let ok = common::send_email(
        &t,
        json!({ "from": "hi@shop.com", "to": events::SIM_SUCCESS, "subject": "Hi", "text": "x" }),
    )
    .await;
    worker::tick(&t.state, &SendLimiter::new(10_000, 1))
        .await
        .unwrap();
    assert_eq!(common::status_of(&t.state.pool, b).await, "bounced");
    assert_eq!(common::status_of(&t.state.pool, c).await, "complained");
    assert_eq!(common::status_of(&t.state.pool, ok).await, "delivered");
}

const CERT_URL: &str = "https://sns.us-east-1.amazonaws.com/test.pem";
const TOPIC: &str = "arn:aws:sns:us-east-1:123456789012:mailrust";

async fn ses_app(pool: sqlx::PgPool) -> (common::TestApp, AppState, rsa::RsaPrivateKey) {
    let t = common::app(pool).await;
    let mut cfg = (*t.state.config).clone();
    cfg.ses = Some(SesConfig {
        region: "us-east-1".into(),
        configuration_set: "mailrust".into(),
        sns_topic_arn: TOPIC.into(),
    });
    let state = AppState::new(t.state.pool.clone(), cfg, t.state.sender.clone());
    let private = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 1024).unwrap();
    state
        .sns_certs
        .insert(CERT_URL, private.to_public_key())
        .await;
    (t, state, private)
}

fn signed(private: &rsa::RsaPrivateKey, topic: &str, message: &Value) -> Value {
    let mut m = sns::SnsMessage {
        kind: "Notification".into(),
        message_id: Uuid::now_v7().to_string(),
        topic_arn: topic.into(),
        message: message.to_string(),
        timestamp: "2026-10-12T00:00:00.000Z".into(),
        signature_version: "2".into(),
        signature: String::new(),
        signing_cert_url: CERT_URL.into(),
        subject: None,
        token: None,
        subscribe_url: None,
    };
    let data = sns::string_to_sign(&m).unwrap();
    let sig = rsa::pkcs1v15::SigningKey::<sha2::Sha256>::new(private.clone()).sign(data.as_bytes());
    m.signature = STANDARD.encode(sig.to_bytes());
    json!({
        "Type": m.kind, "MessageId": m.message_id, "TopicArn": m.topic_arn, "Message": m.message,
        "Timestamp": m.timestamp, "SignatureVersion": m.signature_version, "Signature": m.signature,
        "SigningCertURL": m.signing_cert_url
    })
}

#[sqlx::test]
async fn sns_route_checks_topic_and_signature(pool: sqlx::PgPool) {
    let (t, state, private) = ses_app(pool).await;
    let router = api::router(state);
    let id = queued(&t).await;
    let event = json!({ "eventType": "Delivery", "mail": { "messageId": "ses-9", "tags": { "mailrust_email_id": [id.to_string()] } } });

    let (s, _) = common::call(
        &router,
        common::req(
            "POST",
            "/ses/events",
            None,
            Some(signed(&private, "arn:aws:sns:us-east-1:1:other", &event)),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN, "wrong topic");

    let mut forged = signed(&private, TOPIC, &event);
    forged["Message"] = json!(event.to_string().replace("Delivery", "Complaint"));
    let (s, _) = common::call(
        &router,
        common::req("POST", "/ses/events", None, Some(forged)),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN, "tampered body");

    let (s, _) = common::call(
        &router,
        common::req(
            "POST",
            "/ses/events",
            None,
            Some(signed(&private, TOPIC, &event)),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(common::status_of(&t.state.pool, id).await, "delivered");

    // Fake mode (no SES settings): the route does not exist.
    let (s, _) = common::call(
        &t.router,
        common::req("POST", "/ses/events", None, Some(json!({}))),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}
