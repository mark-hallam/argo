//! `web_extract` body extraction (#723). Fetches a URL with
//! reqwest and runs a small readability-style pass over the
//! response HTML — strips chrome (`<script>` / `<style>` /
//! `<nav>` / `<header>` / `<footer>` / `<aside>`), prefers
//! `<article>` / `<main>` / `[role=main]` content, falls back
//! to all `<p>` tags inside `<body>`. No JS rendering — that's
//! the browser MCP server's job (#642).

use super::{any_selector, http_client, UA};

/// Cap on the extracted body length surfaced to the model.
/// Long-form articles get truncated at ~32k chars; the worker
/// can re-extract a different region if needed.
pub(super) const EXTRACT_MAX_CHARS: usize = 32_000;

pub(super) async fn fetch_and_extract(url: &str) -> Result<(String, String), String> {
    let client = http_client()?;
    let resp = client
        .get(url)
        .header(reqwest::header::USER_AGENT, UA)
        .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml")
        .send()
        .await
        .map_err(|e| format!("fetch {url}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("status {} for {url}", resp.status()));
    }
    let html = resp.text().await.map_err(|e| format!("body {url}: {e}"))?;
    Ok(extract_readable(&html))
}

/// Extract the readable text + title from raw HTML. Strategy:
/// 1. Title from `<title>` (or `<h1>` fallback).
/// 2. Body text from `<article>`, `<main>`, or `[role=main]`,
///    in that order; if none match, walk every `<p>` inside
///    `<body>`.
/// 3. Drop `<script>`, `<style>`, `<nav>`, `<header>`,
///    `<footer>`, `<aside>` content before joining.
/// 4. Collapse runs of whitespace; cap at EXTRACT_MAX_CHARS.
fn extract_readable(html: &str) -> (String, String) {
    let doc = scraper::Html::parse_document(html);
    let title = doc
        .select(&scraper::Selector::parse("title").unwrap_or_else(|_| any_selector()))
        .next()
        .map(|n| n.text().collect::<String>().trim().to_string())
        .unwrap_or_default();

    // Containers we drop entirely before joining text.
    let drop_sel = scraper::Selector::parse(
        "script, style, noscript, nav, header, footer, aside, form, iframe",
    )
    .unwrap_or_else(|_| any_selector());
    let drop_set: std::collections::HashSet<_> = doc.select(&drop_sel).map(|e| e.id()).collect();

    // Try the "main content" containers in priority order.
    let containers = [
        "article",
        "main",
        "[role=main]",
        ".post-body",
        ".article-body",
        "#content",
    ];
    for css in containers {
        if let Ok(sel) = scraper::Selector::parse(css) {
            if let Some(node) = doc.select(&sel).next() {
                let text = collect_text(node, &drop_set);
                if text.chars().count() >= 200 {
                    return (title, cap(text));
                }
            }
        }
    }

    // Fallback: every `<p>` inside <body>, joined with blank
    // lines. Keeps the layout readable for the model.
    let p_sel = scraper::Selector::parse("body p").unwrap_or_else(|_| any_selector());
    let mut paras: Vec<String> = Vec::new();
    for p in doc.select(&p_sel) {
        if drop_set.contains(&p.id()) {
            continue;
        }
        let t = collect_text(p, &drop_set);
        if !t.trim().is_empty() {
            paras.push(t);
        }
    }
    (title, cap(paras.join("\n\n")))
}

fn collect_text(
    node: scraper::ElementRef<'_>,
    drop_set: &std::collections::HashSet<ego_tree::NodeId>,
) -> String {
    let mut out = String::new();
    walk(*node, drop_set, &mut out);
    collapse_ws(&out)
}

/// Recursive walk; skips children of dropped elements.
fn walk(
    node: ego_tree::NodeRef<'_, scraper::node::Node>,
    drop_set: &std::collections::HashSet<ego_tree::NodeId>,
    out: &mut String,
) {
    if drop_set.contains(&node.id()) {
        return;
    }
    if let Some(t) = node.value().as_text() {
        out.push_str(t);
        return;
    }
    if let Some(el) = node.value().as_element() {
        let tag = el.name();
        if matches!(
            tag,
            "p" | "br" | "div" | "li" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "tr"
        ) {
            out.push('\n');
        }
    }
    for child in node.children() {
        walk(child, drop_set, out);
    }
}

fn collapse_ws(s: &str) -> String {
    // Collapse runs of horizontal whitespace; preserve paragraph
    // breaks (two or more newlines stay as two).
    let mut out = String::with_capacity(s.len());
    let mut last_was_space = false;
    let mut newline_run: u8 = 0;
    for ch in s.chars() {
        if ch == '\n' {
            newline_run = newline_run.saturating_add(1);
            last_was_space = true;
            continue;
        }
        if newline_run > 0 {
            out.push_str(if newline_run >= 2 { "\n\n" } else { "\n" });
            newline_run = 0;
        }
        if ch.is_whitespace() {
            if !last_was_space {
                out.push(' ');
                last_was_space = true;
            }
        } else {
            out.push(ch);
            last_was_space = false;
        }
    }
    out.trim().to_string()
}

fn cap(s: String) -> String {
    if s.chars().count() <= EXTRACT_MAX_CHARS {
        return s;
    }
    s.chars().take(EXTRACT_MAX_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_readable_prefers_article_tag() {
        let html = r#"<html><head><title>Page Title</title></head>
            <body>
              <nav>nav junk</nav>
              <article>
                <h1>Heading</h1>
                <p>This is the real article body. It has enough text to clear the 200-character threshold so the article-tag path is chosen over the paragraph fallback. Lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore.</p>
              </article>
              <footer>cookie banner blah</footer>
            </body></html>"#;
        let (title, body) = extract_readable(html);
        assert_eq!(title, "Page Title");
        assert!(body.contains("real article body"));
        assert!(!body.contains("nav junk"), "nav must be dropped");
        assert!(!body.contains("cookie banner"), "footer must be dropped");
    }

    #[test]
    fn extract_readable_falls_back_to_paragraphs_when_no_article() {
        let html = r#"<html><head><title>P</title></head>
            <body>
              <p>First paragraph.</p>
              <p>Second paragraph.</p>
              <script>tracking()</script>
            </body></html>"#;
        let (_, body) = extract_readable(html);
        assert!(body.contains("First paragraph"));
        assert!(body.contains("Second paragraph"));
        assert!(!body.contains("tracking"), "script must be dropped");
    }

    #[test]
    fn extract_readable_caps_extremely_long_bodies() {
        // 50k 'x' characters wrapped in a single paragraph.
        let big = "x".repeat(50_000);
        let html = format!("<html><body><article><p>{big}</p></article></body></html>");
        let (_, body) = extract_readable(&html);
        assert!(body.chars().count() <= EXTRACT_MAX_CHARS);
    }

    #[test]
    fn collapse_ws_keeps_paragraph_breaks_collapses_horizontal() {
        let s = "  hello   world\n\n\nnext  line  \n  again";
        let out = collapse_ws(s);
        assert!(out.contains("hello world"));
        assert!(out.contains("\n\n"));
        assert!(!out.contains("   "));
    }
}
