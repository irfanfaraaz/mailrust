use std::collections::BTreeMap;

use lettre::{
    Message,
    message::{Attachment, Mailbox, MultiPart, SinglePart, header::ContentType},
};
use uuid::Uuid;

pub mod blocks;
pub mod campaign;
pub mod tracking;

#[derive(Debug, Clone)]
pub struct AttachmentData {
    pub filename: String,
    pub content_type: String,
    pub content: Vec<u8>,
}

pub struct Tracking<'a> {
    pub base_url: String,
    pub secret: &'a [u8],
    pub opens: bool,
    pub clicks: bool,
}

pub struct RenderInput<'a> {
    pub email_id: Uuid,
    pub from: &'a str,
    pub to: &'a [String],
    pub cc: &'a [String],
    pub reply_to: &'a [String],
    pub subject: &'a str,
    pub html: Option<&'a str>,
    pub text: Option<&'a str>,
    pub headers: &'a BTreeMap<String, String>,
    pub attachments: &'a [AttachmentData],
    pub tracking: Option<Tracking<'a>>,
}

pub struct Rendered {
    pub raw: Vec<u8>,
    pub links: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("invalid address: {0}")]
    Address(String),
    #[error("could not build the message: {0}")]
    Build(String),
    #[error("could not rewrite the HTML: {0}")]
    Html(String),
}

fn mailbox(s: &str) -> Result<Mailbox, RenderError> {
    s.parse::<Mailbox>()
        .map_err(|_| RenderError::Address(s.to_string()))
}

/// "Ann <ANN@Shop.com>" -> "ann@shop.com". Falls back to the trimmed, lowercased input.
pub fn bare_address(input: &str) -> String {
    match input.parse::<Mailbox>() {
        Ok(m) => m.email.to_string().to_ascii_lowercase(),
        Err(_) => input.trim().to_ascii_lowercase(),
    }
}

enum Body {
    Single(SinglePart),
    Multi(MultiPart),
}

pub fn render(input: &RenderInput) -> Result<Rendered, RenderError> {
    let from = mailbox(input.from)?;
    let from_domain = from.email.domain().to_string();

    let (html, links) = match (input.html, &input.tracking) {
        (Some(h), Some(t)) if t.opens || t.clicks => {
            let (h, l) = tracking::apply(h, input.email_id, t)?;
            (Some(h), l)
        }
        (h, _) => (h.map(tracking::strip_skip).transpose()?, Vec::new()),
    };

    let mut b = Message::builder()
        .from(from)
        .subject(input.subject)
        .message_id(Some(format!("<{}@{}>", input.email_id, from_domain)));
    for a in input.to {
        b = b.to(mailbox(a)?);
    }
    for a in input.cc {
        b = b.cc(mailbox(a)?);
    }
    for a in input.reply_to {
        b = b.reply_to(mailbox(a)?);
    }

    let body = match (html, input.text) {
        (Some(h), Some(t)) => Body::Multi(MultiPart::alternative_plain_html(t.to_string(), h)),
        (Some(h), None) => Body::Single(SinglePart::html(h)),
        (None, Some(t)) => Body::Single(SinglePart::plain(t.to_string())),
        (None, None) => return Err(RenderError::Build("html or text is required".into())),
    };

    let message = if input.attachments.is_empty() {
        match body {
            Body::Single(p) => b.singlepart(p),
            Body::Multi(m) => b.multipart(m),
        }
    } else {
        let mut mixed = match body {
            Body::Single(p) => MultiPart::mixed().singlepart(p),
            Body::Multi(m) => MultiPart::mixed().multipart(m),
        };
        for a in input.attachments {
            let ct = ContentType::parse(&a.content_type)
                .map_err(|e| RenderError::Build(e.to_string()))?;
            mixed =
                mixed.singlepart(Attachment::new(a.filename.clone()).body(a.content.clone(), ct));
        }
        b.multipart(mixed)
    }
    .map_err(|e| RenderError::Build(e.to_string()))?;

    // Custom headers go first. Header order does not matter in email, and the
    // API already rejected names that lettre sets itself and values with CR/LF.
    let mut raw = Vec::new();
    for (k, v) in input.headers {
        raw.extend_from_slice(format!("{k}: {v}\r\n").as_bytes());
    }
    raw.extend_from_slice(&message.formatted());
    Ok(Rendered { raw, links })
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &[u8] = b"0123456789abcdef0123456789abcdef";

    fn input<'a>(
        to: &'a [String],
        headers: &'a BTreeMap<String, String>,
        attachments: &'a [AttachmentData],
        html: Option<&'a str>,
        tracking: Option<Tracking<'a>>,
    ) -> RenderInput<'a> {
        RenderInput {
            email_id: Uuid::now_v7(),
            from: "Shop <hello@shop.com>",
            to,
            cc: &[],
            reply_to: &[],
            subject: "Your receipt",
            html,
            text: Some("plain body"),
            headers,
            attachments,
            tracking,
        }
    }

    fn text(r: &Rendered) -> String {
        String::from_utf8_lossy(&r.raw).to_string()
    }

    #[test]
    fn plain_and_html_with_attachment_and_custom_header() {
        let to = vec!["Ann <ann@x.com>".to_string()];
        let headers = BTreeMap::from([("X-Order".to_string(), "42".to_string())]);
        let att = vec![AttachmentData {
            filename: "r.pdf".into(),
            content_type: "application/pdf".into(),
            content: b"%PDF".to_vec(),
        }];
        let r = render(&input(&to, &headers, &att, Some("<p>Hi</p>"), None)).unwrap();
        let s = text(&r);
        assert!(s.starts_with("X-Order: 42\r\n"));
        assert!(s.contains("Subject: Your receipt"));
        assert!(s.contains("multipart/alternative"));
        assert!(s.contains("Content-Disposition: attachment"));
        assert!(
            s.contains("@shop.com>"),
            "message id uses the sender domain"
        );
        assert!(r.links.is_empty());
    }

    #[test]
    fn bad_address_is_an_error() {
        let to = vec!["not an address".to_string()];
        let h = BTreeMap::new();
        assert!(matches!(
            render(&input(&to, &h, &[], None, None)),
            Err(RenderError::Address(_))
        ));
    }

    #[test]
    fn bare_address_lowercases_and_strips_names() {
        assert_eq!(bare_address("Ann <ANN@Shop.com>"), "ann@shop.com");
        assert_eq!(bare_address(" Bob@X.com "), "bob@x.com");
    }

    #[test]
    fn tracking_rewrites_links_and_adds_pixel() {
        let to = vec!["a@x.com".to_string()];
        let h = BTreeMap::new();
        let html = r##"<html><body><a href="https://shop.com/sale">Sale</a> <a href="mailto:x@y.com">Mail</a> <a href="#top">Top</a> <A HREF="http://shop.com/two">Two</A></body></html>"##;
        let t = Tracking {
            base_url: "https://track.shop.com".into(),
            secret: S,
            opens: true,
            clicks: true,
        };
        let r = render(&input(&to, &h, &[], Some(html), Some(t))).unwrap();
        assert_eq!(
            r.links,
            vec!["https://shop.com/sale", "http://shop.com/two"]
        );
        let s = text(&r);
        assert!(s.contains("track.shop.com/c/"));
        assert!(s.contains("track.shop.com/o/"));
        assert!(s.contains("mailto:x@y.com"));
        assert!(
            !s.contains("https://shop.com/sale\""),
            "original link is replaced"
        );
    }

    #[test]
    fn pixel_is_added_even_without_a_body_tag_and_broken_html_survives() {
        let id = Uuid::now_v7();
        let t = Tracking {
            base_url: "https://m.test".into(),
            secret: S,
            opens: true,
            clicks: true,
        };
        let (out, links) =
            tracking::apply("<p>Hello <a href='https://a.com'>there", id, &t).unwrap();
        assert!(out.contains("Hello"));
        assert!(out.contains("https://m.test/o/"));
        assert_eq!(links, vec!["https://a.com"]);
    }

    #[test]
    fn opens_only_leaves_links_alone() {
        let id = Uuid::now_v7();
        let t = Tracking {
            base_url: "https://m.test".into(),
            secret: S,
            opens: true,
            clicks: false,
        };
        let (out, links) =
            tracking::apply(r#"<body><a href="https://a.com">x</a></body>"#, id, &t).unwrap();
        assert!(out.contains(r#"href="https://a.com""#));
        assert!(links.is_empty());
    }

    #[test]
    fn links_marked_skip_are_not_tracked() {
        let t = Tracking {
            base_url: "https://m.test".into(),
            secret: S,
            opens: true,
            clicks: true,
        };
        let html = r#"<body><a href="https://shop.com/a">A</a><a data-mr-skip href="https://app.test/u/tok">Unsubscribe</a></body>"#;
        let (out, links) = tracking::apply(html, Uuid::now_v7(), &t).unwrap();
        assert_eq!(links, vec!["https://shop.com/a"]);
        assert!(out.contains(r#"href="https://app.test/u/tok""#));
        assert!(!out.contains("data-mr-skip"));
    }

    #[test]
    fn the_skip_marker_is_stripped_with_tracking_off_or_opens_only() {
        let to = vec!["a@x.com".to_string()];
        let h = BTreeMap::new();
        let html = r#"<body><a data-mr-skip href="https://app.test/u/tok">Unsubscribe</a></body>"#;
        let opens = Tracking {
            base_url: "https://m.test".into(),
            secret: S,
            opens: true,
            clicks: false,
        };
        for t in [None, Some(opens)] {
            let s = text(&render(&input(&to, &h, &[], Some(html), t)).unwrap());
            assert!(s.contains("https://app.test/u/tok"));
            assert!(!s.contains("data-mr-skip"));
        }
    }
}
