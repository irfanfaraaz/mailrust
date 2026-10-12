use base64::{Engine, engine::general_purpose::STANDARD};
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;

pub fn new_secret() -> String {
    let mut b = [0u8; 24];
    rand::rngs::OsRng.fill_bytes(&mut b);
    format!("whsec_{}", STANDARD.encode(b))
}

/// Standard Webhooks: base64(HMAC-SHA256(secret, "{id}.{timestamp}.{body}")), sent as "v1,<sig>".
pub fn signature(
    secret: &str,
    msg_id: &str,
    timestamp: i64,
    body: &[u8],
) -> Result<String, String> {
    let key = STANDARD
        .decode(secret.strip_prefix("whsec_").unwrap_or(secret))
        .map_err(|_| "webhook secret is not valid base64".to_string())?;
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&key).map_err(|e| e.to_string())?;
    mac.update(format!("{msg_id}.{timestamp}.").as_bytes());
    mac.update(body);
    Ok(format!(
        "v1,{}",
        STANDARD.encode(mac.finalize().into_bytes())
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    // The worked example from the Standard Webhooks spec. If this fails, re-check the
    // example at standardwebhooks.com before touching the code.
    #[test]
    fn matches_the_spec_example() {
        let sig = signature(
            "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw",
            "msg_p5jXN8AQM9LWM0D4loKWxJek",
            1614265330,
            br#"{"test": 2432232314}"#,
        )
        .unwrap();
        assert_eq!(sig, "v1,g0hM9SsE+OTPJTGt/tmIKtSyZlE3uFJELVlNIOLJ1OE=");
    }

    #[test]
    fn new_secrets_are_valid_and_different() {
        let a = new_secret();
        assert!(a.starts_with("whsec_"));
        assert_ne!(a, new_secret());
        assert!(signature(&a, "m", 1, b"{}").is_ok());
    }
}
