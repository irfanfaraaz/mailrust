use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::{Config, SenderKind};

pub mod fake;

pub const EMAIL_ID_TAG: &str = "mailrust_email_id";

#[derive(Debug, Clone)]
pub struct TenantRef {
    pub account_id: Uuid,
    pub tenant_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DnsRecord {
    pub record_type: String,
    pub name: String,
    pub value: String,
    pub purpose: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainState {
    Pending,
    Verified,
    Failed,
}

#[derive(Debug, Clone)]
pub struct OutgoingEmail {
    pub email_id: Uuid,
    pub tenant: TenantRef,
    pub from: String,
    pub recipients: Vec<String>,
    pub tags: BTreeMap<String, String>,
    pub raw: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum SenderError {
    #[error("temporary sender error: {0}")]
    Retryable(String),
    #[error("permanent sender error: {0}")]
    Permanent(String),
    #[error("the sender paused this tenant")]
    TenantPaused,
    #[error("sender is not configured: {0}")]
    NotConfigured(String),
}

/// One place to plug in a delivery service (SES today, SMTP later).
#[async_trait]
pub trait Sender: Send + Sync {
    async fn provision_account(&self, tenant: &TenantRef) -> Result<(), SenderError>;
    async fn create_domain(
        &self,
        tenant: &TenantRef,
        domain: &str,
    ) -> Result<Vec<DnsRecord>, SenderError>;
    async fn domain_state(
        &self,
        tenant: &TenantRef,
        domain: &str,
    ) -> Result<DomainState, SenderError>;
    async fn send(&self, email: &OutgoingEmail) -> Result<String, SenderError>;
}

pub async fn from_config(config: &Config) -> anyhow::Result<Arc<dyn Sender>> {
    match config.sender {
        SenderKind::Fake => Ok(Arc::new(fake::FakeSender::new(config.outbox_dir.clone()))),
        // Task 14 replaces this arm with the real SesSender.
        SenderKind::Ses => anyhow::bail!("MAILRUST_SENDER=ses is not available in this build yet"),
    }
}
