//! The worker side of a campaign: one reader's copy, rendered at send time.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use sqlx::types::Json;
use uuid::Uuid;

use super::CampaignStatus;
use crate::{
    api::AppState,
    contacts::{ContactStatus, Fields},
    queue::ClaimedEmail,
    render::{
        blocks::{self, Doc, MergeData},
        campaign::{self, EmailParts, Issue},
    },
    tokens,
};

const CACHE_MAX: usize = 64;

/// What every copy of an issue shares. An issue cannot change once it is sending, so entries never go stale.
pub struct Cached {
    subject: String,
    preview_text: Option<String>,
    newsletter_name: String,
    postal_address: String,
    doc: Doc,
}

#[derive(Default)]
pub struct CampaignCache(Mutex<HashMap<Uuid, Arc<Cached>>>);

impl CampaignCache {
    pub fn get(&self, id: Uuid) -> Option<Arc<Cached>> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&id)
            .cloned()
    }

    pub fn put(&self, id: Uuid, c: Arc<Cached>) {
        let mut m = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if m.len() >= CACHE_MAX {
            m.clear();
        }
        m.insert(id, c);
    }
}

pub enum Prepared {
    Send(EmailParts),
    /// The reader left: mark it suppressed, never send.
    Skip,
    /// The issue was paused or cancelled after this was queued: delete it and free the reader.
    TakeBack,
}

async fn cached(state: &AppState, id: Uuid) -> anyhow::Result<Arc<Cached>> {
    if let Some(c) = state.campaign_cache.get(id) {
        return Ok(c);
    }
    let r = sqlx::query!(
        r#"SELECT c.subject, c.preview_text, c.blocks AS "blocks: Json<serde_json::Value>",
                  COALESCE(a.newsletter_name, a.name) AS "name!", COALESCE(a.postal_address, '') AS "postal!"
           FROM campaigns c JOIN accounts a ON a.id = c.account_id WHERE c.id = $1"#,
        id
    )
    .fetch_one(&state.pool)
    .await?;
    let c = Arc::new(Cached {
        subject: r.subject,
        preview_text: r.preview_text,
        newsletter_name: r.name,
        postal_address: r.postal,
        doc: blocks::parse(&r.blocks.0)?,
    });
    state.campaign_cache.put(id, c.clone());
    Ok(c)
}

pub async fn prepare(state: &AppState, email: &ClaimedEmail) -> anyhow::Result<Prepared> {
    let Some(campaign_id) = email.campaign_id else {
        anyhow::bail!("not a campaign email");
    };
    let row = sqlx::query!(
        r#"SELECT c.status AS "status: CampaignStatus", ct.first_name AS "first_name?", ct.last_name AS "last_name?",
                  ct.email AS "email?", ct.fields AS "fields?: Json<Fields>", ct.status AS "contact_status?: ContactStatus"
           FROM campaigns c LEFT JOIN contacts ct ON ct.id = $2 WHERE c.id = $1"#,
        campaign_id,
        email.contact_id
    )
    .fetch_optional(&state.pool)
    .await?;
    let Some(row) = row else {
        return Ok(Prepared::Skip);
    };
    if row.status != CampaignStatus::Sending {
        return Ok(Prepared::TakeBack);
    }
    let (Some(contact_id), Some(ContactStatus::Subscribed)) =
        (email.contact_id, row.contact_status)
    else {
        return Ok(Prepared::Skip);
    };
    let c = cached(state, campaign_id).await?;
    let data = MergeData {
        first_name: row.first_name,
        last_name: row.last_name,
        email: row.email,
        fields: row.fields.map(|f| f.0).unwrap_or_default(),
    };
    let token =
        tokens::sign_campaign_unsubscribe(&state.config.tracking_secret, contact_id, campaign_id);
    let web = state.config.web_base();
    let url = format!("{web}/u/{token}");
    let issue = Issue {
        subject: &c.subject,
        preview_text: c.preview_text.as_deref(),
        newsletter_name: &c.newsletter_name,
        postal_address: &c.postal_address,
        doc: &c.doc,
    };
    Ok(Prepared::Send(campaign::email(
        &issue,
        &data,
        &web,
        Some(&url),
    )))
}
