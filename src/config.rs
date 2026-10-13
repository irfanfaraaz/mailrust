use std::{collections::HashMap, net::SocketAddr, path::PathBuf};

use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SenderKind {
    Fake,
    Ses,
}

#[derive(Debug, Clone)]
pub struct SesConfig {
    pub region: String,
    pub configuration_set: String,
    pub sns_topic_arn: String,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: SocketAddr,
    pub public_url: url::Url,
    pub sender: SenderKind,
    pub ses: Option<SesConfig>,
    pub tracking_secret: Vec<u8>,
    pub ses_max_send_rate: u32,
    pub worker_processes: u32,
    /// Emails one worker claims per round.
    pub worker_batch: u32,
    /// Emails one worker sends at the same time.
    pub worker_concurrency: u32,
    pub allow_private_webhooks: bool,
    pub outbox_dir: PathBuf,
    pub log_json: bool,
    /// Base URL of the dashboard, used in login and invite links.
    pub web_url: url::Url,
    /// Account and address that send login and invite emails.
    pub system_account_id: Option<Uuid>,
    pub system_from: Option<String>,
    /// Shared with the dashboard; only requests carrying it may pass the client IP.
    pub web_secret: Option<String>,
    /// Dev only: also print login and invite links to the log.
    pub dev_log_links: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing setting {0}")]
    Missing(&'static str),
    #[error("bad value for {0}: {1}")]
    Invalid(&'static str, String),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_map(&std::env::vars().collect())
    }

    pub fn from_map(env: &HashMap<String, String>) -> Result<Self, ConfigError> {
        let get = |k: &str| {
            env.get(k)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };
        let req = |k: &'static str| get(k).ok_or(ConfigError::Missing(k));
        let num = |k: &'static str, default: u32| -> Result<u32, ConfigError> {
            match get(k) {
                None => Ok(default),
                Some(v) => match v.parse::<u32>() {
                    Ok(n) if n > 0 => Ok(n),
                    _ => Err(ConfigError::Invalid(
                        k,
                        format!("{v} is not a number above 0"),
                    )),
                },
            }
        };

        let sender = match get("MAILRUST_SENDER").as_deref().unwrap_or("fake") {
            "fake" => SenderKind::Fake,
            "ses" => SenderKind::Ses,
            other => {
                return Err(ConfigError::Invalid(
                    "MAILRUST_SENDER",
                    format!("{other} (use fake or ses)"),
                ));
            }
        };
        let ses = match sender {
            SenderKind::Ses => Some(SesConfig {
                region: req("AWS_REGION")?,
                configuration_set: req("SES_CONFIGURATION_SET")?,
                sns_topic_arn: req("SNS_TOPIC_ARN")?,
            }),
            SenderKind::Fake => None,
        };

        let tracking_secret = req("TRACKING_SECRET")?.into_bytes();
        if tracking_secret.len() < 32 {
            return Err(ConfigError::Invalid(
                "TRACKING_SECRET",
                "must be at least 32 characters".into(),
            ));
        }

        let public_url_raw = req("PUBLIC_URL")?;
        let public_url = url::Url::parse(&public_url_raw)
            .map_err(|e| ConfigError::Invalid("PUBLIC_URL", e.to_string()))?;
        if public_url.host_str().is_none() {
            return Err(ConfigError::Invalid("PUBLIC_URL", "needs a host".into()));
        }

        let bind_raw = get("BIND_ADDR").unwrap_or_else(|| "0.0.0.0:8080".into());
        let bind_addr = bind_raw
            .parse()
            .map_err(|_| ConfigError::Invalid("BIND_ADDR", bind_raw.clone()))?;

        let web_raw = get("WEB_URL").unwrap_or_else(|| "http://localhost:3000".into());
        let web_url = url::Url::parse(&web_raw)
            .map_err(|e| ConfigError::Invalid("WEB_URL", e.to_string()))?;
        let system_account_id = match get("SYSTEM_ACCOUNT_ID") {
            None => None,
            Some(v) => Some(
                v.parse::<Uuid>()
                    .map_err(|_| ConfigError::Invalid("SYSTEM_ACCOUNT_ID", v.clone()))?,
            ),
        };
        let web_secret = get("WEB_SECRET");
        if web_secret.as_ref().is_some_and(|s| s.len() < 32) {
            return Err(ConfigError::Invalid(
                "WEB_SECRET",
                "must be at least 32 characters".into(),
            ));
        }

        Ok(Config {
            database_url: req("DATABASE_URL")?,
            bind_addr,
            public_url,
            sender,
            ses,
            tracking_secret,
            ses_max_send_rate: num("SES_MAX_SEND_RATE", 14)?,
            worker_processes: num("WORKER_PROCESSES", 1)?,
            worker_batch: num("WORKER_BATCH", 50)?,
            worker_concurrency: num("WORKER_CONCURRENCY", 10)?,
            allow_private_webhooks: get("MAILRUST_ALLOW_HTTP_WEBHOOKS").as_deref() == Some("true"),
            outbox_dir: PathBuf::from(get("OUTBOX_DIR").unwrap_or_else(|| "./outbox".into())),
            log_json: get("RUST_LOG_FORMAT").as_deref() == Some("json"),
            web_url,
            system_account_id,
            system_from: get("SYSTEM_FROM"),
            web_secret,
            dev_log_links: get("DEV_LOG_LINKS").as_deref() == Some("true"),
        })
    }

    pub fn public_base(&self) -> String {
        self.public_url.as_str().trim_end_matches('/').to_string()
    }

    pub fn web_base(&self) -> String {
        self.web_url.as_str().trim_end_matches('/').to_string()
    }

    pub fn public_host(&self) -> String {
        self.public_url.host_str().unwrap_or_default().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> HashMap<String, String> {
        [
            ("DATABASE_URL", "postgres://x"),
            ("PUBLIC_URL", "https://mail.example.com/"),
            ("TRACKING_SECRET", "0123456789abcdef0123456789abcdef"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
    }

    #[test]
    fn fake_sender_needs_no_aws_settings() {
        let c = Config::from_map(&base()).unwrap();
        assert_eq!(c.sender, SenderKind::Fake);
        assert!(c.ses.is_none());
        assert_eq!(c.public_base(), "https://mail.example.com");
        assert_eq!(c.public_host(), "mail.example.com");
        assert_eq!(c.ses_max_send_rate, 14);
        assert_eq!(c.bind_addr.port(), 8080);
    }

    #[test]
    fn a_set_rate_is_used_and_private_webhooks_need_true() {
        let c = Config::from_map(&base()).unwrap();
        assert!(
            !c.allow_private_webhooks,
            "private webhooks must be off by default"
        );
        let mut env = base();
        env.insert("SES_MAX_SEND_RATE".into(), "20".into());
        env.insert("MAILRUST_ALLOW_HTTP_WEBHOOKS".into(), "true".into());
        let c = Config::from_map(&env).unwrap();
        assert_eq!(c.ses_max_send_rate, 20);
        assert!(c.allow_private_webhooks);
    }

    #[test]
    fn worker_batch_and_concurrency_default_and_can_be_set() {
        let c = Config::from_map(&base()).unwrap();
        assert_eq!((c.worker_batch, c.worker_concurrency), (50, 10));
        let mut env = base();
        env.insert("WORKER_BATCH".into(), "200".into());
        env.insert("WORKER_CONCURRENCY".into(), "40".into());
        let c = Config::from_map(&env).unwrap();
        assert_eq!((c.worker_batch, c.worker_concurrency), (200, 40));
        for k in ["WORKER_BATCH", "WORKER_CONCURRENCY"] {
            let mut env = base();
            env.insert(k.into(), "0".into());
            assert!(matches!(Config::from_map(&env), Err(ConfigError::Invalid(n, _)) if n == k));
        }
    }

    #[test]
    fn ses_sender_requires_region() {
        let mut env = base();
        env.insert("MAILRUST_SENDER".into(), "ses".into());
        assert!(matches!(
            Config::from_map(&env),
            Err(ConfigError::Missing("AWS_REGION"))
        ));
    }

    #[test]
    fn short_tracking_secret_is_rejected() {
        let mut env = base();
        env.insert("TRACKING_SECRET".into(), "short".into());
        assert!(matches!(
            Config::from_map(&env),
            Err(ConfigError::Invalid("TRACKING_SECRET", _))
        ));
    }

    #[test]
    fn zero_rate_is_rejected() {
        let mut env = base();
        env.insert("SES_MAX_SEND_RATE".into(), "0".into());
        assert!(matches!(
            Config::from_map(&env),
            Err(ConfigError::Invalid("SES_MAX_SEND_RATE", _))
        ));
    }

    #[test]
    fn web_settings_default_and_parse() {
        let c = Config::from_map(&base()).unwrap();
        assert_eq!(c.web_base(), "http://localhost:3000");
        assert_eq!(c.system_account_id, None);
        assert!(!c.dev_log_links);

        let mut env = base();
        env.insert("WEB_URL".into(), "https://app.shop.com/".into());
        env.insert(
            "SYSTEM_ACCOUNT_ID".into(),
            "0192a000-0000-7000-8000-000000000001".into(),
        );
        env.insert(
            "SYSTEM_FROM".into(),
            "Mailrust <login@mail.shop.com>".into(),
        );
        env.insert("WEB_SECRET".into(), "s".repeat(32));
        env.insert("DEV_LOG_LINKS".into(), "true".into());
        let c = Config::from_map(&env).unwrap();
        assert_eq!(c.web_base(), "https://app.shop.com");
        assert!(c.system_account_id.is_some());
        assert_eq!(
            c.system_from.as_deref(),
            Some("Mailrust <login@mail.shop.com>")
        );
        assert!(c.dev_log_links);
    }

    #[test]
    fn short_web_secret_and_bad_account_id_are_rejected() {
        let mut env = base();
        env.insert("WEB_SECRET".into(), "short".into());
        assert!(Config::from_map(&env).is_err());
        let mut env = base();
        env.insert("SYSTEM_ACCOUNT_ID".into(), "nope".into());
        assert!(Config::from_map(&env).is_err());
    }
}
