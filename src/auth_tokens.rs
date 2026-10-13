//! One-time and session tokens for the dashboard. Stored only as `keys::hash_token`.
use base64::Engine;
use rand::RngCore;

pub const SESSION_PREFIX: &str = "mr_sess_";
pub const LOGIN_PREFIX: &str = "mr_login_";
pub const INVITE_PREFIX: &str = "mr_inv_";

/// 32 random bytes, base64url without padding (43 chars), after the prefix.
pub fn generate(prefix: &str) -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    format!(
        "{prefix}{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_carry_their_prefix_and_differ() {
        let a = generate(SESSION_PREFIX);
        let b = generate(SESSION_PREFIX);
        assert!(a.starts_with("mr_sess_"));
        assert_eq!(a.len(), SESSION_PREFIX.len() + 43);
        assert_ne!(a, b);
    }
}
