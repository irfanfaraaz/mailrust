use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

const BODY_LEN: usize = 16 + 1 + 2;
const MAC_LEN: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Open,
    Click(u16),
}

fn body(email_id: Uuid, kind: TokenKind) -> [u8; BODY_LEN] {
    let mut b = [0u8; BODY_LEN];
    b[..16].copy_from_slice(email_id.as_bytes());
    let (k, idx) = match kind {
        TokenKind::Open => (0u8, 0u16),
        TokenKind::Click(i) => (1u8, i),
    };
    b[16] = k;
    b[17..19].copy_from_slice(&idx.to_be_bytes());
    b
}

fn mac(secret: &[u8]) -> HmacSha256 {
    // HMAC accepts keys of any length, so this never fails.
    <HmacSha256 as Mac>::new_from_slice(secret).expect("hmac accepts any key length")
}

/// A short code that says "email X, link N", stamped so only we can make it.
pub fn sign(secret: &[u8], email_id: Uuid, kind: TokenKind) -> String {
    let body = body(email_id, kind);
    let mut m = mac(secret);
    m.update(&body);
    let tag = m.finalize().into_bytes();
    let mut out = Vec::with_capacity(BODY_LEN + MAC_LEN);
    out.extend_from_slice(&body);
    out.extend_from_slice(&tag[..MAC_LEN]);
    URL_SAFE_NO_PAD.encode(out)
}

pub fn verify(secret: &[u8], token: &str) -> Option<(Uuid, TokenKind)> {
    let raw = URL_SAFE_NO_PAD.decode(token).ok()?;
    if raw.len() != BODY_LEN + MAC_LEN {
        return None;
    }
    let (body, tag) = raw.split_at(BODY_LEN);
    let mut m = mac(secret);
    m.update(body);
    m.verify_truncated_left(tag).ok()?; // constant-time compare
    let id = Uuid::from_slice(&body[..16]).ok()?;
    let idx = u16::from_be_bytes([body[17], body[18]]);
    let kind = match body[16] {
        0 => TokenKind::Open,
        1 => TokenKind::Click(idx),
        _ => return None,
    };
    Some((id, kind))
}

const UNSUB_PREFIX: &[u8] = b"unsub:";
const UNSUB_LEN: usize = 16 + 32;

/// Never-expiring unsubscribe token: the contact id and a full HMAC over "unsub:" + id.
/// The prefix and the length keep it apart from tracking tokens.
pub fn sign_unsubscribe(secret: &[u8], contact_id: Uuid) -> String {
    let mut m = mac(secret);
    m.update(UNSUB_PREFIX);
    m.update(contact_id.as_bytes());
    let mut out = Vec::with_capacity(UNSUB_LEN);
    out.extend_from_slice(contact_id.as_bytes());
    out.extend_from_slice(&m.finalize().into_bytes());
    URL_SAFE_NO_PAD.encode(out)
}

pub fn verify_unsubscribe(secret: &[u8], token: &str) -> Option<Uuid> {
    let raw = URL_SAFE_NO_PAD.decode(token.trim()).ok()?;
    if raw.len() != UNSUB_LEN {
        return None;
    }
    let (id, tag) = raw.split_at(16);
    let mut m = mac(secret);
    m.update(UNSUB_PREFIX);
    m.update(id);
    m.verify_slice(tag).ok()?; // constant-time compare
    Uuid::from_slice(id).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &[u8] = b"0123456789abcdef0123456789abcdef";

    #[test]
    fn round_trip() {
        let id = Uuid::now_v7();
        for kind in [TokenKind::Open, TokenKind::Click(0), TokenKind::Click(513)] {
            assert_eq!(verify(S, &sign(S, id, kind)), Some((id, kind)));
        }
    }

    #[test]
    fn rejects_tampering_wrong_secret_and_garbage() {
        let t = sign(S, Uuid::now_v7(), TokenKind::Click(1));
        let mut bytes = URL_SAFE_NO_PAD.decode(&t).unwrap();
        bytes[18] ^= 1; // change the link index
        assert_eq!(verify(S, &URL_SAFE_NO_PAD.encode(bytes)), None);
        assert_eq!(verify(b"another-secret-another-secret-xx", &t), None);
        assert_eq!(verify(S, "not-a-token"), None);
        assert_eq!(verify(S, ""), None);
    }

    #[test]
    fn unsubscribe_tokens_round_trip() {
        let id = Uuid::now_v7();
        let t = sign_unsubscribe(S, id);
        assert_eq!(t.len(), 64, "48 bytes as base64url");
        assert_eq!(verify_unsubscribe(S, &t), Some(id));
    }

    #[test]
    fn unsubscribe_tokens_reject_tampering() {
        let id = Uuid::now_v7();
        let t = sign_unsubscribe(S, id);
        let raw = URL_SAFE_NO_PAD.decode(&t).unwrap();

        let mut flipped = raw.clone();
        flipped[20] ^= 1;
        assert_eq!(verify_unsubscribe(S, &URL_SAFE_NO_PAD.encode(&flipped)), None);

        let mut other_id = raw.clone();
        other_id[..16].copy_from_slice(Uuid::now_v7().as_bytes());
        assert_eq!(verify_unsubscribe(S, &URL_SAFE_NO_PAD.encode(&other_id)), None, "old MAC on another id");

        assert_eq!(verify_unsubscribe(S, &t[..t.len() - 2]), None, "truncated");
        assert_eq!(verify_unsubscribe(b"another-secret-another-secret-xx", &t), None);
        assert_eq!(verify_unsubscribe(S, &sign(S, id, TokenKind::Open)), None, "a tracking token is not an unsubscribe token");
        assert_eq!(verify(S, &t), None, "and the other way round");
        assert_eq!(verify_unsubscribe(S, "not a token"), None);
    }
}
