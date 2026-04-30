//! CDP form / interaction tools — wait_for, cookies, click, fill,
//! submit. All single-shot WS round-trips against
//! `Runtime.evaluate` (or `Network.*` for cookies). Extracted from
//! `cdp/mod.rs` to keep that file under the 500-LOC budget.
use super::{cdp_round_trip, error, port_from_args};
use argo_mcp::{CallToolResult, McpContent};

/// Poll `Runtime.evaluate(document.querySelector(selector) !== null)`
/// every 200ms until truthy or `timeout_ms` elapses. Each poll
/// reconnects (single-shot semantics, but multi-shot loop).
pub async fn wait_for(args: &serde_json::Value) -> CallToolResult {
    let port = port_from_args(args);
    let selector = match args.get("selector").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => {
            return error(
                "missing required field 'selector' (network_idle wait not yet wired)".into(),
            )
        }
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
