use anyhow::Context;
use async_trait::async_trait;
use aws_sdk_sesv2::{
    Client,
    error::{DisplayErrorContext, SdkError},
    operation::{
        create_email_identity::CreateEmailIdentityError, create_tenant::CreateTenantError,
        create_tenant_resource_association::CreateTenantResourceAssociationError,
        send_email::SendEmailError,
    },
    primitives::Blob,
    types::{
        BehaviorOnMxFailure, Destination, DkimStatus, EmailContent, MessageTag, RawMessage,
        SuppressionListReason, SuppressionListScope, TenantSuppressionAttributes,
    },
};

use super::{
    DnsRecord, DomainState, EMAIL_ID_TAG, OutgoingEmail, SendQuota, Sender, SenderError, TenantRef,
};
use crate::config::SesConfig;

pub const SEND_TIMEOUT_SECS: u64 = 60;
const _: () = assert!(SEND_TIMEOUT_SECS < crate::queue::LEASE_SECS as u64);

pub struct SesSender {
    client: Client,
    region: String,
    configuration_set: String,
    aws_account_id: String,
}

fn ctx<E: std::error::Error + 'static, R: std::fmt::Debug>(e: &SdkError<E, R>) -> String {
    DisplayErrorContext(e).to_string()
}

impl SesSender {
    pub async fn new(cfg: &SesConfig) -> anyhow::Result<Self> {
        let shared = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(cfg.region.clone()))
            // Well under the 5-minute queue lease (renewed right before each send), so a hung call can never
            // outlive the lease and let a second worker send the same email.
            .timeout_config(
                aws_config::timeout::TimeoutConfig::builder()
                    .operation_timeout(std::time::Duration::from_secs(SEND_TIMEOUT_SECS))
                    .build(),
            )
            .load()
            .await;
        let identity = aws_sdk_sts::Client::new(&shared)
            .get_caller_identity()
            .send()
            .await
            .context("could not read AWS credentials (sts:GetCallerIdentity failed)")?;
        let aws_account_id = identity
            .account()
            .context("STS returned no account id")?
            .to_string();
        Ok(Self {
            client: Client::new(&shared),
            region: cfg.region.clone(),
            configuration_set: cfg.configuration_set.clone(),
            aws_account_id,
        })
    }

    fn arn(&self, kind: &str, name: &str) -> String {
        format!(
            "arn:aws:ses:{}:{}:{kind}/{name}",
            self.region, self.aws_account_id
        )
    }

    async fn associate(&self, tenant: &str, resource_arn: String) -> Result<(), SenderError> {
        match self
            .client
            .create_tenant_resource_association()
            .tenant_name(tenant)
            .resource_arn(resource_arn)
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(e)
                if matches!(
                    e.as_service_error(),
                    Some(CreateTenantResourceAssociationError::AlreadyExistsException(_))
                ) =>
            {
                Ok(())
            }
            Err(e) => Err(SenderError::Retryable(ctx(&e))),
        }
    }

    async fn dkim_tokens(&self, domain: &str) -> Result<Vec<String>, SenderError> {
        let created = self
            .client
            .create_email_identity()
            .email_identity(domain)
            .configuration_set_name(&self.configuration_set)
            .send()
            .await;
        let attrs = match created {
            Ok(out) => out.dkim_attributes().cloned(),
            Err(e)
                if matches!(
                    e.as_service_error(),
                    Some(CreateEmailIdentityError::AlreadyExistsException(_))
                ) =>
            {
                let got = self
                    .client
                    .get_email_identity()
                    .email_identity(domain)
                    .send()
                    .await
                    .map_err(|e| SenderError::Retryable(ctx(&e)))?;
                got.dkim_attributes().cloned()
            }
            Err(e) => return Err(SenderError::Permanent(ctx(&e))),
        };
        Ok(attrs.map(|a| a.tokens().to_vec()).unwrap_or_default())
    }
}

pub fn dkim_records(domain: &str, tokens: &[String]) -> Vec<DnsRecord> {
    tokens
        .iter()
        .map(|t| DnsRecord {
            record_type: "CNAME".into(),
            name: format!("{t}._domainkey.{domain}"),
            value: format!("{t}.dkim.amazonses.com"),
            purpose: "DKIM (proves the mail is really from this domain)".into(),
        })
        .collect()
}

pub fn mail_from_records(domain: &str, region: &str) -> Vec<DnsRecord> {
    vec![
        DnsRecord {
            record_type: "MX".into(),
            name: format!("bounce.{domain}"),
            value: format!("10 feedback-smtp.{region}.amazonses.com"),
            purpose: "MAIL FROM (bounces come back to SES)".into(),
        },
        DnsRecord {
            record_type: "TXT".into(),
            name: format!("bounce.{domain}"),
            value: "v=spf1 include:amazonses.com ~all".into(),
            purpose: "SPF for the MAIL FROM domain".into(),
        },
    ]
}

pub fn map_send_error(e: &SendEmailError) -> SenderError {
    match e {
        SendEmailError::SendingPausedException(_) => SenderError::TenantPaused,
        SendEmailError::TooManyRequestsException(_)
        | SendEmailError::LimitExceededException(_)
        | SendEmailError::AccountSuspendedException(_) => SenderError::Retryable(e.to_string()),
        SendEmailError::MessageRejected(_)
        | SendEmailError::MailFromDomainNotVerifiedException(_)
        | SendEmailError::BadRequestException(_)
        | SendEmailError::NotFoundException(_) => SenderError::Permanent(e.to_string()),
        _ => SenderError::Retryable(e.to_string()),
    }
}

#[async_trait]
impl Sender for SesSender {
    async fn provision_account(&self, tenant: &TenantRef) -> Result<(), SenderError> {
        let suppression = TenantSuppressionAttributes::builder()
            .suppression_scope(SuppressionListScope::Tenant)
            .suppressed_reasons(SuppressionListReason::Bounce)
            .suppressed_reasons(SuppressionListReason::Complaint)
            .build();
        match self
            .client
            .create_tenant()
            .tenant_name(&tenant.tenant_name)
            .suppression_attributes(suppression)
            .send()
            .await
        {
            Ok(_) => {}
            Err(e)
                if matches!(
                    e.as_service_error(),
                    Some(CreateTenantError::AlreadyExistsException(_))
                ) => {}
            Err(e) => return Err(SenderError::Retryable(ctx(&e))),
        }
        // The tenant gets SES's "Standard" reputation policy by default: auto-pause on high-severity findings.
        self.associate(
            &tenant.tenant_name,
            self.arn("configuration-set", &self.configuration_set),
        )
        .await
    }

    async fn create_domain(
        &self,
        tenant: &TenantRef,
        domain: &str,
    ) -> Result<Vec<DnsRecord>, SenderError> {
        let tokens = self.dkim_tokens(domain).await?;
        self.client
            .put_email_identity_mail_from_attributes()
            .email_identity(domain)
            .mail_from_domain(format!("bounce.{domain}"))
            .behavior_on_mx_failure(BehaviorOnMxFailure::UseDefaultValue)
            .send()
            .await
            .map_err(|e| SenderError::Retryable(ctx(&e)))?;
        self.associate(&tenant.tenant_name, self.arn("identity", domain))
            .await?;
        let mut records = dkim_records(domain, &tokens);
        records.extend(mail_from_records(domain, &self.region));
        Ok(records)
    }

    async fn domain_state(
        &self,
        _tenant: &TenantRef,
        domain: &str,
    ) -> Result<DomainState, SenderError> {
        let out = self
            .client
            .get_email_identity()
            .email_identity(domain)
            .send()
            .await
            .map_err(|e| SenderError::Retryable(ctx(&e)))?;
        if out.verified_for_sending_status() {
            return Ok(DomainState::Verified);
        }
        let failed = out
            .dkim_attributes()
            .and_then(|a| a.status())
            .is_some_and(|s| matches!(s, DkimStatus::Failed));
        Ok(if failed {
            DomainState::Failed
        } else {
            DomainState::Pending
        })
    }

    async fn quota(&self) -> Result<Option<SendQuota>, SenderError> {
        let out = self
            .client
            .get_account()
            .send()
            .await
            .map_err(|e| SenderError::Retryable(ctx(&e)))?;
        Ok(out.send_quota().map(|q| SendQuota {
            max_send_rate: q.max_send_rate(),
            max_24h_send: q.max24_hour_send(),
            sent_last_24h: q.sent_last24_hours(),
        }))
    }

    async fn send(&self, email: &OutgoingEmail) -> Result<String, SenderError> {
        let raw = RawMessage::builder()
            .data(Blob::new(email.raw.clone()))
            .build()
            .map_err(|e| SenderError::Permanent(e.to_string()))?;
        let mut req = self
            .client
            .send_email()
            .from_email_address(&email.from)
            .destination(
                Destination::builder()
                    .set_to_addresses(Some(email.recipients.clone()))
                    .build(),
            )
            .content(EmailContent::builder().raw(raw).build())
            .configuration_set_name(&self.configuration_set)
            .tenant_name(&email.tenant.tenant_name);
        let id_tag = (EMAIL_ID_TAG.to_string(), email.email_id.to_string());
        for (k, v) in email
            .tags
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .chain(std::iter::once(id_tag))
        {
            let tag = MessageTag::builder()
                .name(k)
                .value(v)
                .build()
                .map_err(|e| SenderError::Permanent(e.to_string()))?;
            req = req.email_tags(tag);
        }
        // No SDK retries: a retry after a timeout could reach SES twice. A failed send goes back through
        // Mailrust's own queue instead (retry backoff, or unknown → wait for the SES "Send" event).
        let no_retry = aws_sdk_sesv2::config::Builder::default()
            .retry_config(aws_config::retry::RetryConfig::disabled());
        match req.customize().config_override(no_retry).send().await {
            Ok(out) => Ok(out.message_id().unwrap_or_default().to_string()),
            Err(e) => Err(match &e {
                // The request may have reached SES; sending again could send twice.
                SdkError::TimeoutError(_) | SdkError::ResponseError(_) => {
                    SenderError::Unknown(ctx(&e))
                }
                SdkError::DispatchFailure(d) if d.is_timeout() => SenderError::Unknown(ctx(&e)),
                _ => match e.as_service_error() {
                    Some(se) => map_send_error(se),
                    None => SenderError::Retryable(ctx(&e)),
                },
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_sesv2::types::error::{
        MessageRejected, SendingPausedException, TooManyRequestsException,
    };

    fn test_ses(http: impl aws_sdk_sesv2::config::HttpClient + 'static) -> SesSender {
        let conf = aws_sdk_sesv2::Config::builder()
            .behavior_version(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(aws_sdk_sesv2::config::Credentials::new(
                "a", "b", None, None, "test",
            ))
            .http_client(http)
            .build();
        SesSender {
            client: Client::from_conf(conf),
            region: "us-east-1".into(),
            configuration_set: "cs".into(),
            aws_account_id: "123456789012".into(),
        }
    }

    fn mail() -> OutgoingEmail {
        OutgoingEmail {
            email_id: uuid::Uuid::now_v7(),
            tenant: TenantRef {
                account_id: uuid::Uuid::now_v7(),
                tenant_name: "t".into(),
            },
            from: "a@shop.com".into(),
            recipients: vec!["b@x.com".into()],
            tags: Default::default(),
            raw: b"Subject: hi\r\n\r\nhi".to_vec(),
        }
    }

    #[tokio::test]
    async fn quota_reads_the_ses_account_limits() {
        let http = aws_smithy_http_client::test_util::infallible_client_fn(|_| {
            axum::http::Response::builder()
                .status(200)
                .body(r#"{"SendQuota":{"Max24HourSend":200.0,"MaxSendRate":1.0,"SentLast24Hours":5.0}}"#)
                .unwrap()
        });
        let got = test_ses(http).quota().await.unwrap();
        assert_eq!(
            got,
            Some(SendQuota {
                max_send_rate: 1.0,
                max_24h_send: 200.0,
                sent_last_24h: 5.0
            })
        );
    }

    /// A retry after a timeout could reach SES twice. Mailrust's own unknown → grace → requeue path handles it instead.
    #[tokio::test]
    async fn the_sdk_never_retries_a_send_by_itself() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        let calls = Arc::new(AtomicUsize::new(0));
        let seen = calls.clone();
        let http = aws_smithy_http_client::test_util::infallible_client_fn(move |_| {
            seen.fetch_add(1, Ordering::SeqCst);
            axum::http::Response::builder()
                .status(500)
                .body("")
                .unwrap()
        });
        let ses = test_ses(http);
        let email = mail();
        assert!(ses.send(&email).await.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1, "the SDK retried the send");
    }

    #[test]
    fn dns_records_look_like_ses_expects() {
        let r = dkim_records("shop.com", &["abc".into()]);
        assert_eq!(r[0].name, "abc._domainkey.shop.com");
        assert_eq!(r[0].value, "abc.dkim.amazonses.com");
        let m = mail_from_records("shop.com", "us-east-1");
        assert_eq!(m[0].value, "10 feedback-smtp.us-east-1.amazonses.com");
        assert_eq!(m[1].name, "bounce.shop.com");
    }

    #[test]
    fn send_errors_map_to_the_right_kind() {
        assert!(matches!(
            map_send_error(&SendEmailError::SendingPausedException(
                SendingPausedException::builder().build()
            )),
            SenderError::TenantPaused
        ));
        assert!(matches!(
            map_send_error(&SendEmailError::TooManyRequestsException(
                TooManyRequestsException::builder().build()
            )),
            SenderError::Retryable(_)
        ));
        assert!(matches!(
            map_send_error(&SendEmailError::MessageRejected(
                MessageRejected::builder().build()
            )),
            SenderError::Permanent(_)
        ));
    }
}
