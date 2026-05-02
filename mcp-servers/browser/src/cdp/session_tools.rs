//! Session-aware browser-tool handlers — every tool in the
//! `session_*` family except the lifecycle (open/close/list)
//! and the per-session WS helpers, which stay in
//! `sessions.rs`. Lifted out so `sessions.rs` keeps under the
//! 500-LOC ceiling as more session variants land.
//!
//! All handlers in here go through one of:
//!   - `super::sessions::runtime_evaluate(sid, expression)` —
//!     for tools that drive `Runtime.evaluate` (eval,
//!     read_page, click, fill, submit, extract_links, scroll,
//!     wait_for).
//!   - `super::sessions::send_cdp_method(sid, method, params)`
//!     — for tools that drive other CDP methods (Page.*,
//!     Network.*, etc — navigate, screenshot, pdf, cookies).
//!
//! Returns mirror the single-shot variants in dom.rs / forms.rs
//! / mod.rs so a host can swap between modes without changing
//! response-handling code.
use super::sessions::{ok_result, send_cdp_method, runtime_evaluate};
use argo_mcp::{CallToolResult, McpContent};

fn error(text: String) -> CallToolResult {
    CallToolResult {
        content: vec![McpContent::Text { text }],
        is_error: true,
    }
}

fn ok(text: String) -> CallToolResult {
    ok_result(text)
}

pub async fn session_navigate(args: &serde_json::Value) -> CallToolResult {
    let Some(sid) = args.get("session_id").and_then(|v| v.as_str()) else {
        return error("missing required field 'session_id'".into());
    };
    let Some(url) = args.get("url").and_then(|v| v.as_str()) else {
        return error("missing required field 'url'".into());
    };
    match send_cdp_method(sid, "Page.navigate", serde_json::json!({"url": url})).await {
        Ok(parsed) => {
            if let Some(err_msg) = parsed
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|v| v.as_str())
            {
                return error(format!("Page.navigate: {err_msg}"));
            }
            let frame_id = parsed["result"]["frameId"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let payload = serde_json::json!({
                "session_id": sid,
                "navigated_to": url,
                "frame_id": frame_id,
            });
            ok(serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()))
        }
        Err(e) => error(e),
    }
}

pub async fn session_eval(args: &serde_json::Value) -> CallToolResult {
    let Some(sid) = args.get("session_id").and_then(|v| v.as_str()) else {
        return error("missing required field 'session_id'".into());
    };
    let Some(expression) = args.get("expression").and_then(|v| v.as_str()) else {
        return error("missing required field 'expression'".into());
    };
    match runtime_evaluate(sid, expression).await {
        Ok(parsed) => {
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
            ok(summary)
        }
        Err(e) => error(e),
    }
}

pub async fn session_read_page(args: &serde_json::Value) -> CallToolResult {
    let Some(sid) = args.get("session_id").and_then(|v| v.as_str()) else {
        return error("missing required field 'session_id'".into());
    };
    match runtime_evaluate(sid, "document.body ? document.body.innerText : ''").await {
        Ok(parsed) => {
            let value = parsed["result"]["result"]["value"]
                .as_str()
                .map(String::from)
                .unwrap_or_default();
            ok(value)
        }
        Err(e) => error(e),
    }
}

pub async fn session_click(args: &serde_json::Value) -> CallToolResult {
    let Some(sid) = args.get("session_id").and_then(|v| v.as_str()) else {
        return error("missing required field 'session_id'".into());
    };
    let Some(selector) = args.get("selector").and_then(|v| v.as_str()) else {
        return error("missing required field 'selector'".into());
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
    match runtime_evaluate(sid, &js).await {
        Ok(parsed) => {
            let value = parsed["result"]["result"]["value"]
                .as_str()
                .map(String::from)
                .unwrap_or_else(|| "(no value)".into());
            if value == "clicked" {
                ok(value)
            } else {
                error(value)
            }
        }
        Err(e) => error(e),
    }
}

pub async fn session_fill(args: &serde_json::Value) -> CallToolResult {
    let Some(sid) = args.get("session_id").and_then(|v| v.as_str()) else {
        return error("missing required field 'session_id'".into());
    };
    let Some(selector) = args.get("selector").and_then(|v| v.as_str()) else {
        return error("missing required field 'selector'".into());
    };
    let Some(value) = args.get("value").and_then(|v| v.as_str()) else {
        return error("missing required field 'value'".into());
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
    match runtime_evaluate(sid, &js).await {
        Ok(parsed) => {
            let result = parsed["result"]["result"]["value"]
                .as_str()
                .map(String::from)
                .unwrap_or_else(|| "(no value)".into());
            if result == "filled" {
                ok(result)
            } else {
                error(result)
            }
        }
        Err(e) => error(e),
    }
}

pub async fn session_submit(args: &serde_json::Value) -> CallToolResult {
    let Some(sid) = args.get("session_id").and_then(|v| v.as_str()) else {
        return error("missing required field 'session_id'".into());
    };
    let Some(selector) = args.get("selector").and_then(|v| v.as_str()) else {
        return error("missing required field 'selector'".into());
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
    match runtime_evaluate(sid, &js).await {
        Ok(parsed) => {
            let result = parsed["result"]["result"]["value"]
                .as_str()
                .map(String::from)
                .unwrap_or_else(|| "(no value)".into());
            if result == "submitted" {
                ok(result)
            } else {
                error(result)
            }
        }
        Err(e) => error(e),
    }
}

pub async fn session_extract_links(args: &serde_json::Value) -> CallToolResult {
    let Some(sid) = args.get("session_id").and_then(|v| v.as_str()) else {
        return error("missing required field 'session_id'".into());
    };
    let js = "JSON.stringify(Array.from(document.querySelectorAll('a[href]')).map(a => ({ href: a.href, text: (a.innerText || '').trim() })))";
    match runtime_evaluate(sid, js).await {
        Ok(parsed) => {
            let raw = parsed["result"]["result"]["value"]
                .as_str()
                .map(String::from)
                .unwrap_or_else(|| "[]".to_string());
            let parsed_arr: serde_json::Value =
                serde_json::from_str(&raw).unwrap_or(serde_json::Value::Array(Vec::new()));
            let pretty = serde_json::to_string_pretty(&parsed_arr).unwrap_or(raw);
            ok(pretty)
        }
        Err(e) => error(e),
    }
}

pub async fn session_scroll(args: &serde_json::Value) -> CallToolResult {
    let Some(sid) = args.get("session_id").and_then(|v| v.as_str()) else {
        return error("missing required field 'session_id'".into());
    };
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
    let expr = format!("{js}; ({{x: window.scrollX, y: window.scrollY}})");
    match runtime_evaluate(sid, &expr).await {
        Ok(parsed) => {
            let value = &parsed["result"]["result"]["value"];
            ok(serde_json::to_string(value).unwrap_or_else(|_| value.to_string()))
        }
        Err(e) => error(e),
    }
}

pub async fn session_wait_for(args: &serde_json::Value) -> CallToolResult {
    let Some(sid) = args.get("session_id").and_then(|v| v.as_str()) else {
        return error("missing required field 'session_id'".into());
    };
    let Some(selector) = args.get("selector").and_then(|v| v.as_str()) else {
        return error("missing required field 'selector'".into());
    };
    let timeout_ms = args
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(30_000);
    let escaped = selector.replace('\\', "\\\\").replace('"', "\\\"");
    let probe_js = format!("document.querySelector(\"{escaped}\") !== null");
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
    loop {
        match runtime_evaluate(sid, &probe_js).await {
            Ok(parsed) => {
                if parsed["result"]["result"]["value"]
                    .as_bool()
                    .unwrap_or(false)
                {
                    let elapsed = timeout_ms.saturating_sub(
                        deadline
                            .saturating_duration_since(std::time::Instant::now())
                            .as_millis() as u64,
                    );
                    return ok(format!("matched after {elapsed}ms"));
                }
            }
            Err(e) => return error(e),
        }
        if std::time::Instant::now() >= deadline {
            return error(format!(
                "timed out after {timeout_ms}ms waiting for selector '{selector}'"
            ));
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

pub async fn session_pdf(args: &serde_json::Value) -> CallToolResult {
    let Some(sid) = args.get("session_id").and_then(|v| v.as_str()) else {
        return error("missing required field 'session_id'".into());
    };
    match send_cdp_method(sid, "Page.printToPDF", serde_json::json!({})).await {
        Ok(parsed) => {
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
        Err(e) => error(e),
    }
}

pub async fn session_cookies(args: &serde_json::Value) -> CallToolResult {
    let Some(sid) = args.get("session_id").and_then(|v| v.as_str()) else {
        return error("missing required field 'session_id'".into());
    };
    match send_cdp_method(sid, "Network.getCookies", serde_json::json!({})).await {
        Ok(parsed) => {
            let cookies = &parsed["result"]["cookies"];
            let pretty =
                serde_json::to_string_pretty(cookies).unwrap_or_else(|_| cookies.to_string());
            ok(pretty)
        }
        Err(e) => error(e),
    }
}

pub async fn session_screenshot(args: &serde_json::Value) -> CallToolResult {
    let Some(sid) = args.get("session_id").and_then(|v| v.as_str()) else {
        return error("missing required field 'session_id'".into());
    };
    let full_page = args
        .get("full_page")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let params = serde_json::json!({
        "format": "png",
        "captureBeyondViewport": full_page,
    });
    match send_cdp_method(sid, "Page.captureScreenshot", params).await {
        Ok(parsed) => {
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
        Err(e) => error(e),
    }
}
