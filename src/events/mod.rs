use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::{
    email_events::{self, EventKind, NewEvent},
    emails::EmailStatus,
    safety::{self, SuppressionReason},
    sender::EMAIL_ID_TAG,
};

pub mod sns;

pub const SIM_SUCCESS: &str = "success@simulator.amazonses.com";
pub const SIM_BOUNCE: &str = "bounce@simulator.amazonses.com";
pub const SIM_COMPLAINT: &str = "complaint@simulator.amazonses.com";
pub const SIM_SUPPRESSED: &str = "suppressionlist@simulator.amazonses.com";

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SesEvent {
    pub event_type: String,
    pub mail: SesMail,
    pub bounce: Option<SesBounce>,
    pub complaint: Option<SesComplaint>,
    pub delivery: Option<SesDelivery>,
    pub delivery_delay: Option<SesDelay>,
    pub reject: Option<SesReject>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SesMail {
    pub message_id: String,
    #[serde(default)]
    pub destination: Vec<String>,
    #[serde(default)]
    pub tags: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SesRecipient {
    pub email_address: String,
    pub diagnostic_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SesBounce {
    pub bounce_type: String,
    pub bounced_recipients: Vec<SesRecipient>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SesComplaint {
    pub complained_recipients: Vec<SesRecipient>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SesDelivery {
    #[serde(default)]
    pub recipients: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SesDelay {
    pub delayed_recipients: Vec<SesRecipient>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SesReject {
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Applied {
    Unmatched,
    Duplicate,
    Done {
        account_id: Uuid,
        affects_safety: bool,
    },
}

struct Target {
    id: Uuid,
    account_id: Uuid,
    status: EmailStatus,
}

async fn find_target(conn: &mut PgConnection, ev: &SesEvent) -> sqlx::Result<Option<Target>> {
    let tagged = ev
        .mail
        .tags
        .get(EMAIL_ID_TAG)
        .and_then(|v| v.first())
        .and_then(|s| s.parse::<Uuid>().ok());
    match tagged {
        Some(id) => {
            sqlx::query_as!(
                Target,
                r#"SELECT id, account_id, status AS "status: EmailStatus" FROM emails WHERE id = $1 FOR UPDATE"#,
                id
            )
            .fetch_optional(&mut *conn)
            .await
        }
        None => {
            sqlx::query_as!(
                Target,
                r#"SELECT id, account_id, status AS "status: EmailStatus" FROM emails WHERE provider_message_id = $1 FOR UPDATE"#,
                ev.mail.message_id
            )
            .fetch_optional(&mut *conn)
            .await
        }
    }
}

async fn raise(conn: &mut PgConnection, t: &Target, new: EmailStatus) -> sqlx::Result<()> {
    if new.rank() > t.status.rank() {
        sqlx::query!(
            "UPDATE emails SET status = $2, updated_at = now() WHERE id = $1",
            t.id,
            new as EmailStatus
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

async fn rec(
    conn: &mut PgConnection,
    t: &Target,
    kind: EventKind,
    recipient: Option<&str>,
    data: serde_json::Value,
) -> sqlx::Result<()> {
    email_events::record(
        conn,
        NewEvent {
            account_id: t.account_id,
            email_id: t.id,
            kind,
            recipient,
            data,
            likely_bot: false,
        },
    )
    .await?;
    Ok(())
}

/// Applies one SES event inside the caller's transaction.
pub async fn apply(conn: &mut PgConnection, ev: &SesEvent) -> anyhow::Result<Applied> {
    let Some(t) = find_target(conn, ev).await? else {
        return Ok(Applied::Unmatched);
    };
    // The event can arrive before the worker saved the SES id; keep the first one we learn.
    sqlx::query!(
        "UPDATE emails SET provider_message_id = COALESCE(provider_message_id, $2) WHERE id = $1",
        t.id,
        ev.mail.message_id
    )
    .execute(&mut *conn)
    .await?;

    let mut affects_safety = false;
    match ev.event_type.as_str() {
        "Send" => {
            // Only matters for crash recovery: SES did get it, so do not send again
            // (also when the grace period already put it back in the queue, or another worker re-claimed it:
            // that worker's renew_lease then fails and it skips the send).
            if matches!(
                t.status,
                EmailStatus::Unknown | EmailStatus::Queued | EmailStatus::Sending
            ) {
                sqlx::query!(
                    "UPDATE emails SET status = 'sent', sent_at = COALESCE(sent_at, now()), unknown_since = NULL, updated_at = now()
                     WHERE id = $1",
                    t.id
                )
                .execute(&mut *conn)
                .await?;
                rec(
                    conn,
                    &t,
                    EventKind::Sent,
                    None,
                    json!({ "recipients": ev.mail.destination, "reconciled": true }),
                )
                .await?;
            }
        }
        "Delivery" => {
            raise(conn, &t, EmailStatus::Delivered).await?;
            let list = ev
                .delivery
                .as_ref()
                .map(|d| d.recipients.clone())
                .filter(|r| !r.is_empty())
                .unwrap_or_else(|| ev.mail.destination.clone());
            for r in &list {
                rec(conn, &t, EventKind::Delivered, Some(r), json!({})).await?;
            }
        }
        "DeliveryDelay" => {
            raise(conn, &t, EmailStatus::DeliveryDelayed).await?;
            for r in ev.delivery_delay.iter().flat_map(|d| &d.delayed_recipients) {
                rec(
                    conn,
                    &t,
                    EventKind::DeliveryDelayed,
                    Some(&r.email_address),
                    json!({}),
                )
                .await?;
            }
        }
        "Bounce" => {
            let Some(b) = &ev.bounce else {
                return Ok(Applied::Done {
                    account_id: t.account_id,
                    affects_safety,
                });
            };
            if b.bounce_type == "Permanent" {
                raise(conn, &t, EmailStatus::Bounced).await?;
                for r in &b.bounced_recipients {
                    rec(
                        conn,
                        &t,
                        EventKind::Bounced,
                        Some(&r.email_address),
                        json!({ "diagnostic_code": r.diagnostic_code }),
                    )
                    .await?;
                    safety::add_suppression(
                        conn,
                        t.account_id,
                        &r.email_address,
                        SuppressionReason::Bounce,
                        Some(t.id),
                    )
                    .await?;
                }
                affects_safety = true;
            } else {
                for r in &b.bounced_recipients {
                    rec(
                        conn,
                        &t,
                        EventKind::SoftBounced,
                        Some(&r.email_address),
                        json!({ "bounce_type": b.bounce_type }),
                    )
                    .await?;
                }
            }
        }
        "Complaint" => {
            raise(conn, &t, EmailStatus::Complained).await?;
            for r in ev.complaint.iter().flat_map(|c| &c.complained_recipients) {
                rec(
                    conn,
                    &t,
                    EventKind::Complained,
                    Some(&r.email_address),
                    json!({}),
                )
                .await?;
                safety::add_suppression(
                    conn,
                    t.account_id,
                    &r.email_address,
                    SuppressionReason::Complaint,
                    Some(t.id),
                )
                .await?;
            }
            affects_safety = true;
        }
        "Reject" => {
            raise(conn, &t, EmailStatus::Failed).await?;
            let reason = ev
                .reject
                .as_ref()
                .map(|r| r.reason.clone())
                .unwrap_or_default();
            rec(
                conn,
                &t,
                EventKind::Failed,
                None,
                json!({ "reason": reason }),
            )
            .await?;
        }
        other => tracing::debug!(event_type = other, "ignored SES event"),
    }
    Ok(Applied::Done {
        account_id: t.account_id,
        affects_safety,
    })
}

/// One transaction: SNS de-duplication + apply. Real SNS events and fake-sender simulations both come through here.
pub async fn ingest(
    pool: &PgPool,
    ev: &SesEvent,
    sns_message_id: Option<&str>,
) -> anyhow::Result<Applied> {
    let mut tx = pool.begin().await?;
    if let Some(mid) = sns_message_id {
        let fresh = sqlx::query!(
            "INSERT INTO sns_messages (message_id) VALUES ($1) ON CONFLICT DO NOTHING",
            mid
        )
        .execute(&mut *tx)
        .await?
        .rows_affected()
            == 1;
        if !fresh {
            return Ok(Applied::Duplicate);
        }
    }
    let applied = apply(&mut tx, ev).await?;
    tx.commit().await?;
    after_ingest(pool, &applied).await;
    Ok(applied)
}

async fn after_ingest(pool: &PgPool, applied: &Applied) {
    if let Applied::Done {
        account_id,
        affects_safety: true,
    } = applied
        && let Err(e) = safety::check_account(pool, *account_id).await
    {
        tracing::error!(error = ?e, "auto-pause check failed");
    }
}

fn sim_event(kind: &str, mail: &SesMail) -> SesEvent {
    SesEvent {
        event_type: kind.into(),
        mail: mail.clone(),
        ..Default::default()
    }
}

fn recips(list: &[String]) -> Vec<SesRecipient> {
    list.iter()
        .map(|a| SesRecipient {
            email_address: a.clone(),
            diagnostic_code: None,
        })
        .collect()
}

/// Fake sender only: acts like SES's mailbox simulator, so the full loop runs locally.
pub async fn simulate(
    pool: &PgPool,
    email_id: Uuid,
    provider_id: &str,
    recipients: &[String],
) -> anyhow::Result<()> {
    let mail = SesMail {
        message_id: provider_id.to_string(),
        destination: recipients.to_vec(),
        tags: HashMap::from([(EMAIL_ID_TAG.to_string(), vec![email_id.to_string()])]),
    };
    let (mut delivered, mut bounced, mut complained) = (Vec::new(), Vec::new(), Vec::new());
    for r in recipients {
        match r.as_str() {
            SIM_BOUNCE | SIM_SUPPRESSED => bounced.push(r.clone()),
            SIM_COMPLAINT => {
                delivered.push(r.clone());
                complained.push(r.clone());
            }
            _ => delivered.push(r.clone()),
        }
    }
    if !delivered.is_empty() {
        let mut ev = sim_event("Delivery", &mail);
        ev.delivery = Some(SesDelivery {
            recipients: delivered,
        });
        ingest(pool, &ev, None).await?;
    }
    if !bounced.is_empty() {
        let mut ev = sim_event("Bounce", &mail);
        ev.bounce = Some(SesBounce {
            bounce_type: "Permanent".into(),
            bounced_recipients: recips(&bounced),
        });
        ingest(pool, &ev, None).await?;
    }
    if !complained.is_empty() {
        let mut ev = sim_event("Complaint", &mail);
        ev.complaint = Some(SesComplaint {
            complained_recipients: recips(&complained),
        });
        ingest(pool, &ev, None).await?;
    }
    Ok(())
}
