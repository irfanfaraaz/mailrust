use std::cell::{Cell, RefCell};

use lol_html::{RewriteStrSettings, element, html_content::ContentType, rewrite_str};
use uuid::Uuid;

use super::{RenderError, Tracking};
use crate::tokens::{self, TokenKind};

/// Rewrites http(s) links to go through /c/<token> and adds the 1x1 open pixel.
/// Returns the new HTML and the original link list (index = link number in the token).
pub fn apply(
    html: &str,
    email_id: Uuid,
    t: &Tracking,
) -> Result<(String, Vec<String>), RenderError> {
    let links = RefCell::new(Vec::<String>::new());
    let saw_body = Cell::new(false);
    let pixel = format!(
        r#"<img src="{}/o/{}.gif" width="1" height="1" alt="" style="border:0;width:1px;height:1px" />"#,
        t.base_url,
        tokens::sign(t.secret, email_id, TokenKind::Open)
    );

    let mut handlers = Vec::new();
    if t.clicks {
        handlers.push(element!("a[href]", |el| {
            // The unsubscribe footer: never through /c/ (a link scanner or a click count must not touch it).
            if el.get_attribute("data-mr-skip").is_some() {
                el.remove_attribute("data-mr-skip");
                return Ok(());
            }
            if let Some(href) = el.get_attribute("href") {
                let target = href.trim().to_string();
                let lower = target.to_ascii_lowercase();
                let mut list = links.borrow_mut();
                if (lower.starts_with("http://") || lower.starts_with("https://"))
                    && list.len() < u16::MAX as usize
                {
                    let idx = list.len() as u16;
                    list.push(target);
                    let token = tokens::sign(t.secret, email_id, TokenKind::Click(idx));
                    el.set_attribute("href", &format!("{}/c/{}", t.base_url, token))?;
                }
            }
            Ok(())
        }));
    }
    if t.opens {
        handlers.push(element!("body", |el| {
            saw_body.set(true);
            el.append(&pixel, ContentType::Html);
            Ok(())
        }));
    }

    let mut out = rewrite_str(
        html,
        RewriteStrSettings {
            element_content_handlers: handlers,
            ..RewriteStrSettings::new()
        },
    )
    .map_err(|e| RenderError::Html(e.to_string()))?;
    if t.opens && !saw_body.get() {
        out.push_str(&pixel);
    }
    Ok((out, links.into_inner()))
}
