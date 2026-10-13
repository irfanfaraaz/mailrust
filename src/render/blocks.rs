//! The editor document (Tiptap JSON) → email HTML and plain text. No raw HTML ever passes through:
//! every tag is built here, and every text and attribute value is escaped.
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
};

use serde::Deserialize;
use uuid::Uuid;

use crate::contacts::valid_field_key;

pub const MAX_FALLBACK: usize = 100;
const BUILT_IN: [&str; 3] = ["first_name", "last_name", "email"];

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Node {
    Doc {
        #[serde(default)]
        content: Vec<Node>,
    },
    Paragraph {
        #[serde(default)]
        content: Vec<Node>,
    },
    Heading {
        attrs: HeadingAttrs,
        #[serde(default)]
        content: Vec<Node>,
    },
    BulletList {
        #[serde(default)]
        content: Vec<Node>,
    },
    OrderedList {
        #[serde(default)]
        content: Vec<Node>,
    },
    ListItem {
        #[serde(default)]
        content: Vec<Node>,
    },
    Blockquote {
        #[serde(default)]
        content: Vec<Node>,
    },
    HorizontalRule {},
    HardBreak {},
    Text {
        text: String,
        #[serde(default)]
        marks: Vec<Mark>,
    },
    Image {
        attrs: ImageAttrs,
    },
    Button {
        attrs: ButtonAttrs,
    },
    MergeTag {
        attrs: MergeTagAttrs,
    },
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Mark {
    Bold {},
    Italic {},
    Link {
        attrs: LinkAttrs,
    },
    /// Pasted marks (underline, colours, …): the text renders plain.
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct HeadingAttrs {
    pub level: u8,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct LinkAttrs {
    pub href: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ImageAttrs {
    pub id: Uuid,
    #[serde(default)]
    pub alt: String,
    #[serde(default)]
    pub href: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ButtonAttrs {
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub href: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct MergeTagAttrs {
    pub key: String,
    #[serde(default)]
    pub fallback: Option<String>,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum BlockError {
    #[error("the issue body is not a valid document: {0}")]
    Shape(String),
    #[error("the merge tag {0} needs a fallback for readers without it")]
    MergeTagFallback(String),
    #[error("{0} is not a merge tag: use first_name, last_name, email or a contact field name")]
    MergeTagKey(String),
    #[error("links must start with http://, https:// or mailto: ({0})")]
    LinkScheme(String),
    #[error("every image needs alt text")]
    ImageAlt,
    #[error("headings are level 1, 2 or 3")]
    HeadingLevel,
    #[error("every button needs a label")]
    ButtonLabel,
}

impl BlockError {
    pub fn code(&self) -> &'static str {
        match self {
            BlockError::MergeTagFallback(_) => "merge_tag_fallback",
            _ => "bad_blocks",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Doc {
    pub content: Vec<Node>,
}

/// Shape only: the JSON is a Tiptap document made of the node types above.
pub fn parse(v: &serde_json::Value) -> Result<Doc, BlockError> {
    match Node::deserialize(v).map_err(|e| BlockError::Shape(e.to_string()))? {
        Node::Doc { content } => Ok(Doc { content }),
        _ => Err(BlockError::Shape("the root must be a doc".into())),
    }
}

fn children(n: &Node) -> &[Node] {
    match n {
        Node::Doc { content }
        | Node::Paragraph { content }
        | Node::Heading { content, .. }
        | Node::BulletList { content }
        | Node::OrderedList { content }
        | Node::ListItem { content }
        | Node::Blockquote { content } => content,
        _ => &[],
    }
}

fn walk<'a>(
    nodes: &'a [Node],
    f: &mut impl FnMut(&'a Node) -> Result<(), BlockError>,
) -> Result<(), BlockError> {
    for n in nodes {
        f(n)?;
        walk(children(n), f)?;
    }
    Ok(())
}

pub fn safe_link(href: &str) -> bool {
    url::Url::parse(href.trim()).is_ok_and(|u| {
        matches!(u.scheme(), "http" | "https" | "mailto")
            && (u.scheme() == "mailto" || u.host_str().is_some_and(|h| !h.is_empty()))
    })
}

fn check_link(href: &str) -> Result<(), BlockError> {
    if safe_link(href) {
        Ok(())
    } else {
        Err(BlockError::LinkScheme(
            href.trim().chars().take(80).collect(),
        ))
    }
}

fn check_merge(a: &MergeTagAttrs) -> Result<(), BlockError> {
    if !BUILT_IN.contains(&a.key.as_str()) && !valid_field_key(&a.key) {
        return Err(BlockError::MergeTagKey(a.key.chars().take(40).collect()));
    }
    let fallback = a.fallback.as_deref().map(str::trim).unwrap_or("");
    if a.key != "email" && fallback.is_empty() {
        return Err(BlockError::MergeTagFallback(a.key.clone()));
    }
    if fallback.chars().count() > MAX_FALLBACK {
        return Err(BlockError::Shape(
            "a merge tag fallback is at most 100 characters".into(),
        ));
    }
    Ok(())
}

/// The rules a document must follow before it can be sent.
pub fn validate(doc: &Doc) -> Result<(), BlockError> {
    walk(&doc.content, &mut |n| match n {
        Node::Heading { attrs, .. } if !(1..=3).contains(&attrs.level) => {
            Err(BlockError::HeadingLevel)
        }
        Node::Text { marks, .. } => marks.iter().try_for_each(|m| match m {
            Mark::Link { attrs } => check_link(&attrs.href),
            _ => Ok(()),
        }),
        Node::Image { attrs } => {
            if attrs.alt.trim().is_empty() {
                return Err(BlockError::ImageAlt);
            }
            attrs.href.as_deref().map_or(Ok(()), check_link)
        }
        Node::Button { attrs } => {
            if attrs.label.trim().is_empty() {
                return Err(BlockError::ButtonLabel);
            }
            check_link(&attrs.href)
        }
        Node::MergeTag { attrs } => check_merge(attrs),
        _ => Ok(()),
    })
}

pub fn image_ids(doc: &Doc) -> Vec<Uuid> {
    let mut ids = BTreeSet::new();
    let _ = walk(&doc.content, &mut |n| {
        if let Node::Image { attrs } = n {
            ids.insert(attrs.id);
        }
        Ok(())
    });
    ids.into_iter().collect()
}

pub fn merge_keys(doc: &Doc) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    let _ = walk(&doc.content, &mut |n| {
        if let Node::MergeTag { attrs } = n {
            keys.insert(attrs.key.clone());
        }
        Ok(())
    });
    keys
}

/// What readers without a first name see (for the send dialog's "312 of 1,200 have no first name").
pub fn first_name_fallback(doc: &Doc) -> Option<String> {
    let mut found = None;
    let _ = walk(&doc.content, &mut |n| {
        if let Node::MergeTag { attrs } = n
            && attrs.key == "first_name"
            && found.is_none()
        {
            found = attrs
                .fallback
                .as_deref()
                .map(str::trim)
                .filter(|f| !f.is_empty())
                .map(str::to_string);
        }
        Ok(())
    });
    found
}

/// One reader's values. `Default` = nothing known: every tag shows its fallback (previews, the archive).
#[derive(Debug, Clone, Default)]
pub struct MergeData {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub fields: BTreeMap<String, String>,
}

impl MergeData {
    pub fn resolve<'a>(&'a self, a: &'a MergeTagAttrs) -> &'a str {
        let value = match a.key.as_str() {
            "first_name" => self.first_name.as_deref(),
            "last_name" => self.last_name.as_deref(),
            "email" => self.email.as_deref(),
            k => self.fields.get(k).map(String::as_str),
        };
        value
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .or(a.fallback.as_deref().map(str::trim))
            .unwrap_or("")
    }
}

pub struct Ctx<'a> {
    pub data: &'a MergeData,
    /// `"{WEB_URL}/img"` in an email, `"/img"` in the archive.
    pub image_base: &'a str,
}

pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#39;"),
            c => o.push(c),
        }
    }
    o
}

const P: &str = "margin:0 0 16px;font-size:16px;line-height:1.6";
const LINK: &str = "color:#2563eb;text-decoration:underline";

pub fn html(doc: &Doc, cx: &Ctx) -> String {
    let mut out = String::new();
    for n in &doc.content {
        block_html(n, cx, &mut out);
    }
    out
}

fn block_html(n: &Node, cx: &Ctx, out: &mut String) {
    match n {
        Node::Doc { content } => content.iter().for_each(|c| block_html(c, cx, out)),
        Node::Paragraph { content } => {
            let _ = write!(out, r#"<p style="{P}">"#);
            if content.is_empty() {
                out.push_str("&nbsp;");
            } else {
                inline_html(content, cx, out);
            }
            out.push_str("</p>");
        }
        Node::Heading { attrs, content } => {
            let (l, size) = match attrs.level {
                1 => (1, 28),
                2 => (2, 22),
                _ => (3, 18),
            };
            let _ = write!(
                out,
                r#"<h{l} style="margin:24px 0 12px;font-size:{size}px;line-height:1.3">"#
            );
            inline_html(content, cx, out);
            let _ = write!(out, "</h{l}>");
        }
        Node::BulletList { content } | Node::OrderedList { content } => {
            let tag = if matches!(n, Node::BulletList { .. }) {
                "ul"
            } else {
                "ol"
            };
            let _ = write!(
                out,
                r#"<{tag} style="margin:0 0 16px;padding-left:24px;font-size:16px;line-height:1.6">"#
            );
            content.iter().for_each(|c| block_html(c, cx, out));
            let _ = write!(out, "</{tag}>");
        }
        Node::ListItem { content } => {
            out.push_str(r#"<li style="margin:0 0 4px">"#);
            for c in content {
                match c {
                    Node::Paragraph { content } => inline_html(content, cx, out),
                    other => block_html(other, cx, out),
                }
            }
            out.push_str("</li>");
        }
        Node::Blockquote { content } => {
            out.push_str(r#"<blockquote style="margin:0 0 16px;padding:0 0 0 16px;border-left:3px solid #d4d4d8;color:#52525b">"#);
            content.iter().for_each(|c| block_html(c, cx, out));
            out.push_str("</blockquote>");
        }
        Node::HorizontalRule {} => {
            out.push_str(r#"<hr style="border:0;border-top:1px solid #e4e4e7;margin:24px 0">"#)
        }
        Node::Image { attrs } => {
            let img = format!(
                r#"<img src="{}/{}" alt="{}" style="display:block;max-width:100%;height:auto;margin:0 0 16px;border:0">"#,
                esc(cx.image_base),
                attrs.id,
                esc(&attrs.alt)
            );
            match attrs.href.as_deref().filter(|h| safe_link(h)) {
                Some(h) => {
                    let _ = write!(out, r#"<a href="{}">{img}</a>"#, esc(h.trim()));
                }
                None => out.push_str(&img),
            }
        }
        Node::Button { attrs } => {
            if safe_link(&attrs.href) {
                let _ = write!(
                    out,
                    r#"<table role="presentation" cellpadding="0" cellspacing="0" style="margin:0 0 16px"><tr><td style="border-radius:6px;background:#18181b"><a href="{}" style="display:inline-block;padding:12px 20px;color:#ffffff;text-decoration:none;font-weight:600;font-size:16px">{}</a></td></tr></table>"#,
                    esc(attrs.href.trim()),
                    esc(&attrs.label)
                );
            }
        }
        Node::Text { .. } | Node::HardBreak {} | Node::MergeTag { .. } => {
            inline_html(std::slice::from_ref(n), cx, out)
        }
    }
}

fn inline_html(nodes: &[Node], cx: &Ctx, out: &mut String) {
    for n in nodes {
        match n {
            Node::Text { text, marks } => {
                let link = marks.iter().find_map(|m| match m {
                    Mark::Link { attrs } if safe_link(&attrs.href) => Some(attrs.href.trim()),
                    _ => None,
                });
                let bold = marks.iter().any(|m| matches!(m, Mark::Bold {}));
                let italic = marks.iter().any(|m| matches!(m, Mark::Italic {}));
                if let Some(h) = link {
                    let _ = write!(out, r#"<a href="{}" style="{LINK}">"#, esc(h));
                }
                if bold {
                    out.push_str("<strong>");
                }
                if italic {
                    out.push_str("<em>");
                }
                out.push_str(&esc(text));
                if italic {
                    out.push_str("</em>");
                }
                if bold {
                    out.push_str("</strong>");
                }
                if link.is_some() {
                    out.push_str("</a>");
                }
            }
            Node::HardBreak {} => out.push_str("<br>"),
            Node::MergeTag { attrs } => out.push_str(&esc(cx.data.resolve(attrs))),
            // A block where inline content was expected (a hand-written document): render it as a block.
            other => block_html(other, cx, out),
        }
    }
}

pub fn text(doc: &Doc, cx: &Ctx) -> String {
    let mut out = String::new();
    for n in &doc.content {
        block_text(n, cx, &mut out, "");
    }
    format!("{}\n", out.trim_end())
}

fn block_text(n: &Node, cx: &Ctx, out: &mut String, prefix: &str) {
    match n {
        Node::Doc { content } | Node::Blockquote { content } => {
            let p = if matches!(n, Node::Blockquote { .. }) {
                format!("{prefix}> ")
            } else {
                prefix.to_string()
            };
            content.iter().for_each(|c| block_text(c, cx, out, &p));
        }
        Node::Paragraph { content } | Node::Heading { content, .. } => {
            out.push_str(prefix);
            out.push_str(&inline_text(content, cx).replace('\n', &format!("\n{prefix}")));
            out.push_str("\n\n");
        }
        Node::BulletList { content } => {
            content
                .iter()
                .for_each(|i| list_item_text(i, "- ", cx, out, prefix));
            out.push('\n');
        }
        Node::OrderedList { content } => {
            for (k, i) in content.iter().enumerate() {
                list_item_text(i, &format!("{}. ", k + 1), cx, out, prefix);
            }
            out.push('\n');
        }
        Node::ListItem { .. } => list_item_text(n, "- ", cx, out, prefix),
        Node::HorizontalRule {} => {
            out.push_str(prefix);
            out.push_str("---\n\n");
        }
        Node::Image { attrs } => {
            let _ = write!(out, "{prefix}[{}]", attrs.alt.trim());
            if let Some(h) = attrs.href.as_deref().filter(|h| safe_link(h)) {
                let _ = write!(out, " ({})", h.trim());
            }
            out.push_str("\n\n");
        }
        Node::Button { attrs } => {
            if safe_link(&attrs.href) {
                let _ = write!(
                    out,
                    "{prefix}{}: {}\n\n",
                    attrs.label.trim(),
                    attrs.href.trim()
                );
            }
        }
        Node::Text { .. } | Node::HardBreak {} | Node::MergeTag { .. } => {
            out.push_str(prefix);
            out.push_str(&inline_text(std::slice::from_ref(n), cx));
            out.push_str("\n\n");
        }
    }
}

fn list_item_text(item: &Node, marker: &str, cx: &Ctx, out: &mut String, prefix: &str) {
    let parts: Vec<String> = children(item)
        .iter()
        .map(|c| match c {
            Node::Paragraph { content } => inline_text(content, cx),
            other => {
                let mut s = String::new();
                block_text(other, cx, &mut s, "  ");
                s.trim_end().to_string()
            }
        })
        .collect();
    let _ = writeln!(out, "{prefix}{marker}{}", parts.join("\n  "));
}

fn inline_text(nodes: &[Node], cx: &Ctx) -> String {
    let mut s = String::new();
    for n in nodes {
        match n {
            Node::Text { text, marks } => {
                s.push_str(text);
                let link = marks.iter().find_map(|m| match m {
                    Mark::Link { attrs } if safe_link(&attrs.href) => Some(attrs.href.trim()),
                    _ => None,
                });
                if let Some(h) = link
                    && h != text.trim()
                    && !h.to_ascii_lowercase().starts_with("mailto:")
                {
                    let _ = write!(s, " ({h})");
                }
            }
            Node::HardBreak {} => s.push('\n'),
            Node::MergeTag { attrs } => s.push_str(cx.data.resolve(attrs)),
            _ => {}
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn doc(content: serde_json::Value) -> Doc {
        parse(&json!({"type": "doc", "content": content})).unwrap()
    }
    fn p(text: &str) -> serde_json::Value {
        json!({"type": "paragraph", "content": [{"type": "text", "text": text}]})
    }
    fn out(d: &Doc, data: &MergeData) -> (String, String) {
        let cx = Ctx {
            data,
            image_base: "https://app.shop.com/img",
        };
        (html(d, &cx), text(d, &cx))
    }
    const IMG: &str = "0190a0a0-0000-7000-8000-000000000001";

    #[test]
    fn every_block_renders_to_html_and_text() {
        let d = doc(json!([
            {"type": "heading", "attrs": {"level": 2}, "content": [{"type": "text", "text": "News"}]},
            {"type": "paragraph", "content": [
                {"type": "text", "text": "Hello "},
                {"type": "text", "text": "bold", "marks": [{"type": "bold"}]},
                {"type": "text", "text": " and "},
                {"type": "text", "text": "site", "marks": [{"type": "link", "attrs": {"href": "https://shop.com/a"}}, {"type": "italic"}]},
                {"type": "hardBreak"},
                {"type": "text", "text": "next line"}
            ]},
            {"type": "bulletList", "content": [
                {"type": "listItem", "content": [p("one")]},
                {"type": "listItem", "content": [p("two")]}
            ]},
            {"type": "orderedList", "attrs": {"start": 1}, "content": [{"type": "listItem", "content": [p("first")]}]},
            {"type": "blockquote", "content": [p("quoted")]},
            {"type": "horizontalRule"},
            {"type": "image", "attrs": {"id": IMG, "alt": "A cat", "href": "https://shop.com/cat", "src": "/img/x"}},
            {"type": "button", "attrs": {"label": "Shop now", "href": "https://shop.com/sale"}},
            {"type": "paragraph"}
        ]));
        validate(&d).unwrap();
        let (h, t) = out(&d, &MergeData::default());
        assert!(h.contains(r#"<h2 style="#) && h.contains(">News</h2>"));
        assert!(h.contains("<strong>bold</strong>"));
        assert!(h.contains(r#"<a href="https://shop.com/a" style="color:#2563eb;text-decoration:underline"><em>site</em></a>"#));
        assert!(h.contains("<br>next line"));
        assert!(
            h.contains("<ul ")
                && h.contains(">one</li>")
                && h.contains("<ol ")
                && h.contains(">first</li>")
        );
        assert!(h.contains("<blockquote ") && h.contains("quoted"));
        assert!(h.contains("<hr "));
        assert!(h.contains(&format!(r#"<a href="https://shop.com/cat"><img src="https://app.shop.com/img/{IMG}" alt="A cat""#)));
        assert!(h.contains(r#"<a href="https://shop.com/sale""#) && h.contains(">Shop now</a>"));
        assert!(
            h.ends_with("&nbsp;</p>"),
            "an empty paragraph keeps its space"
        );

        assert!(t.starts_with("News\n\nHello bold and site (https://shop.com/a)\nnext line\n\n"));
        assert!(t.contains("- one\n- two\n"));
        assert!(t.contains("1. first\n"));
        assert!(t.contains("> quoted\n"));
        assert!(t.contains("---\n"));
        assert!(t.contains("[A cat] (https://shop.com/cat)"));
        assert!(t.contains("Shop now: https://shop.com/sale"));
        assert!(t.ends_with('\n') && !t.ends_with("\n\n"));
    }

    #[test]
    fn text_and_attributes_are_escaped() {
        let d = doc(json!([
            {"type": "paragraph", "content": [
                {"type": "text", "text": "<script>alert('x')</script> & \"q\""},
                {"type": "text", "text": "l", "marks": [{"type": "link", "attrs": {"href": "https://a.com/?q=\"><script>"}}]}
            ]},
            {"type": "image", "attrs": {"id": IMG, "alt": "\" onerror=\"x"}},
            {"type": "button", "attrs": {"label": "<b>Go</b>", "href": "https://a.com"}}
        ]));
        let (h, _) = out(&d, &MergeData::default());
        assert!(!h.contains("<script>") && !h.contains("<b>"));
        assert!(h.contains("&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt; &amp; &quot;q&quot;"));
        assert!(h.contains(r#"href="https://a.com/?q=&quot;&gt;&lt;script&gt;""#));
        assert!(h.contains(r#"alt="&quot; onerror=&quot;x""#));
        assert!(h.contains("&lt;b&gt;Go&lt;/b&gt;"));
    }

    #[test]
    fn only_http_https_and_mailto_links_pass() {
        for ok in [
            "https://a.com",
            "http://a.com/x",
            " HTTPS://A.COM ",
            "mailto:hi@a.com",
        ] {
            assert!(safe_link(ok), "{ok}");
        }
        for bad in [
            "javascript:alert(1)",
            "JaVaScRiPt:x",
            "data:text/html,x",
            "vbscript:x",
            "//a.com",
            "https://",
            "a.com",
            "",
            "ftp://a.com",
        ] {
            assert!(!safe_link(bad), "{bad}");
        }
        let link = |href: &str| {
            doc(json!([{"type": "paragraph", "content": [
            {"type": "text", "text": "x", "marks": [{"type": "link", "attrs": {"href": href}}]}]}]))
        };
        assert!(matches!(
            validate(&link("javascript:alert(1)")),
            Err(BlockError::LinkScheme(_))
        ));
        let bad_button =
            doc(json!([{"type": "button", "attrs": {"label": "Go", "href": "data:x"}}]));
        assert!(matches!(
            validate(&bad_button),
            Err(BlockError::LinkScheme(_))
        ));
        let bad_image = doc(
            json!([{"type": "image", "attrs": {"id": IMG, "alt": "a", "href": "javascript:x"}}]),
        );
        assert!(matches!(
            validate(&bad_image),
            Err(BlockError::LinkScheme(_))
        ));
        // A bad link that slipped past the checks is never rendered as a link.
        let (h, _) = out(&link("javascript:alert(1)"), &MergeData::default());
        assert!(!h.contains("javascript") && h.contains(">x</p>"));
    }

    fn tag(key: &str, fallback: Option<&str>) -> Doc {
        doc(json!([{"type": "paragraph", "content": [
            {"type": "text", "text": "Hi "},
            {"type": "mergeTag", "attrs": {"key": key, "fallback": fallback}},
            {"type": "text", "text": "!"}
        ]}]))
    }

    #[test]
    fn merge_tags_use_the_value_then_the_fallback() {
        let d = tag("first_name", Some("there"));
        validate(&d).unwrap();
        let ann = MergeData {
            first_name: Some("Ann".into()),
            ..Default::default()
        };
        assert!(out(&d, &ann).0.contains("Hi Ann!"));
        assert!(out(&d, &MergeData::default()).0.contains("Hi there!"));
        let blank = MergeData {
            first_name: Some("  ".into()),
            ..Default::default()
        };
        assert!(
            out(&d, &blank).1.starts_with("Hi there!"),
            "a blank value uses the fallback"
        );
        let evil = MergeData {
            first_name: Some("<i>x</i>".into()),
            ..Default::default()
        };
        assert!(out(&d, &evil).0.contains("Hi &lt;i&gt;x&lt;/i&gt;!"));

        let field = tag("plan", Some("free"));
        validate(&field).unwrap();
        let pro = MergeData {
            fields: BTreeMap::from([("plan".into(), "pro".into())]),
            ..Default::default()
        };
        assert!(out(&field, &pro).0.contains("Hi pro!"));
        assert_eq!(merge_keys(&field), BTreeSet::from(["plan".to_string()]));
    }

    #[test]
    fn merge_tag_rules() {
        assert_eq!(
            validate(&tag("first_name", None)),
            Err(BlockError::MergeTagFallback("first_name".into()))
        );
        assert_eq!(
            validate(&tag("last_name", Some("  "))),
            Err(BlockError::MergeTagFallback("last_name".into()))
        );
        assert_eq!(
            validate(&tag("first_name", None)).unwrap_err().code(),
            "merge_tag_fallback"
        );
        validate(&tag("email", None)).expect("email needs no fallback");
        assert!(matches!(
            validate(&tag("Bad-Key", Some("x"))),
            Err(BlockError::MergeTagKey(_))
        ));
        assert!(matches!(
            validate(&tag("first_name", Some(&"x".repeat(101)))),
            Err(BlockError::Shape(_))
        ));
        assert_eq!(
            first_name_fallback(&tag("first_name", Some(" there "))),
            Some("there".into())
        );
        assert_eq!(first_name_fallback(&tag("email", None)), None);
    }

    #[test]
    fn other_rules_and_bad_shapes() {
        let h4 = doc(json!([{"type": "heading", "attrs": {"level": 4}, "content": []}]));
        assert_eq!(validate(&h4), Err(BlockError::HeadingLevel));
        let no_alt = doc(json!([{"type": "image", "attrs": {"id": IMG, "alt": " "}}]));
        assert_eq!(validate(&no_alt), Err(BlockError::ImageAlt));
        let no_label =
            doc(json!([{"type": "button", "attrs": {"label": "", "href": "https://a.com"}}]));
        assert_eq!(validate(&no_label), Err(BlockError::ButtonLabel));
        for bad in [
            json!({"type": "paragraph"}),
            json!({"type": "doc", "content": [{"type": "table"}]}),
            json!({"type": "doc", "content": [{"type": "image", "attrs": {"id": null, "alt": "x"}}]}),
            json!({"type": "doc", "content": "nope"}),
            json!([1, 2]),
        ] {
            assert!(matches!(parse(&bad), Err(BlockError::Shape(_))), "{bad}");
        }
        assert_eq!(
            parse(&json!({"type": "doc", "content": [{"type": "table"}]}))
                .unwrap_err()
                .code(),
            "bad_blocks"
        );
    }

    #[test]
    fn unknown_marks_render_as_plain_text() {
        let d = doc(json!([{"type": "paragraph", "content": [
            {"type": "text", "text": "under", "marks": [{"type": "underline"}, {"type": "textStyle", "attrs": {"color": "red"}}]}
        ]}]));
        validate(&d).unwrap();
        assert!(out(&d, &MergeData::default()).0.contains(">under</p>"));
    }

    #[test]
    fn image_ids_are_collected_once() {
        let d = doc(json!([
            {"type": "image", "attrs": {"id": IMG, "alt": "a"}},
            {"type": "blockquote", "content": [{"type": "image", "attrs": {"id": IMG, "alt": "b"}}]}
        ]));
        assert_eq!(image_ids(&d), vec![IMG.parse::<Uuid>().unwrap()]);
    }
}
