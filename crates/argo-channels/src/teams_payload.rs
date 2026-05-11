//! Payload helpers for the Teams channel — adaptive-card building,
//! minimal inline-markdown to HTML conversion, and approval-token
//! extraction. Split out of `teams.rs` to stay under the 500-LOC
//! per-file ceiling.

use crate::{ChannelError, ChannelResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// One adaptive-card attachment (title + body text).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TeamsAttachment {
    /// Card title rendered as the heading TextBlock.
    pub title: String,
    /// Card body rendered as a wrapped TextBlock.
    pub body: String,
}

/// Build one `application/vnd.microsoft.card.adaptive` (v1.5)
/// attachment entry. Graph requires the inner card to be JSON-
/// stringified into the `content` field.
#[must_use]
pub fn adaptive_card(idx: usize, a: &TeamsAttachment) -> Value {
    let content = json!({
        "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
        "type": "AdaptiveCard",
        "version": "1.5",
        "body": [
            {"type": "TextBlock", "size": "Medium", "weight": "Bolder", "text": a.title},
            {"type": "TextBlock", "text": a.body, "wrap": true},
        ],
    })
    .to_string();
    json!({
        "id": idx.to_string(),
        "contentType": "application/vnd.microsoft.card.adaptive",
        "content": content,
    })
}

/// HTML-escape, then convert `**bold**`, `*italic*`, `` `code` ``,
/// `[text](url)` to inline tags; wrap in `<p>`. When `markdown` is
/// `false`, the text is just HTML-escaped.
#[must_use]
pub fn body_html(text: &str, markdown: bool) -> String {
    let escaped = html_escape(text);
    if !markdown {
        return escaped;
    }
    let s = wrap_pairs(&escaped, "**", "<strong>", "</strong>");
    let s = wrap_pairs(&s, "*", "<em>", "</em>");
    let s = wrap_pairs(&s, "`", "<code>", "</code>");
    let s = expand_links(&s);
    let s = s.replace("\n\n", "</p><p>").replace('\n', "<br/>");
    format!("<p>{s}</p>")
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn wrap_pairs(text: &str, marker: &str, open: &str, close: &str) -> String {
    let parts: Vec<&str> = text.split(marker).collect();
    if parts.len() < 3 {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    for (i, part) in parts.iter().enumerate() {
        out.push_str(part);
        if i + 1 < parts.len() {
            out.push_str(if i % 2 == 0 { open } else { close });
        }
    }
    out
}

fn expand_links(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(end_text) = after.find(']') else {
            out.push_str(&rest[open..]);
            return out;
        };
        let label = &after[..end_text];
        let after_label = &after[end_text + 1..];
        if !after_label.starts_with('(') {
            out.push('[');
            rest = after;
            continue;
        }
        let after_paren = &after_label[1..];
        let Some(end_url) = after_paren.find(')') else {
            out.push_str(&rest[open..]);
            return out;
        };
        let url = &after_paren[..end_url];
        out.push_str(&format!("<a href=\"{url}\">{label}</a>"));
        rest = &after_paren[end_url + 1..];
    }
    out.push_str(rest);
    out
}

/// Extract the `token=...` query parameter from a signed approval
/// URL.
///
/// # Errors
/// Returns [`ChannelError::Config`] when the URL has no query string
/// or no `token=` parameter.
pub fn token_from_url(url: &str) -> ChannelResult<String> {
    url.split_once('?')
        .and_then(|(_, q)| q.split('&').find_map(|kv| kv.strip_prefix("token=")))
        .map(str::to_string)
        .ok_or_else(|| ChannelError::Config("Teams approval URL missing token".into()))
}
