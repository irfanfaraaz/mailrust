use std::path::PathBuf;

use async_trait::async_trait;

use super::{DnsRecord, DomainState, OutgoingEmail, Sender, SenderError, TenantRef};

/// Sends nothing. Writes each email to `<outbox>/<id>.eml` so you can open it in a mail app.
pub struct FakeSender {
    outbox: PathBuf,
}

impl FakeSender {
    pub fn new(outbox: PathBuf) -> Self {
        Self { outbox }
    }
}

#[async_trait]
impl Sender for FakeSender {
    async fn provision_account(&self, _tenant: &TenantRef) -> Result<(), SenderError> {
        Ok(())
    }

    async fn create_domain(
        &self,
        _tenant: &TenantRef,
        domain: &str,
    ) -> Result<Vec<DnsRecord>, SenderError> {
        Ok(vec![DnsRecord {
            record_type: "TXT".into(),
            name: format!("_mailrust.{domain}"),
            value: "fake-sender".into(),
            purpose: "fake sender: nothing to add".into(),
        }])
    }

    async fn domain_state(
        &self,
        _tenant: &TenantRef,
        _domain: &str,
    ) -> Result<DomainState, SenderError> {
        Ok(DomainState::Verified)
    }

    async fn send(&self, email: &OutgoingEmail) -> Result<String, SenderError> {
        tokio::fs::create_dir_all(&self.outbox)
            .await
            .map_err(|e| SenderError::Retryable(e.to_string()))?;
        let path = self.outbox.join(format!("{}.eml", email.email_id));
        tokio::fs::write(&path, &email.raw)
            .await
            .map_err(|e| SenderError::Retryable(e.to_string()))?;
        Ok(format!("fake-{}", email.email_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn tenant() -> TenantRef {
        TenantRef {
            account_id: Uuid::now_v7(),
            tenant_name: "mailrust-test".into(),
        }
    }

    #[tokio::test]
    async fn send_writes_the_raw_message_to_the_outbox() {
        let dir = std::env::temp_dir().join(format!("mailrust-fake-{}", Uuid::now_v7()));
        let sender = FakeSender::new(dir.clone());
        let email = OutgoingEmail {
            email_id: Uuid::now_v7(),
            tenant: tenant(),
            from: "a@shop.com".into(),
            recipients: vec!["b@x.com".into()],
            tags: Default::default(),
            raw: b"Subject: hi\r\n\r\nbody".to_vec(),
        };
        let id = sender.send(&email).await.unwrap();
        assert_eq!(id, format!("fake-{}", email.email_id));
        let written = std::fs::read(dir.join(format!("{}.eml", email.email_id))).unwrap();
        assert_eq!(written, email.raw);
    }

    #[tokio::test]
    async fn domains_are_verified_at_once() {
        let sender = FakeSender::new(std::env::temp_dir());
        assert_eq!(
            sender.domain_state(&tenant(), "shop.com").await.unwrap(),
            DomainState::Verified
        );
        assert_eq!(
            sender
                .create_domain(&tenant(), "shop.com")
                .await
                .unwrap()
                .len(),
            1
        );
    }
}
