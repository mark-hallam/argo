//! Chrome DevTools Protocol — minimal client. Three paths:
//! HTTP probes (list_tabs / chrome_version) hit /json + /json/version;
//! single-shot WS calls go through the shared `cdp_round_trip()`
//! helper; polling loops (wait_for) repeat the single-shot probe
//! until truthy or timeout. Stateful sessions live in
//! sessions.rs + session_tools.rs (persistent WS, no /json round-
//! trip per call).
mod dom;
mod forms;
mod session_tools;
mod sessions;

pub use dom::{eval, extract_links, read_page};
pub use forms::{click, cookies, fill, submit, wait_for};
pub use session_tools::{
    session_click, session_cookies, session_eval, session_extract_links, session_fill,
    session_get_url, session_navigate, session_pdf, session_read_page,
    session_screenshot, session_scroll, session_submit, session_wait_for,
};
pub use sessions::{session_close, session_list, session_open, session_reap};

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

pub(super) async fn fetch_tabs(port: u16) -> Result<Vec<serde_json::Value>, String> {
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

/// Open `url` in the first page-type tab via `Page.navigate`.
/// Returns the frame id from the response (handy if the caller
/// wants to correlate the load with subsequent CDP events). Does
/// NOT wait for `Page.frameStoppedLoading` — that's the agent
/// loop's job.
pub async fn navigate(args: &serde_json::Value) -> CallToolResult {
    let port = port_from_args(args);
    let url = match args.get("url").and_then(|v| v.as_str()) {
        Some(u) => u.to_string(),
        None => return error("missing required field 'url'".into()),
    };
    let req = serde_json::json!({
        "id": 1,
        "method": "Page.navigate",
        "params": { "url": url }
    });
    let parsed = match cdp_round_trip(port, req).await {
        Ok(p) => p,
        Err(e) => return error(e),
    };
    let frame_id = parsed["result"]["frameId"]
        .as_str()
        .map(String::from)
        .unwrap_or_else(|| "<unknown>".into());
    if let Some(err) = parsed["result"].get("errorText").and_then(|v| v.as_str()) {
        if !err.is_empty() {
            return error(format!("navigate failed: {err}"));
        }
    }
    CallToolResult {
        content: vec![McpContent::Text {
            text: format!("navigated to {url} (frameId={frame_id})"),
        }],
        is_error: false,
    }
}

/// Scroll the first page-type tab via Runtime.evaluate.
///
/// `direction = "up" | "down" | "to-top" | "to-bottom"`. The
/// "up"/"down" variants take an optional `pixels` (default 500).
pub async fn scroll(args: &serde_json::Value) -> CallToolResult {
    let port = port_from_args(args);
    let direction = args
        .get("direction")
        .and_then(|v| v.as_str())
        .unwrap_or("down");
    let pixels = args.get("pixels").and_then(|v| v.as_i64()).unwrap_or(500);
    let js = match direction {
        "up" => format!("window.scrollBy(0, -{pixels})"),
        "down" => format!("window.scrollBy(0, {pixels})"),
        "to-top" => "window.scrollTo(0, 0)".to_string(),
        "to-bottom" => "window.scrollTo(0, document.body.scrollHeight)".to_string(),
        other => {
            return error(format!(
                "unknown direction '{other}': use one of up|down|to-top|to-bottom"
            ));
        }
    };
    let req = serde_json::json!({
        "id": 1,
        "method": "Runtime.evaluate",
        "params": {
            "expression": format!("{js}; ({{x: window.scrollX, y: window.scrollY}})"),
            "returnByValue": true,
        }
    });
    let parsed = match cdp_round_trip(port, req).await {
        Ok(p) => p,
        Err(e) => return error(e),
    };
    let value = &parsed["result"]["result"]["value"];
    let summary = serde_json::to_string(value).unwrap_or_else(|_| value.to_string());
    CallToolResult {
        content: vec![McpContent::Text { text: summary }],
        is_error: false,
    }
}

/// Return all `<a href>` links on the first open page as a JSON
/// array of `{href, text}` objects. Single-shot `Runtime.evaluate`.
pub async fn pdf(args: &serde_json::Value) -> CallToolResult {
    let port = port_from_args(args);
    let req = serde_json::json!({
        "id": 1,
        "method": "Page.printToPDF",
        "params": {}
    });
    let parsed = match cdp_round_trip(port, req).await {
        Ok(p) => p,
        Err(e) => return error(e),
    };
    let data = parsed["result"]["data"].as_str().map(String::from);
    let Some(data) = data else {
        return error(format!("no `data` in printToPDF response: {parsed}"));
    };
    CallToolResult {
        content: vec![McpContent::Image {
            data: Some(format!("data:application/pdf;base64,{data}")),
            mime_type: Some("application/pdf".into()),
        }],
        is_error: false,
    }
}

/// Capture a viewport (or full-page) screenshot via
/// `Page.captureScreenshot`. Returns the base64 PNG payload as an
/// MCP `Image` content block.
pub async fn screenshot(args: &serde_json::Value) -> CallToolResult {
    let port = port_from_args(args);
    let full_page = args
        .get("full_page")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let req = serde_json::json!({
        "id": 1,
        "method": "Page.captureScreenshot",
        "params": {
            "format": "png",
            "captureBeyondViewport": full_page,
        }
    });
    let parsed = match cdp_round_trip(port, req).await {
        Ok(p) => p,
        Err(e) => return error(e),
    };
    let data = parsed["result"]["data"].as_str().map(String::from);
    let Some(data) = data else {
        return error(format!("no `data` in screenshot response: {parsed}"));
    };
    CallToolResult {
        content: vec![McpContent::Image {
            data: Some(format!("data:image/png;base64,{data}")),
            mime_type: Some("image/png".into()),
        }],
        is_error: false,
    }
}

pub(super) fn port_from_args(args: &serde_json::Value) -> u16 {
    args.get("port")
        .and_then(|v| v.as_u64())
        .and_then(|n| u16::try_from(n).ok())
        .unwrap_or(DEFAULT_PORT)
}

/// Open a WS to the first page-type tab on `port`, send `request`,
/// return the parsed response body. Single-shot.
pub(super) async fn cdp_round_trip(
    port: u16,
    request: serde_json::Value,
) -> Result<serde_json::Value, String> {
    use futures::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    let tabs = fetch_tabs(port).await?;
    let ws_url = tabs
        .iter()
        .find_map(|t| {
            t.get("type")
                .and_then(|v| v.as_str())
                .filter(|&ty| ty == "page")
                .and(t.get("webSocketDebuggerUrl").and_then(|v| v.as_str()))
        })
        .map(str::to_string)
        .ok_or_else(|| "no `page`-type tab in /json (open a tab in Chrome)".to_string())?;
    let (mut ws, _resp) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .map_err(|e| format!("connect ws {ws_url}: {e}"))?;
    let serialized =
        serde_json::to_string(&request).map_err(|e| format!("serialize CDP request: {e}"))?;
    ws.send(Message::Text(serialized))
        .await
        .map_err(|e| format!("send CDP request: {e}"))?;
    let response_text = loop {
        match ws.next().await {
            Some(Ok(Message::Text(t))) => break t.to_string(),
            Some(Ok(Message::Close(_))) | None => {
                return Err("WebSocket closed before response".into());
            }
            Some(Ok(_)) => continue, // ignore Ping/Pong/Binary
            Some(Err(e)) => return Err(format!("read CDP frame: {e}")),
        }
    };
    let _ = ws.close(None).await;
    let parsed: serde_json::Value =
        serde_json::from_str(&response_text).map_err(|e| format!("parse CDP response: {e}"))?;
    if let Some(err) = parsed.get("error") {
        return Err(format!("CDP error: {err}"));
    }
    Ok(parsed)
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

pub(super) fn error(text: String) -> CallToolResult {
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
    async fn screenshot_with_no_chrome_returns_clean_error() {
        let port = pick_free_port();
        let args = serde_json::json!({"port": port});
        let result = screenshot(&args).await;
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
