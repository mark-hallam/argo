//! CDP form / interaction tools — wait_for, cookies, click, fill,
//! submit. All single-shot WS round-trips against
//! `Runtime.evaluate` (or `Network.*` for cookies/network-idle).
//! Extracted from
//! `cdp/mod.rs` to keep that file under the 500-LOC budget.
use super::{cdp_round_trip, error, port_from_args};
use argo_mcp::{CallToolResult, McpContent};
use futures::{SinkExt, StreamExt};
use std::collections::HashSet;
use std::time::{Duration, Instant};
use tokio_tungstenite::tungstenite::Message;

/// Poll a CSS selector or wait for `Network.*` events to go idle.
pub async fn wait_for(args: &serde_json::Value) -> CallToolResult {
    let mode = args
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("selector");
    match mode {
        "selector" => wait_for_selector(args).await,
        "network_idle" | "network-idle" => wait_for_network_idle(args).await,
        other => error(format!(
            "unknown wait_for mode '{other}': use selector|network_idle"
        )),
    }
}

/// Poll `Runtime.evaluate(document.querySelector(selector) !== null)`
/// every 200ms until truthy or `timeout_ms` elapses. Each poll
/// reconnects (single-shot semantics, but multi-shot loop).
async fn wait_for_selector(args: &serde_json::Value) -> CallToolResult {
    let port = port_from_args(args);
    let selector = match args.get("selector").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return error("missing required field 'selector'".into()),
    };
    let timeout_ms = args
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(30_000);
    let escaped = selector.replace('\\', "\\\\").replace('"', "\\\"");
    let probe_js = format!("document.querySelector(\"{escaped}\") !== null");
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
    loop {
        let req = serde_json::json!({
            "id": 1,
            "method": "Runtime.evaluate",
            "params": { "expression": &probe_js, "returnByValue": true }
        });
        let parsed = match cdp_round_trip(port, req).await {
            Ok(p) => p,
            Err(e) => return error(e),
        };
        if parsed["result"]["result"]["value"]
            .as_bool()
            .unwrap_or(false)
        {
            return CallToolResult {
                content: vec![McpContent::Text {
                    text: format!(
                        "matched after {}ms",
                        timeout_ms.saturating_sub(
                            deadline
                                .saturating_duration_since(std::time::Instant::now())
                                .as_millis() as u64
                        )
                    ),
                }],
                is_error: false,
            };
        }
        if std::time::Instant::now() >= deadline {
            return error(format!(
                "timed out after {timeout_ms}ms waiting for selector '{selector}'"
            ));
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

async fn wait_for_network_idle(args: &serde_json::Value) -> CallToolResult {
    let port = port_from_args(args);
    let timeout_ms = args
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(30_000);
    let quiet_period_ms = args
        .get("quiet_period_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(500);
    match network_idle_wait(port, timeout_ms, quiet_period_ms).await {
        Ok(elapsed_ms) => CallToolResult {
            content: vec![McpContent::Text {
                text: format!(
                    "network idle after {elapsed_ms}ms (quiet_period_ms={quiet_period_ms})"
                ),
            }],
            is_error: false,
        },
        Err(e) => error(e),
    }
}

async fn network_idle_wait(
    port: u16,
    timeout_ms: u64,
    quiet_period_ms: u64,
) -> Result<u64, String> {
    let ws_url = first_page_ws_url(port).await?;
    let (mut ws, _resp) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .map_err(|e| format!("connect ws {ws_url}: {e}"))?;
    let enable = serde_json::json!({
        "id": 1,
        "method": "Network.enable",
        "params": {}
    });
    let serialized =
        serde_json::to_string(&enable).map_err(|e| format!("serialize CDP request: {e}"))?;
    ws.send(Message::Text(serialized))
        .await
        .map_err(|e| format!("send Network.enable: {e}"))?;

    let start = Instant::now();
    let deadline = start + Duration::from_millis(timeout_ms);
    let quiet = Duration::from_millis(quiet_period_ms);
    let mut state = NetworkIdleState::new(start);
    loop {
        let now = Instant::now();
        if state.is_idle_at(now, quiet) {
            let _ = ws.close(None).await;
            return Ok(now.saturating_duration_since(start).as_millis() as u64);
        }
        if now >= deadline {
            let _ = ws.close(None).await;
            return Err(format!(
                "timed out after {timeout_ms}ms waiting for network_idle \
                 (quiet_period_ms={quiet_period_ms}, in_flight={})",
                state.in_flight_len()
            ));
        }
        let wait_for = state.next_check_delay(now, quiet, deadline);
        match tokio::time::timeout(wait_for, ws.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => {
                let frame: serde_json::Value =
                    serde_json::from_str(&text).map_err(|e| format!("parse CDP frame: {e}"))?;
                state.observe(&frame, Instant::now());
            }
            Ok(Some(Ok(Message::Close(_)))) | Ok(None) => {
                return Err("WebSocket closed before network_idle".into());
            }
            Ok(Some(Ok(_))) => {}
            Ok(Some(Err(e))) => return Err(format!("read CDP frame: {e}")),
            Err(_) => {}
        }
    }
}

async fn first_page_ws_url(port: u16) -> Result<String, String> {
    let tabs = super::fetch_tabs(port).await?;
    tabs.iter()
        .find_map(|t| {
            t.get("type")
                .and_then(|v| v.as_str())
                .filter(|&ty| ty == "page")
                .and(t.get("webSocketDebuggerUrl").and_then(|v| v.as_str()))
        })
        .map(str::to_string)
        .ok_or_else(|| "no `page`-type tab in /json (open a tab in Chrome)".to_string())
}

struct NetworkIdleState {
    in_flight: HashSet<String>,
    zero_since: Option<Instant>,
}

impl NetworkIdleState {
    fn new(now: Instant) -> Self {
        Self {
            in_flight: HashSet::new(),
            zero_since: Some(now),
        }
    }

    fn observe(&mut self, frame: &serde_json::Value, now: Instant) {
        let Some(method) = frame.get("method").and_then(|v| v.as_str()) else {
            return;
        };
        match method {
            "Network.requestWillBeSent" => {
                if let Some(id) = request_id(frame) {
                    self.in_flight.insert(id);
                }
            }
            "Network.responseReceived" | "Network.loadingFinished" | "Network.loadingFailed" => {
                if let Some(id) = request_id(frame) {
                    self.in_flight.remove(&id);
                }
            }
            _ => {}
        }
        self.refresh_zero_since(now);
    }

    fn is_idle_at(&self, now: Instant, quiet: Duration) -> bool {
        self.zero_since
            .map(|zero| now.saturating_duration_since(zero) >= quiet)
            .unwrap_or(false)
    }

    fn next_check_delay(&self, now: Instant, quiet: Duration, deadline: Instant) -> Duration {
        let until_deadline = deadline.saturating_duration_since(now);
        let until_quiet = self
            .zero_since
            .map(|zero| (zero + quiet).saturating_duration_since(now))
            .unwrap_or(until_deadline);
        std::cmp::min(until_deadline, until_quiet)
    }

    fn in_flight_len(&self) -> usize {
        self.in_flight.len()
    }

    fn refresh_zero_since(&mut self, now: Instant) {
        if self.in_flight.is_empty() {
            if self.zero_since.is_none() {
                self.zero_since = Some(now);
            }
        } else {
            self.zero_since = None;
        }
    }
}

fn request_id(frame: &serde_json::Value) -> Option<String> {
    frame
        .get("params")
        .and_then(|p| p.get("requestId"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

/// Cookies via Network domain. `op = "get" | "set" | "clear"`.
pub async fn cookies(args: &serde_json::Value) -> CallToolResult {
    let port = port_from_args(args);
    let op = args.get("op").and_then(|v| v.as_str()).unwrap_or("get");
    match op {
        "get" => {
            let req = serde_json::json!({
                "id": 1,
                "method": "Network.getCookies",
                "params": {}
            });
            let parsed = match cdp_round_trip(port, req).await {
                Ok(p) => p,
                Err(e) => return error(e),
            };
            let cookies = &parsed["result"]["cookies"];
            let pretty =
                serde_json::to_string_pretty(cookies).unwrap_or_else(|_| cookies.to_string());
            CallToolResult {
                content: vec![McpContent::Text { text: pretty }],
                is_error: false,
            }
        }
        "set" => {
            let name = match args.get("name").and_then(|v| v.as_str()) {
                Some(n) => n.to_string(),
                None => return error("missing 'name' for op=set".into()),
            };
            let value = args
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let mut params = serde_json::json!({ "name": name, "value": value });
            for k in ["domain", "url", "path"] {
                if let Some(v) = args.get(k).and_then(|v| v.as_str()) {
                    params[k] = serde_json::Value::String(v.into());
                }
            }
            let req = serde_json::json!({
                "id": 1,
                "method": "Network.setCookie",
                "params": params,
            });
            let parsed = match cdp_round_trip(port, req).await {
                Ok(p) => p,
                Err(e) => return error(e),
            };
            let success = parsed["result"]["success"].as_bool().unwrap_or(false);
            CallToolResult {
                content: vec![McpContent::Text {
                    text: if success {
                        format!("set cookie '{name}'")
                    } else {
                        format!("setCookie returned success=false: {parsed}")
                    },
                }],
                is_error: !success,
            }
        }
        "clear" => {
            let req = serde_json::json!({
                "id": 1,
                "method": "Network.clearBrowserCookies",
                "params": {}
            });
            let parsed = match cdp_round_trip(port, req).await {
                Ok(p) => p,
                Err(e) => return error(e),
            };
            let _ = parsed;
            CallToolResult {
                content: vec![McpContent::Text {
                    text: "cleared all cookies".into(),
                }],
                is_error: false,
            }
        }
        other => error(format!("unknown op '{other}': use one of get|set|clear")),
    }
}

/// Click an element matched by a CSS selector via
/// `Runtime.evaluate(el.click())`. Returns 'clicked' on match,
/// 'no match for selector' otherwise.
pub async fn click(args: &serde_json::Value) -> CallToolResult {
    let port = port_from_args(args);
    let selector = match args.get("selector").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return error("missing required field 'selector'".into()),
    };
    let escaped = selector.replace('\\', "\\\\").replace('"', "\\\"");
    let js = format!(
        "(function(){{ \
            var el = document.querySelector(\"{escaped}\"); \
            if (!el) return 'no match for selector'; \
            el.click(); \
            return 'clicked'; \
        }})()"
    );
    let req = serde_json::json!({
        "id": 1,
        "method": "Runtime.evaluate",
        "params": { "expression": js, "returnByValue": true }
    });
    let parsed = match cdp_round_trip(port, req).await {
        Ok(p) => p,
        Err(e) => return error(e),
    };
    let value = parsed["result"]["result"]["value"]
        .as_str()
        .map(String::from)
        .unwrap_or_else(|| "(no value)".into());
    let is_err = value != "clicked";
    CallToolResult {
        content: vec![McpContent::Text { text: value }],
        is_error: is_err,
    }
}

#[cfg(test)]
#[path = "../wait_network_idle_tests.rs"]
mod wait_network_idle_tests;

/// Set the `value` of a form field (input/textarea/select) and
/// dispatch an `input` event so client-side form-handling JS sees
/// the change.
pub async fn fill(args: &serde_json::Value) -> CallToolResult {
    let port = port_from_args(args);
    let selector = match args.get("selector").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return error("missing required field 'selector'".into()),
    };
    let value = match args.get("value").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return error("missing required field 'value'".into()),
    };
    let esc_sel = selector.replace('\\', "\\\\").replace('"', "\\\"");
    let esc_val = value.replace('\\', "\\\\").replace('"', "\\\"");
    let js = format!(
        "(function(){{ \
            var el = document.querySelector(\"{esc_sel}\"); \
            if (!el) return 'no match for selector'; \
            el.value = \"{esc_val}\"; \
            el.dispatchEvent(new Event('input', {{ bubbles: true }})); \
            el.dispatchEvent(new Event('change', {{ bubbles: true }})); \
            return 'filled'; \
        }})()"
    );
    let req = serde_json::json!({
        "id": 1,
        "method": "Runtime.evaluate",
        "params": { "expression": js, "returnByValue": true }
    });
    let parsed = match cdp_round_trip(port, req).await {
        Ok(p) => p,
        Err(e) => return error(e),
    };
    let result = parsed["result"]["result"]["value"]
        .as_str()
        .map(String::from)
        .unwrap_or_else(|| "(no value)".into());
    let is_err = result != "filled";
    CallToolResult {
        content: vec![McpContent::Text { text: result }],
        is_error: is_err,
    }
}

/// Submit a form on the first page-type tab. CSS selector is
/// required and must match a `<form>` (or any element whose
/// nearest enclosing form will be submitted).
pub async fn submit(args: &serde_json::Value) -> CallToolResult {
    let port = port_from_args(args);
    let selector = match args.get("selector").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return error("missing required field 'selector'".into()),
    };
    let escaped = selector.replace('\\', "\\\\").replace('"', "\\\"");
    let js = format!(
        "(function(){{ \
            var el = document.querySelector(\"{escaped}\"); \
            if (!el) return 'no match for selector'; \
            var form = el.tagName === 'FORM' ? el : el.form; \
            if (!form) return 'matched element has no enclosing <form>'; \
            form.submit(); \
            return 'submitted'; \
        }})()"
    );
    let req = serde_json::json!({
        "id": 1,
        "method": "Runtime.evaluate",
        "params": {
            "expression": js,
            "returnByValue": true,
        }
    });
    let parsed = match cdp_round_trip(port, req).await {
        Ok(p) => p,
        Err(e) => return error(e),
    };
    let value = parsed["result"]["result"]["value"]
        .as_str()
        .map(String::from)
        .unwrap_or_else(|| "(no value)".into());
    let is_err = value != "submitted";
    CallToolResult {
        content: vec![McpContent::Text { text: value }],
        is_error: is_err,
    }
}
