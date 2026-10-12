use std::{collections::HashMap, sync::Arc, time::Duration};

use base64::{Engine, engine::general_purpose::STANDARD};
use rsa::{
    RsaPublicKey,
    pkcs1v15::{Signature, VerifyingKey},
    pkcs8::DecodePublicKey,
    signature::Verifier,
};
use serde::Deserialize;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SnsMessage {
    #[serde(rename = "Type")]
    pub kind: String,
    pub message_id: String,
    pub topic_arn: String,
    #[serde(default)]
    pub message: String,
    pub timestamp: String,
    pub signature_version: String,
    pub signature: String,
    #[serde(rename = "SigningCertURL")]
    pub signing_cert_url: String,
    pub subject: Option<String>,
    pub token: Option<String>,
    #[serde(rename = "SubscribeURL")]
    pub subscribe_url: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SnsError {
    #[error("certificate URL is not an AWS SNS URL")]
    BadCertUrl,
    #[error("unknown SNS message type {0}")]
    UnknownType(String),
    #[error("SNS message is missing {0}")]
    MissingField(&'static str),
    #[error("unsupported signature version {0}")]
    BadSignatureVersion(String),
    #[error("signature does not match")]
    BadSignature,
    #[error("could not fetch certificate: {0}")]
    CertFetch(String),
    #[error("could not read certificate: {0}")]
    CertParse(String),
}

/// The exact text AWS signed: chosen keys in a fixed order, each as "Key\nValue\n".
pub fn string_to_sign(m: &SnsMessage) -> Result<String, SnsError> {
    let mut s = String::new();
    let mut push = |k: &str, v: &str| {
        s.push_str(k);
        s.push('\n');
        s.push_str(v);
        s.push('\n');
    };
    match m.kind.as_str() {
        "Notification" => {
            push("Message", &m.message);
            push("MessageId", &m.message_id);
            if let Some(sub) = &m.subject {
                push("Subject", sub);
            }
            push("Timestamp", &m.timestamp);
            push("TopicArn", &m.topic_arn);
            push("Type", &m.kind);
        }
        "SubscriptionConfirmation" | "UnsubscribeConfirmation" => {
            push("Message", &m.message);
            push("MessageId", &m.message_id);
            push(
                "SubscribeURL",
                m.subscribe_url
                    .as_deref()
                    .ok_or(SnsError::MissingField("SubscribeURL"))?,
            );
            push("Timestamp", &m.timestamp);
            push(
                "Token",
                m.token.as_deref().ok_or(SnsError::MissingField("Token"))?,
            );
            push("TopicArn", &m.topic_arn);
            push("Type", &m.kind);
        }
        other => return Err(SnsError::UnknownType(other.to_string())),
    }
    Ok(s)
}

fn is_sns_host(host: &str) -> bool {
    let Some(rest) = host.strip_prefix("sns.") else {
        return false;
    };
    let region = rest
        .strip_suffix(".amazonaws.com")
        .or_else(|| rest.strip_suffix(".amazonaws.com.cn"));
    region.is_some_and(|r| {
        !r.is_empty()
            && r.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    })
}

pub fn check_cert_url(url: &str) -> Result<(), SnsError> {
    let u = url::Url::parse(url).map_err(|_| SnsError::BadCertUrl)?;
    if u.scheme() == "https" && u.host_str().is_some_and(is_sns_host) && u.path().ends_with(".pem")
    {
        Ok(())
    } else {
        Err(SnsError::BadCertUrl)
    }
}

pub fn is_sns_host_url(url: &str) -> bool {
    url::Url::parse(url)
        .is_ok_and(|u| u.scheme() == "https" && u.host_str().is_some_and(is_sns_host))
}

pub fn public_key_from_pem(pem: &[u8]) -> Result<RsaPublicKey, SnsError> {
    let (_, pem) =
        x509_parser::pem::parse_x509_pem(pem).map_err(|e| SnsError::CertParse(e.to_string()))?;
    let cert = pem
        .parse_x509()
        .map_err(|e| SnsError::CertParse(e.to_string()))?;
    RsaPublicKey::from_public_key_der(cert.public_key().raw)
        .map_err(|e| SnsError::CertParse(e.to_string()))
}

pub fn verify_with_key(m: &SnsMessage, key: &RsaPublicKey) -> Result<(), SnsError> {
    let data = string_to_sign(m)?;
    let raw = STANDARD
        .decode(m.signature.as_bytes())
        .map_err(|_| SnsError::BadSignature)?;
    let sig = Signature::try_from(raw.as_slice()).map_err(|_| SnsError::BadSignature)?;
    let ok = match m.signature_version.as_str() {
        "1" => VerifyingKey::<sha1::Sha1>::new(key.clone()).verify(data.as_bytes(), &sig),
        "2" => VerifyingKey::<sha2::Sha256>::new(key.clone()).verify(data.as_bytes(), &sig),
        v => return Err(SnsError::BadSignatureVersion(v.to_string())),
    };
    ok.map_err(|_| SnsError::BadSignature)
}

/// Fetches AWS signing certificates once per URL and keeps their keys in memory.
pub struct CertCache {
    client: reqwest::Client,
    keys: Mutex<HashMap<String, Arc<RsaPublicKey>>>,
}

impl Default for CertCache {
    fn default() -> Self {
        Self::new()
    }
}

impl CertCache {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_default();
        Self {
            client,
            keys: Mutex::new(HashMap::new()),
        }
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Pre-loads a key (used by tests, which cannot fetch real AWS certificates).
    pub async fn insert(&self, url: &str, key: RsaPublicKey) {
        self.keys
            .lock()
            .await
            .insert(url.to_string(), Arc::new(key));
    }

    pub async fn key_for(&self, url: &str) -> Result<Arc<RsaPublicKey>, SnsError> {
        check_cert_url(url)?;
        if let Some(k) = self.keys.lock().await.get(url) {
            return Ok(k.clone());
        }
        let pem = self
            .client
            .get(url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| SnsError::CertFetch(e.to_string()))?
            .bytes()
            .await
            .map_err(|e| SnsError::CertFetch(e.to_string()))?;
        let key = Arc::new(public_key_from_pem(&pem)?);
        self.keys.lock().await.insert(url.to_string(), key.clone());
        Ok(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note() -> SnsMessage {
        SnsMessage {
            kind: "Notification".into(),
            message_id: "m1".into(),
            topic_arn: "arn:aws:sns:us-east-1:1:t".into(),
            message: "{}".into(),
            timestamp: "2026-10-12T00:00:00.000Z".into(),
            signature_version: "2".into(),
            signature: String::new(),
            signing_cert_url: "https://sns.us-east-1.amazonaws.com/x.pem".into(),
            subject: None,
            token: None,
            subscribe_url: None,
        }
    }

    #[test]
    fn notification_string_has_the_aws_order() {
        assert_eq!(
            string_to_sign(&note()).unwrap(),
            "Message\n{}\nMessageId\nm1\nTimestamp\n2026-10-12T00:00:00.000Z\nTopicArn\narn:aws:sns:us-east-1:1:t\nType\nNotification\n"
        );
    }

    #[test]
    fn cert_urls_must_be_aws() {
        assert!(
            check_cert_url("https://sns.us-east-1.amazonaws.com/SimpleNotificationService-abc.pem")
                .is_ok()
        );
        assert!(check_cert_url("http://sns.us-east-1.amazonaws.com/a.pem").is_err());
        assert!(check_cert_url("https://sns.us-east-1.amazonaws.com.evil.com/a.pem").is_err());
        assert!(check_cert_url("https://evil.com/sns.us-east-1.amazonaws.com/a.pem").is_err());
        assert!(check_cert_url("https://sns.us-east-1.amazonaws.com/a.txt").is_err());
        assert!(!is_sns_host_url(
            "https://example.com/?Action=ConfirmSubscription"
        ));
    }

    #[test]
    fn signatures_v1_and_v2_verify_and_tampering_fails() {
        use rsa::signature::{SignatureEncoding, Signer};
        let private = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 1024).unwrap();
        let public = private.to_public_key();
        for version in ["1", "2"] {
            let mut m = note();
            m.signature_version = version.into();
            let data = string_to_sign(&m).unwrap();
            let sig = match version {
                "1" => rsa::pkcs1v15::SigningKey::<sha1::Sha1>::new(private.clone())
                    .sign(data.as_bytes())
                    .to_bytes(),
                _ => rsa::pkcs1v15::SigningKey::<sha2::Sha256>::new(private.clone())
                    .sign(data.as_bytes())
                    .to_bytes(),
            };
            m.signature = STANDARD.encode(sig);
            assert!(verify_with_key(&m, &public).is_ok());
            m.message = "{\"forged\":true}".into();
            assert!(verify_with_key(&m, &public).is_err());
        }
    }
}
