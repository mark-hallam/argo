//! CDP DOM-read tools — eval, read_page, extract_links. All
//! single-shot WS round-trips against `Runtime.evaluate`.
//! Extracted from `cdp/mod.rs` to keep that file under the 500-LOC
//! budget.
use super::{cdp_round_trip, error, port_from_args};
use argo_mcp::{CallToolResult, McpContent};

/// Evaluate a JS expression on the first page-type tab via
/// `Runtime.evaluate`. Returns `result.value` (or `description` for
/// errors / non-serializable values) as a Text content block.
pub async fn eval(args: &serde_json::Value) -> CallToolResult {
    let port = port_from_args(args);
    let expression = match args.get("expression").and_then(|v| v.as_str()) {
        Some(e) => e.to_string(),
        None => return error("missing required field 'expression'".into()),
    };
    let req = serde_json::json!({
        "id": 1,
        "method": "Runtime.evaluate",
        "params": {
            "expression": expression,
            "returnByValue": true,
        }
    });
    let parsed = match cdp_round_trip(port, req).await {
        Ok(p) => p,
        Err(e) => return error(e),
    };
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

/// Return all `<a href>` links on the first open page as a JSON
/// array of `{href, text}` objects. Single-shot `Runtime.evaluate`.
pub async fn extract_links(args: &serde_json::Value) -> CallToolResult {
    let port = port_from_args(args);
    let req = serde_json::json!({
        "id": 1,
        "method": "Runtime.evaluate",
        "params": {
            "expression": "JSON.stringify(Array.from(document.querySelectorAll('a[href]')).map(a => ({ href: a.href, text: (a.innerText || '').trim() })))",
            "returnByValue": true,
        }
    });
    let parsed = match cdp_round_trip(port, req).await {
        Ok(p) => p,
        Err(e) => return error(e),
    };
    let raw = parsed["result"]["result"]["value"]
        .as_str()
        .map(String::from)
        .unwrap_or_else(|| "[]".to_string());
    let parsed_arr: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or(serde_json::Value::Array(Vec::new()));
    let pretty = serde_json::to_string_pretty(&parsed_arr).unwrap_or(raw);
    CallToolResult {
        content: vec![McpContent::Text { text: pretty }],
        is_error: false,
    }
}

/// Return `document.body.innerText` of the first open page via
/// `Runtime.evaluate`. Convenience wrapper around `eval` for the
/// common "give me the page text" case.
pub async fn read_page(args: &serde_json::Value) -> CallToolResult {
    let port = port_from_args(args);
    let req = serde_json::json!({
        "id": 1,
        "method": "Runtime.evaluate",
        "params": {
            "expression": "document.body ? document.body.innerText : ''",
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
        .unwrap_or_default();
    CallToolResult {
        content: vec![McpContent::Text { text: value }],
        is_error: false,
    }
}
