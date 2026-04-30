//! Chrome DevTools Protocol — minimal client.
//!
//! Two paths:
//!   - HTTP probes (`list_tabs`, `chrome_version`) hit `/json` +
//!     `/json/version`; useful to verify the user's Chrome attach
//!     config before driving anything.
//!   - WebSocket (`eval`) opens the first page's
//!     `webSocketDebuggerUrl`, sends `Runtime.evaluate`, and returns
//!     the result. Single-shot — no session caching across calls.
//!
//! The 11 remaining spec tools (navigate, click, fill, submit,
//! screenshot, read_page, extract_links, wait_for, scroll, cookies,
//! pdf) need richer state (DOM nodes, frame stacks, cookies) and
//! land with chromiumoxide in a follow-up.
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

/// `GET http://127.0.0.1:<port>/json` and return the parsed array
/// of tabs. Internal helper for `eval`.
async fn fetch_tabs(port: u16) -> Result<Vec<serde_json::Value>, String> {
    let url = format!("http://127.0.0.1:{port}/json");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|e| format!("build http client: {e}"))?;
    let resp = client.get(&url).send().await.map_err(|e| {
        format!(
            "no Chrome on port {port} ({e}); start Chrome with `--remote-debugging-port={port}`"
        )
    })?;
    if !resp.status().is_success() {
        return Err(format!("/json returned {}", resp.status()));
    }
    let body = resp.text().await.map_err(|e| format!("read body: {e}"))?;
    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("parse /json: {e}"))?;
    parsed
        .as_array()
        .cloned()
        .ok_or_else(|| "/json did not return a JSON array".into())
}

/// Run `Runtime.evaluate` on the first available page via raw CDP
/// over WebSocket. Returns the `result.value` (or `result.description`
/// if no value, e.g. for thrown errors).
pub async fn eval(args: &serde_json::Value) -> CallToolResult {
    use futures::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    let port = args
        .get("port")
        .and_then(|v| v.as_u64())
        .and_then(|n| u16::try_from(n).ok())
        .unwrap_or(DEFAULT_PORT);
    let expression = match args.get("expression").and_then(|v| v.as_str()) {
        Some(e) => e.to_string(),
        None => return error("missing required field 'expression'".into()),
    };
    let tabs = match fetch_tabs(port).await {
        Ok(t) => t,
        Err(e) => return error(e),
    };
    let ws_url = tabs
        .iter()
        .find_map(|t| {
            t.get("type")
                .and_then(|v| v.as_str())
                .filter(|&ty| ty == "page")
                .and(t.get("webSocketDebuggerUrl").and_then(|v| v.as_str()))
        })
        .map(str::to_string);
    let Some(ws_url) = ws_url else {
        return error("no `page`-type tab in /json (open a tab in Chrome)".into());
    };
    let (mut ws, _resp) = match tokio_tungstenite::connect_async(&ws_url).await {
        Ok(p) => p,
        Err(e) => return error(format!("connect ws {ws_url}: {e}")),
    };
    let req = serde_json::json!({
        "id": 1,
        "method": "Runtime.evaluate",
        "params": {
            "expression": expression,
            "returnByValue": true,
        }
    });
    let serialized = match serde_json::to_string(&req) {
        Ok(s) => s,
        Err(e) => return error(format!("serialize CDP request: {e}")),
    };
    if let Err(e) = ws.send(Message::Text(serialized)).await {
        return error(format!("send CDP request: {e}"));
    }
    let response_text = loop {
        match ws.next().await {
            Some(Ok(Message::Text(t))) => break t.to_string(),
            Some(Ok(Message::Close(_))) | None => {
                return error("WebSocket closed before response".into());
            }
            Some(Ok(_)) => continue, // ignore Ping/Pong/Binary
            Some(Err(e)) => return error(format!("read CDP frame: {e}")),
        }
    };
    let _ = ws.close(None).await;
    let parsed: serde_json::Value = match serde_json::from_str(&response_text) {
        Ok(v) => v,
        Err(e) => return error(format!("parse CDP response: {e}")),
    };
    if let Some(err) = parsed.get("error") {
        return error(format!("CDP error: {err}"));
    }
    let result = &parsed["result"]["result"];
    let summary = result
        .get("value")
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| v.to_string()))
        .or_else(|| {
            result
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from)
        })
        .unwrap_or_else(|| result.to_string());
    CallToolResult {
        content: vec![McpContent::Text { text: summary }],
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
    async fn eval_with_no_chrome_returns_clean_error() {
        let port = pick_free_port();
        let args = serde_json::json!({"port": port, "expression": "1 + 1"});
        let result = eval(&args).await;
        assert!(result.is_error);
        match &result.content[0] {
            McpContent::Text { text } => {
                assert!(
                    text.contains("no Chrome on port") || text.contains("/json"),
                    "unexpected text: {text}"
                );
            }
            _ => panic!("expected text content"),
        }
    }

    #[tokio::test]
    async fn eval_missing_expression_returns_field_error() {
        let port = pick_free_port();
        let args = serde_json::json!({"port": port});
        let result = eval(&args).await;
        assert!(result.is_error);
        match &result.content[0] {
            McpContent::Text { text } => assert!(text.contains("missing required field")),
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
