//! Browserbase cloud-browser integration. Opt-in via the
//! `browserbase` feature flag (#642).
//!
//! Architecture: Browserbase hosts a Chromium instance and gives
//! you back a CDP WebSocket URL. From there, the existing
//! `cdp::sessions` manager can drive it like any local browser —
//! navigate / click / read_page / etc all work unchanged. The new
//! work in this module is just the lifecycle:
//!
//! - `POST https://api.browserbase.com/v1/sessions` → create a
//!   hosted session, get back `{id, connectUrl}`.
//! - `DELETE https://api.browserbase.com/v1/sessions/<id>` → tear
//!   it down (also auto-expires server-side after timeout).
//!
//! Configuration (when feature is enabled):
//!   - `BROWSERBASE_API_KEY` (required) — sign up at
//!     <https://browserbase.com>; free tier covers ~60 min/month.
//!   - `BROWSERBASE_PROJECT_ID` (required) — found on the
//!     Browserbase dashboard.
//!
//! Without `--features browserbase`, the path returns a clear
//! "rebuild with feature" error so callers know the path forward.
//! Without the env vars, returns a "set $BROWSERBASE_API_KEY"
//! error.
use argo_mcp::{CallToolResult, McpContent};

#[cfg(feature = "browserbase")]
fn err(text: String) -> CallToolResult {
    CallToolResult {
        content: vec![McpContent::Text { text }],
        is_error: true,
    }
}

/// `cloud_browserbase_open_session` tool — create a hosted
/// Chromium session via Browserbase. Returns the session id and
/// the CDP WebSocket URL — the caller (or downstream tools) can
/// then drive it like any local browser.
#[cfg(feature = "browserbase")]
pub async fn open_session() -> CallToolResult {
    let api_key = match std::env::var("BROWSERBASE_API_KEY") {
        Ok(k) if !k.trim().is_empty() => k,
        _ => {
            return err(
                "BROWSERBASE_API_KEY is not set. Sign up at https://browserbase.com and \
                 export the API key from your dashboard."
                    .to_string(),
            );
        }
    };
    let project_id = match std::env::var("BROWSERBASE_PROJECT_ID") {
        Ok(p) if !p.trim().is_empty() => p,
        _ => {
            return err(
                "BROWSERBASE_PROJECT_ID is not set. Look it up on your Browserbase \
                 dashboard and export it alongside BROWSERBASE_API_KEY."
                    .to_string(),
            );
        }
    };

    let body = serde_json::json!({ "projectId": project_id });
    let client = reqwest::Client::new();
    let resp = match client
        .post("https://api.browserbase.com/v1/sessions")
        .header("X-BB-API-Key", &api_key)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return err(format!("browserbase POST /v1/sessions: {e}")),
    };
    let status = resp.status();
    let body_text = resp
        .text()
        .await
        .unwrap_or_else(|e| format!("(failed to read response body: {e})"));
    if !status.is_success() {
        return err(format!(
            "browserbase POST /v1/sessions returned {status}: {body_text}"
        ));
    }
    let parsed: serde_json::Value =
        serde_json::from_str(&body_text).unwrap_or(serde_json::Value::Null);
    let session_id = parsed
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let connect_url = parsed
        .get("connectUrl")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if session_id.is_empty() || connect_url.is_empty() {
        return err(format!(
            "browserbase response missing id/connectUrl: {body_text}"
        ));
    }
    let payload = serde_json::json!({
        "session_id": session_id,
        "connect_url": connect_url,
        "hint": "use the connect_url with the existing session_open / session_navigate tools",
    });
    CallToolResult {
        content: vec![McpContent::Text {
            text: serde_json::to_string(&payload).unwrap_or_default(),
        }],
        is_error: false,
    }
}

/// `cloud_browserbase_close_session` tool — tear down a hosted
/// session. Idempotent at the Browserbase end; safe to call
/// multiple times.
#[cfg(feature = "browserbase")]
pub async fn close_session(session_id: &str) -> CallToolResult {
    let api_key = match std::env::var("BROWSERBASE_API_KEY") {
        Ok(k) if !k.trim().is_empty() => k,
        _ => return err("BROWSERBASE_API_KEY is not set.".to_string()),
    };
    if session_id.trim().is_empty() {
        return err("session_id argument is required".to_string());
    }
    let url = format!("https://api.browserbase.com/v1/sessions/{session_id}");
    let client = reqwest::Client::new();
    let resp = match client.delete(&url).header("X-BB-API-Key", &api_key).send().await {
        Ok(r) => r,
        Err(e) => return err(format!("browserbase DELETE {url}: {e}")),
    };
    let status = resp.status();
    if !status.is_success() && status != reqwest::StatusCode::NOT_FOUND {
        let body_text = resp
            .text()
            .await
            .unwrap_or_else(|e| format!("(failed to read response body: {e})"));
        return err(format!(
            "browserbase DELETE {url} returned {status}: {body_text}"
        ));
    }
    CallToolResult {
        content: vec![McpContent::Text {
            text: serde_json::to_string(&serde_json::json!({
                "session_id": session_id,
                "closed": true,
            }))
            .unwrap_or_default(),
        }],
        is_error: false,
    }
}

#[cfg(not(feature = "browserbase"))]
pub async fn open_session() -> CallToolResult {
    CallToolResult {
        content: vec![McpContent::Text {
            text: "cloud_browserbase_open_session requires the `browserbase` feature. \
                   Rebuild with `cargo build -p argo-mcp-browser --features browserbase` \
                   then export BROWSERBASE_API_KEY + BROWSERBASE_PROJECT_ID. Sign up free \
                   at https://browserbase.com (60 min/month free tier)."
                .into(),
        }],
        is_error: true,
    }
}

#[cfg(not(feature = "browserbase"))]
pub async fn close_session(_session_id: &str) -> CallToolResult {
    CallToolResult {
        content: vec![McpContent::Text {
            text: "cloud_browserbase_close_session requires the `browserbase` feature. \
                   Rebuild with `cargo build -p argo-mcp-browser --features browserbase`."
                .into(),
        }],
        is_error: true,
    }
}
