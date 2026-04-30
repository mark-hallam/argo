//! Chrome DevTools Protocol — HTTP-only operations.
//!
//! The full CDP requires WebSocket (Runtime.evaluate, DOM.*, etc.),
//! which is what chromiumoxide drives. This module covers just the
//! HTTP-only subset (`/json`, `/json/version`) — useful to verify
//! the user's Chrome attach config before the real driver lands.
use argo_mcp::{CallToolResult, McpContent};
use std::time::Duration;

const DEFAULT_PORT: u16 = 9222;

/// `GET http://127.0.0.1:<port>/json` and return the tab list as a
/// JSON content block. Failure modes (Chrome not running, port not
/// open, bad response) become `is_error = true` Text content blocks.
pub async fn list_tabs(args: &serde_json::Value) -> CallToolResult {
    let port = args
        .get("port")
        .and_then(|v| v.as_u64())
        .and_then(|n| u16::try_from(n).ok())
        .unwrap_or(DEFAULT_PORT);
    let url = format!("http://127.0.0.1:{port}/json");
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return error(format!("build http client: {e}"));
        }
    };
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            return error(format!(
                "no Chrome on port {port} ({e}); start Chrome with `--remote-debugging-port={port}`"
            ));
        }
    };
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return error(format!("/json returned {status}: {body}"));
    }
    let body = match resp.text().await {
        Ok(b) => b,
        Err(e) => return error(format!("read /json body: {e}")),
    };
    let parsed: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => return error(format!("parse /json: {e}")),
    };
    // Trim each entry to the fields callers care about. Full payload
    // is large + redundant.
    let trimmed = trim_tabs(&parsed);
    let serialized = match serde_json::to_string_pretty(&trimmed) {
        Ok(s) => s,
        Err(e) => return error(format!("re-serialize: {e}")),
    };
    CallToolResult {
        content: vec![McpContent::Text { text: serialized }],
        is_error: false,
    }
}

/// `GET http://127.0.0.1:<port>/json/version` and return the
/// User-Agent + ProtocolVersion + browser WebSocket URL as JSON.
pub async fn chrome_version(args: &serde_json::Value) -> CallToolResult {
    let port = args
        .get("port")
        .and_then(|v| v.as_u64())
        .and_then(|n| u16::try_from(n).ok())
        .unwrap_or(DEFAULT_PORT);
    let url = format!("http://127.0.0.1:{port}/json/version");
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(e) => return error(format!("build http client: {e}")),
    };
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            return error(format!(
                "no Chrome on port {port} ({e}); start Chrome with `--remote-debugging-port={port}`"
            ));
        }
    };
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return error(format!("/json/version returned {status}: {body}"));
    }
    let body = match resp.text().await {
        Ok(b) => b,
        Err(e) => return error(format!("read /json/version body: {e}")),
    };
    // Pass the body through verbatim (after re-pretty-printing for
    // readability). Fields are well-defined: Browser, Protocol-Version,
    // User-Agent, V8-Version, WebKit-Version, webSocketDebuggerUrl.
    let parsed: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => return error(format!("parse /json/version: {e}")),
    };
    let serialized = match serde_json::to_string_pretty(&parsed) {
        Ok(s) => s,
        Err(e) => return error(format!("re-serialize: {e}")),
    };
    CallToolResult {
        content: vec![McpContent::Text { text: serialized }],
        is_error: false,
    }
}

fn trim_tabs(parsed: &serde_json::Value) -> serde_json::Value {
    let Some(arr) = parsed.as_array() else {
        return parsed.clone();
    };
    let trimmed: Vec<serde_json::Value> = arr
        .iter()
        .map(|e| {
            serde_json::json!({
                "id": e.get("id"),
                "type": e.get("type"),
                "title": e.get("title"),
                "url": e.get("url"),
                "webSocketDebuggerUrl": e.get("webSocketDebuggerUrl"),
            })
        })
        .collect();
    serde_json::Value::Array(trimmed)
}

fn error(text: String) -> CallToolResult {
    CallToolResult {
        content: vec![McpContent::Text { text }],
        is_error: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_tabs_keeps_relevant_fields() {
        let raw = serde_json::json!([{
            "id": "abc",
            "type": "page",
            "title": "Example",
            "url": "https://example.com/",
            "webSocketDebuggerUrl": "ws://localhost:9222/devtools/page/abc",
            "description": "...",
            "devtoolsFrontendUrl": "...",
        }]);
        let trimmed = trim_tabs(&raw);
        let arr = trimmed.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        let e = &arr[0];
        assert_eq!(e["id"], "abc");
        assert_eq!(e["type"], "page");
        assert_eq!(e["title"], "Example");
        assert_eq!(e["url"], "https://example.com/");
        assert_eq!(
            e["webSocketDebuggerUrl"],
            "ws://localhost:9222/devtools/page/abc"
        );
        // Trimmed-out fields are absent.
        assert!(e.get("description").is_none());
        assert!(e.get("devtoolsFrontendUrl").is_none());
    }

    fn pick_free_port() -> u16 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        port
    }

    #[tokio::test]
    async fn list_tabs_with_no_chrome_returns_clean_error() {
        let port = pick_free_port();
        let args = serde_json::json!({"port": port});
        let result = list_tabs(&args).await;
        assert!(result.is_error);
        match &result.content[0] {
            McpContent::Text { text } => {
                assert!(
                    text.contains("no Chrome on port") || text.contains("/json"),
                    "unexpected error text: {text}"
                );
            }
            _ => panic!("expected text content"),
        }
    }

    #[tokio::test]
    async fn chrome_version_with_no_chrome_returns_clean_error() {
        let port = pick_free_port();
        let args = serde_json::json!({"port": port});
        let result = chrome_version(&args).await;
        assert!(result.is_error);
        match &result.content[0] {
            McpContent::Text { text } => {
                assert!(
                    text.contains("no Chrome on port") || text.contains("/json/version"),
                    "unexpected error text: {text}"
                );
            }
            _ => panic!("expected text content"),
        }
    }
}
