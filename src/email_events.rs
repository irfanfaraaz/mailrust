use serde::Serialize;
use sqlx::PgConnection;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EventKind {
    Sent,
    Delivered,
    DeliveryDelayed,
    Bounced,
    SoftBounced,
    Complained,
    Opened,
    Clicked,
    Failed,
    Suppressed,
}

impl EventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EventKind::Sent => "sent",
            EventKind::Delivered => "delivered",
            EventKind::DeliveryDelayed => "delivery_delayed",
            EventKind::Bounced => "bounced",
            EventKind::SoftBounced => "soft_bounced",
            EventKind::Complained => "complained",
            EventKind::Opened => "opened",
            EventKind::Clicked => "clicked",
            EventKind::Failed => "failed",
            EventKind::Suppressed => "suppressed",
        }
    }

    pub fn webhook_type(self) -> Option<&'static str> {
        Some(match self {
            EventKind::Sent => "email.sent",
            EventKind::Delivered => "email.delivered",
            EventKind::DeliveryDelayed => "email.delivery_delayed",
            EventKind::Bounced => "email.bounced",
            EventKind::Complained => "email.complained",
            EventKind::Opened => "email.opened",
            EventKind::Clicked => "email.clicked",
            EventKind::Failed => "email.failed",
            EventKind::Suppressed => "email.suppressed",
            EventKind::SoftBounced => return None,
        })
    }
}

pub struct NewEvent<'a> {
    pub account_id: Uuid,
    pub email_id: Uuid,
    pub kind: EventKind,
    pub recipient: Option<&'a str>,
    pub data: serde_json::Value,
    pub likely_bot: bool,
}

/// The single place every email event is written. Task 11 also queues webhooks here.
pub async fn record(conn: &mut PgConnection, ev: NewEvent<'_>) -> sqlx::Result<Uuid> {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO email_events (id, account_id, email_id, type, recipient, data, likely_bot)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        id,
        ev.account_id,
        ev.email_id,
        ev.kind.as_str(),
        ev.recipient,
        ev.data,
        ev.likely_bot
    )
    .execute(&mut *conn)
    .await?;
    Ok(id)
}
