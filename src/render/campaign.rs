//! A campaign as a whole email (template, footer, List-Unsubscribe headers) and as an archive page.
use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::{
    RenderError, Tracking,
    blocks::{self, Ctx, Doc, MergeData, esc},
};

/// Gmail clips messages over ~102 KB of HTML.
pub const MAX_EMAIL_HTML: usize = 102_400;
/// The archive page runs no script and loads images only from the app itself.
pub const ARCHIVE_CSP: &str = "default-src 'none'; img-src 'self'; style-src 'unsafe-inline'";
const FONT: &str = "-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif";

pub struct Issue<'a> {
    pub subject: &'a str,
    pub preview_text: Option<&'a str>,
    pub newsletter_name: &'a str,
    pub postal_address: &'a str,
    pub doc: &'a Doc,
}

pub struct EmailParts {
    pub html: String,
    pub text: String,
    pub headers: BTreeMap<String, String>,
}

fn lines_html(s: &str) -> String {
    esc(s.trim()).replace('\n', "<br>")
}

/// `unsubscribe_url`: None for a test email (no List-Unsubscribe headers; the footer says it is a test).
pub fn email(
    issue: &Issue,
    data: &MergeData,
    web_base: &str,
    unsubscribe_url: Option<&str>,
) -> EmailParts {
    let image_base = format!("{web_base}/img");
    let cx = Ctx {
        data,
        image_base: &image_base,
    };
    let body = blocks::html(issue.doc, &cx);
    let name = esc(issue.newsletter_name);
    let unsubscribe = match unsubscribe_url {
        Some(u) => format!(
            r#"<a data-mr-skip href="{}" style="color:#71717a;text-decoration:underline">Unsubscribe</a>"#,
            esc(u)
        ),
        None => "This is a test email: readers get an unsubscribe link here.".to_string(),
    };
    let preview = issue.preview_text.map(esc).unwrap_or_default();
    let html = format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="color-scheme" content="light"><title>{subject}</title></head><body style="margin:0;padding:0;background:#f4f4f5"><div style="display:none;max-height:0;overflow:hidden;opacity:0">{preview}</div><table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="background:#f4f4f5"><tr><td align="center" style="padding:24px 12px"><table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="max-width:600px;background:#ffffff;border-radius:8px"><tr><td style="padding:32px 28px;font-family:{FONT};color:#18181b">{body}</td></tr></table><p style="max-width:600px;margin:16px auto 0;font-family:{FONT};font-size:12px;line-height:1.5;color:#71717a">You get this because you subscribed to {name}.<br>{postal}<br>{unsubscribe}</p></td></tr></table></body></html>"#,
        subject = esc(issue.subject),
        postal = lines_html(issue.postal_address),
    );
    let footer_link = match unsubscribe_url {
        Some(u) => format!("Unsubscribe: {u}"),
        None => "This is a test email: readers get an unsubscribe link here.".into(),
    };
    let text = format!(
        "{}\n--\nYou get this because you subscribed to {}.\n{}\n{footer_link}\n",
        blocks::text(issue.doc, &cx),
        issue.newsletter_name,
        issue.postal_address.trim()
    );
    let mut headers = BTreeMap::new();
    if let Some(u) = unsubscribe_url {
        headers.insert("List-Unsubscribe".to_string(), format!("<{u}>"));
        headers.insert(
            "List-Unsubscribe-Post".to_string(),
            "List-Unsubscribe=One-Click".to_string(),
        );
    }
    EmailParts {
        html,
        text,
        headers,
    }
}

/// The worst case a reader can get: 100-char names, a 254-char address, 500-char fields.
fn long_merge_data(doc: &Doc) -> MergeData {
    let mut d = MergeData {
        first_name: Some("x".repeat(100)),
        last_name: Some("x".repeat(100)),
        email: Some(format!("{}@{}.com", "x".repeat(64), "y".repeat(185))),
        fields: BTreeMap::new(),
    };
    for k in blocks::merge_keys(doc) {
        if !matches!(k.as_str(), "first_name" | "last_name" | "email") {
            d.fields.insert(k, "x".repeat(500));
        }
    }
    d
}

/// HTML bytes of the largest copy any reader can get: footer, opens + clicks tracking, long merge values.
pub fn email_size(issue: &Issue, web_base: &str, secret: &[u8]) -> Result<usize, RenderError> {
    let unsubscribe = format!("{web_base}/u/{}", "x".repeat(86));
    let parts = email(
        issue,
        &long_merge_data(issue.doc),
        web_base,
        Some(&unsubscribe),
    );
    let t = Tracking {
        base_url: web_base.to_string(),
        secret,
        opens: true,
        clicks: true,
    };
    let (html, _) = super::tracking::apply(&parts.html, Uuid::nil(), &t)?;
    Ok(html.len())
}

/// The public archive copy: merge tags show fallbacks, images load from this site, no pixel, no unsubscribe.
pub fn archive_page(
    issue: &Issue,
    sent_at: DateTime<Utc>,
    newsletter_slug: &str,
    signup_open: bool,
) -> String {
    let data = MergeData::default();
    let cx = Ctx {
        data: &data,
        image_base: "/img",
    };
    let body = blocks::html(issue.doc, &cx);
    let name = esc(issue.newsletter_name);
    let slug = esc(newsletter_slug);
    let subscribe = if signup_open {
        format!(
            r#"<p style="margin:32px 0 0"><a href="/join/{slug}" style="display:inline-block;padding:10px 18px;border-radius:6px;background:#18181b;color:#ffffff;text-decoration:none;font-weight:600">Subscribe to {name}</a></p>"#
        )
    } else {
        String::new()
    };
    format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta http-equiv="Content-Security-Policy" content="{ARCHIVE_CSP}"><title>{subject} · {name}</title></head><body style="margin:0;background:#f4f4f5;font-family:{FONT};color:#18181b"><main style="max-width:600px;margin:0 auto;padding:24px 16px"><p style="margin:0 0 16px;font-size:14px"><a href="/p/{slug}" style="color:#52525b">← {name}</a></p><article style="background:#ffffff;border-radius:8px;padding:32px 28px"><h1 style="margin:0 0 4px;font-size:28px;line-height:1.3">{subject}</h1><p style="margin:0 0 24px;font-size:14px;color:#71717a"><time datetime="{iso}">{date}</time></p>{body}</article>{subscribe}</main></body></html>"#,
        subject = esc(issue.subject),
        iso = sent_at.to_rfc3339(),
        date = sent_at.format("%-d %b %Y"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::blocks::parse;
    use chrono::TimeZone;
    use serde_json::json;

    const S: &[u8] = b"0123456789abcdef0123456789abcdef";

    fn doc() -> Doc {
        parse(&json!({"type": "doc", "content": [
            {"type": "paragraph", "content": [
                {"type": "text", "text": "Hi "},
                {"type": "mergeTag", "attrs": {"key": "first_name", "fallback": "there"}},
                {"type": "text", "text": ", read "},
                {"type": "text", "text": "this", "marks": [{"type": "link", "attrs": {"href": "https://shop.com/post"}}]}
            ]},
            {"type": "image", "attrs": {"id": "0190a0a0-0000-7000-8000-000000000001", "alt": "Cat"}}
        ]}))
        .unwrap()
    }

    fn issue(d: &Doc) -> Issue<'_> {
        Issue {
            subject: "Weekly <1>",
            preview_text: Some("What is new"),
            newsletter_name: "Shop & Co",
            postal_address: "1 Main St\nLondon <UK>",
            doc: d,
        }
    }

    #[test]
    fn an_issue_email_has_template_footer_and_one_click_headers() {
        let d = doc();
        let ann = MergeData {
            first_name: Some("Ann".into()),
            ..Default::default()
        };
        let p = email(
            &issue(&d),
            &ann,
            "https://app.shop.com",
            Some("https://app.shop.com/u/TOKEN"),
        );
        assert_eq!(
            p.headers.get("List-Unsubscribe").map(String::as_str),
            Some("<https://app.shop.com/u/TOKEN>")
        );
        assert_eq!(
            p.headers.get("List-Unsubscribe-Post").map(String::as_str),
            Some("List-Unsubscribe=One-Click")
        );
        assert!(p.html.starts_with("<!doctype html>"));
        assert!(p.html.contains("<title>Weekly &lt;1&gt;</title>"));
        assert!(p.html.contains("What is new"), "preview text");
        assert!(p.html.contains("Hi Ann, read"));
        assert!(
            p.html
                .contains(r#"src="https://app.shop.com/img/0190a0a0-0000-7000-8000-000000000001""#)
        );
        assert!(p.html.contains("Shop &amp; Co"));
        assert!(p.html.contains("1 Main St<br>London &lt;UK&gt;"));
        assert!(
            p.html
                .contains(r#"<a data-mr-skip href="https://app.shop.com/u/TOKEN""#)
        );
        assert!(p.text.contains("Hi Ann, read this (https://shop.com/post)"));
        assert!(p.text.contains("Unsubscribe: https://app.shop.com/u/TOKEN"));
        assert!(
            p.text.contains("1 Main St\nLondon <UK>"),
            "text is not HTML-escaped"
        );
    }

    #[test]
    fn a_test_email_has_no_unsubscribe_headers() {
        let d = doc();
        let p = email(
            &issue(&d),
            &MergeData::default(),
            "https://app.shop.com",
            None,
        );
        assert!(p.headers.is_empty());
        assert!(p.html.contains("Hi there, read"));
        assert!(p.html.contains("This is a test email"));
        assert!(!p.html.contains("/u/"));
    }

    #[test]
    fn the_size_counts_tracking_and_long_merge_values() {
        let d = doc();
        let plain = email(
            &issue(&d),
            &MergeData::default(),
            "https://app.shop.com",
            Some("https://app.shop.com/u/x"),
        )
        .html
        .len();
        let measured = email_size(&issue(&d), "https://app.shop.com", S).unwrap();
        assert!(
            measured > plain + 90,
            "a 100-char first name and the tracked links are counted"
        );
        assert!(measured < MAX_EMAIL_HTML);
        let big = parse(&json!({"type": "doc", "content": (0..1200).map(|_| json!({"type": "paragraph", "content": [{"type": "text", "text": "x".repeat(90)}]})).collect::<Vec<_>>()})).unwrap();
        assert!(email_size(&issue(&big), "https://app.shop.com", S).unwrap() > MAX_EMAIL_HTML);
    }

    #[test]
    fn the_archive_page_shows_fallbacks_and_nothing_personal() {
        let d = doc();
        let at = Utc.with_ymd_and_hms(2026, 10, 13, 9, 30, 0).unwrap();
        let page = archive_page(&issue(&d), at, "weekly", true);
        assert!(page.starts_with("<!doctype html>"));
        assert!(page.contains("<h1"), "the subject is the page title");
        assert!(page.contains("Weekly &lt;1&gt;"));
        assert!(page.contains("Hi there, read"));
        assert!(page.contains(r#"src="/img/0190a0a0-0000-7000-8000-000000000001""#));
        assert!(page.contains(r#"href="/p/weekly""#));
        assert!(page.contains(r#"href="/join/weekly""#));
        assert!(page.contains("13 Oct 2026"));
        assert!(
            !page.contains("/o/") && !page.contains("Unsubscribe") && !page.contains("<script")
        );
        assert!(!archive_page(&issue(&d), at, "weekly", false).contains("/join/"));
    }
}
