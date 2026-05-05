//! Search backends for `web_search` (#723).
//!
//! Backend selection is env-driven so operators can rotate keys
//! without a restart, and the test path stays deterministic
//! (no env → DuckDuckGo HTML scrape).

use super::{any_selector, http_client, UA};

/// One search hit, in the wire shape we hand back to the model.
#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct SearchHit {
    pub title: String,
    pub snippet: String,
    pub url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SearchBackend {
    DuckDuckGoHtml,
    Tavily,
    Brave,
}

impl SearchBackend {
    pub(super) fn detect() -> Self {
        if env_present("TAVILY_API_KEY") {
            SearchBackend::Tavily
        } else if env_present("BRAVE_API_KEY") || env_present("BRAVE_SEARCH_API_KEY") {
            SearchBackend::Brave
        } else {
            SearchBackend::DuckDuckGoHtml
        }
    }

    pub(super) fn name(&self) -> &'static str {
        match self {
            SearchBackend::DuckDuckGoHtml => "duckduckgo_html",
            SearchBackend::Tavily => "tavily",
            SearchBackend::Brave => "brave",
        }
    }

    pub(super) async fn run(&self, query: &str, limit: u32) -> Result<Vec<SearchHit>, String> {
        match self {
            SearchBackend::DuckDuckGoHtml => duckduckgo_html_search(query, limit).await,
            SearchBackend::Tavily => tavily_search(query, limit).await,
            SearchBackend::Brave => brave_search(query, limit).await,
        }
    }
}

fn env_present(key: &str) -> bool {
    std::env::var(key)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
}

async fn duckduckgo_html_search(query: &str, limit: u32) -> Result<Vec<SearchHit>, String> {
    let endpoint = std::env::var("ARGO_DDG_HTML_URL")
        .unwrap_or_else(|_| "https://html.duckduckgo.com/html/".into());
    let client = http_client()?;
    let resp = client
        .get(endpoint)
        .header(reqwest::header::USER_AGENT, UA)
        .query(&[("q", query), ("kl", "us-en")])
        .send()
        .await
        .map_err(|e| format!("DDG HTML fetch: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("DDG HTML status {}", resp.status()));
    }
    let html = resp
        .text()
        .await
        .map_err(|e| format!("DDG HTML body: {e}"))?;
    Ok(parse_ddg_html(&html, limit))
}

async fn tavily_search(query: &str, limit: u32) -> Result<Vec<SearchHit>, String> {
    let key = std::env::var("TAVILY_API_KEY").map_err(|_| "TAVILY_API_KEY not set".to_string())?;
    let endpoint =
        std::env::var("ARGO_TAVILY_URL").unwrap_or_else(|_| "https://api.tavily.com/search".into());
    let client = http_client()?;
    let body = serde_json::json!({
        "api_key": key,
        "query": query,
        "max_results": limit,
        "search_depth": "basic",
    });
    let resp = client
        .post(endpoint)
        .header(reqwest::header::USER_AGENT, UA)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Tavily fetch: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Tavily status {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("Tavily body: {e}"))?;
    let arr = json
        .get("results")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(arr
        .into_iter()
        .take(limit as usize)
        .map(|r| SearchHit {
            title: r
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            snippet: r
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            url: r
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
        .filter(|h| !h.url.is_empty())
        .collect())
}

async fn brave_search(query: &str, limit: u32) -> Result<Vec<SearchHit>, String> {
    let key = std::env::var("BRAVE_API_KEY")
        .or_else(|_| std::env::var("BRAVE_SEARCH_API_KEY"))
        .map_err(|_| "BRAVE_API_KEY not set".to_string())?;
    let endpoint = std::env::var("ARGO_BRAVE_URL")
        .unwrap_or_else(|_| "https://api.search.brave.com/res/v1/web/search".into());
    let client = http_client()?;
    let resp = client
        .get(endpoint)
        .header(reqwest::header::USER_AGENT, UA)
        .header("X-Subscription-Token", key)
        .query(&[("q", query), ("count", &limit.to_string())])
        .send()
        .await
        .map_err(|e| format!("Brave fetch: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Brave status {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("Brave body: {e}"))?;
    let arr = json
        .get("web")
        .and_then(|w| w.get("results"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(arr
        .into_iter()
        .take(limit as usize)
        .map(|r| SearchHit {
            title: r
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            snippet: r
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            url: r
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
        .filter(|h| !h.url.is_empty())
        .collect())
}

/// Parse a DuckDuckGo HTML SERP into our wire shape.
/// Selectors target the public, long-stable result class names
/// (`.result`, `.result__a`, `.result__snippet`). DDG occasionally
/// changes the HTML; if all three selectors miss simultaneously
/// the function returns an empty vec and the caller surfaces that
/// to the model as "no results" rather than panicking.
fn parse_ddg_html(html: &str, limit: u32) -> Vec<SearchHit> {
    let doc = scraper::Html::parse_document(html);
    // Accept either `.result` (current) or `.web-result` (older
    // variant) as the row container.
    let row_sel =
        scraper::Selector::parse(".result, .web-result").unwrap_or_else(|_| any_selector());
    let title_sel =
        scraper::Selector::parse(".result__a, a.result__a").unwrap_or_else(|_| any_selector());
    let snippet_sel = scraper::Selector::parse(".result__snippet, .result-snippet")
        .unwrap_or_else(|_| any_selector());

    let mut hits = Vec::new();
    for row in doc.select(&row_sel).take(limit as usize) {
        let title_node = row.select(&title_sel).next();
        let title = title_node
            .map(|n| n.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        let url = title_node
            .and_then(|n| n.value().attr("href"))
            .map(canonicalize_ddg_href)
            .unwrap_or_default();
        let snippet = row
            .select(&snippet_sel)
            .next()
            .map(|n| n.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        if !url.is_empty() && !title.is_empty() {
            hits.push(SearchHit {
                title,
                snippet,
                url,
            });
        }
    }
    hits
}

/// DDG wraps result links in a redirector
/// (`/l/?uddg=<percent-encoded-url>`). Pull the real URL out
/// when we see that shape; pass through otherwise.
fn canonicalize_ddg_href(href: &str) -> String {
    if let Some(idx) = href.find("uddg=") {
        let encoded = &href[idx + 5..];
        let cut = encoded.find('&').map(|n| &encoded[..n]).unwrap_or(encoded);
        if let Some(decoded) = percent_decode(cut) {
            return decoded;
        }
    }
    if href.starts_with("//") {
        return format!("https:{href}");
    }
    href.to_string()
}

fn percent_decode(s: &str) -> Option<String> {
    let mut out = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16)?;
            let lo = (bytes[i + 2] as char).to_digit(16)?;
            out.push(((hi << 4) | lo) as u8);
            i += 3;
        } else if b == b'+' {
            out.push(b' ');
            i += 1;
        } else {
            out.push(b);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ddg_html_extracts_title_snippet_url() {
        let html = include_str!("../../../tests/fixtures/ddg_serp.html");
        let hits = parse_ddg_html(html, 5);
        assert!(
            !hits.is_empty(),
            "fixture must produce at least one hit; got {hits:?}"
        );
        let first = &hits[0];
        assert!(first.title.contains("Argo"));
        assert!(first.url.starts_with("https://"));
        assert!(!first.snippet.is_empty());
    }

    #[test]
    fn parse_ddg_html_resolves_redirector_links() {
        let html = r##"<div class="result">
                <a class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fpage&amp;rut=abc">
                    Example title</a>
                <a class="result__snippet">A short snippet.</a>
            </div>"##;
        let hits = parse_ddg_html(html, 5);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].url, "https://example.com/page");
        assert_eq!(hits[0].title, "Example title");
    }

    #[test]
    fn parse_ddg_html_returns_empty_on_drift() {
        // Garbage HTML — no result rows. We MUST return an
        // empty vec, not panic, so a SERP layout change
        // degrades gracefully (no results) rather than crashing
        // the worker.
        let hits = parse_ddg_html("<html><body>nothing here</body></html>", 5);
        assert!(hits.is_empty());
    }

    #[test]
    fn search_backend_default_is_duckduckgo_when_no_keys() {
        // Detection runs against process env; in test runs we
        // ensure neither key is set so the default selects.
        let saved_t = std::env::var("TAVILY_API_KEY").ok();
        let saved_b = std::env::var("BRAVE_API_KEY").ok();
        let saved_b2 = std::env::var("BRAVE_SEARCH_API_KEY").ok();
        std::env::remove_var("TAVILY_API_KEY");
        std::env::remove_var("BRAVE_API_KEY");
        std::env::remove_var("BRAVE_SEARCH_API_KEY");
        assert_eq!(SearchBackend::detect(), SearchBackend::DuckDuckGoHtml);
        if let Some(v) = saved_t {
            std::env::set_var("TAVILY_API_KEY", v);
        }
        if let Some(v) = saved_b {
            std::env::set_var("BRAVE_API_KEY", v);
        }
        if let Some(v) = saved_b2 {
            std::env::set_var("BRAVE_SEARCH_API_KEY", v);
        }
    }
}
