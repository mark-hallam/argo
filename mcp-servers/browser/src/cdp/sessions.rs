//! Stateful browser sessions — a persistent CDP WebSocket pinned
//! to one tab so multi-step flows (navigate → fill → click) avoid
//! reconnecting per tool call.
//!
//! Implemented on `tokio-tungstenite` directly (no chromiumoxide
//! dep) because the existing single-shot CDP code already covers
//! the wire format. `BrowserSession` is just a `WebSocketStream`
//! plus a per-session message-id counter, kept alive in a global
//! `OnceLock<Mutex<HashMap<id, …>>>` until the user closes it.
//!
//! New tools:
//!   - `session_open  {port?}`           → `{"session_id": "..."}`
//!   - `session_close {session_id}`      → `{"closed": true}`
//!   - `session_eval  {session_id, expression}` → result.value
//!
//! Existing single-shot tools (`navigate`, `click`, …) are
//! unchanged. The session-aware variants are additive.
use argo_mcp::{CallToolResult, McpContent};
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

type Ws = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

struct BrowserSession {
    ws: Ws,
    next_id: u64,
}

fn registry() -> &'static Mutex<HashMap<String, Mutex<BrowserSession>>> {
    static R: OnceLock<Mutex<HashMap<String, Mutex<BrowserSession>>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(HashMap::new()))
}

const DEFAULT_PORT: u16 = 9222;

fn port_from_args(args: &serde_json::Value) -> u16 {
    args.get("port")
        .and_then(|v| v.as_u64())
        .and_then(|n| u16::try_from(n).ok())
        .unwrap_or(DEFAULT_PORT)
}

fn error(text: String) -> CallToolResult {
    CallToolResult {
        content: vec![McpContent::Text { text }],
        is_error: true,
    }
}

fn ok(text: String) -> CallToolResult {
    CallToolResult {
        content: vec![McpContent::Text { text }],
        is_error: false,
    }
}

/// `pub(super)` shim so the sibling `session_tools` module can
/// build `ok` content blocks without needing visibility into
/// `McpContent`. Returns an `is_error=false` Text content
/// block — same shape as the private `ok()` above.
pub(super) fn ok_result(text: String) -> CallToolResult {
    ok(text)
}

/// Open a persistent WebSocket to the first page-type tab and
/// register it in the session registry.
pub async fn session_open(args: &serde_json::Value) -> CallToolResult {
    let port = port_from_args(args);
    let tabs = match super::fetch_tabs(port).await {
        Ok(t) => t,
        Err(e) => return error(e),
    };
    let ws_url = match tabs
        .iter()
        .find_map(|t| {
            t.get("type")
                .and_then(|v| v.as_str())
                .filter(|&ty| ty == "page")
                .and(t.get("webSocketDebuggerUrl").and_then(|v| v.as_str()))
        })
        .map(str::to_string)
    {
        Some(u) => u,
        None => return error("no `page`-type tab in /json (open a tab in Chrome)".into()),
    };
    let (ws, _resp) = match connect_async(&ws_url).await {
        Ok(p) => p,
        Err(e) => return error(format!("connect ws {ws_url}: {e}")),
    };
    // Use a tiny pseudo-unique id derived from monotonic time —
    // avoids pulling in the uuid crate just for this.
    let id = format!(
        "br-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let session = BrowserSession { ws, next_id: 1 };
    registry().lock().await.insert(id.clone(), Mutex::new(session));
    ok(format!("{{\"session_id\":\"{id}\"}}"))
}

/// Close a registered session, dropping its WebSocket.
pub async fn session_close(args: &serde_json::Value) -> CallToolResult {
    let Some(sid) = args.get("session_id").and_then(|v| v.as_str()) else {
        return error("missing required field 'session_id'".into());
    };
    let removed = registry().lock().await.remove(sid);
    if removed.is_some() {
        ok(format!("{{\"session_id\":\"{sid}\",\"closed\":true}}"))
    } else {
        error(format!("no session '{sid}'"))
    }
}

/// `session_list` — return every active session id. Lets a
/// host audit + reap stale sessions when the agent loop has
/// gone home but the WS connections are still pinned.
pub async fn session_list(_args: &serde_json::Value) -> CallToolResult {
    let reg = registry().lock().await;
    let mut ids: Vec<String> = reg.keys().cloned().collect();
    ids.sort();
    let payload = serde_json::json!({
        "session_ids": ids,
        "count": reg.len(),
    });
    ok(serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()))
}


/// Internal helper: send a `Runtime.evaluate` over the
/// session's persistent WS and return the parsed JSON-RPC
/// envelope. Thin wrapper around `send_cdp_method` for the
/// common Runtime.evaluate case.
pub(super) async fn runtime_evaluate(
    sid: &str,
    expression: &str,
) -> Result<serde_json::Value, String> {
    send_cdp_method(
        sid,
        "Runtime.evaluate",
        serde_json::json!({"expression": expression, "returnByValue": true}),
    )
    .await
}

/// Internal helper: send any CDP method against the session's
/// persistent WS and return the parsed JSON-RPC envelope. All
/// session-aware tools (Runtime.evaluate variants AND
/// Page.* / Network.* / DOM.* methods) go through here so the
/// per-session message-id counter + read loop stay in one
/// place.
pub(super) async fn send_cdp_method(
    sid: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let reg = registry().lock().await;
    let session_mu = reg.get(sid).ok_or_else(|| format!("no session '{sid}'"))?;
    let mut session = session_mu.lock().await;
    let id = session.next_id;
    session.next_id += 1;
    let req = serde_json::json!({
        "id": id,
        "method": method,
        "params": params,
    });
    let serialized =
        serde_json::to_string(&req).map_err(|e| format!("serialize: {e}"))?;
    session
        .ws
        .send(Message::Text(serialized))
        .await
        .map_err(|e| format!("send: {e}"))?;
    let response_text = loop {
        match session.ws.next().await {
            Some(Ok(Message::Text(t))) => break t.to_string(),
            Some(Ok(Message::Close(_))) | None => return Err("ws closed".into()),
            Some(Ok(_)) => continue,
            Some(Err(e)) => return Err(format!("read: {e}")),
        }
    };
    serde_json::from_str(&response_text).map_err(|e| format!("parse: {e}"))
}
