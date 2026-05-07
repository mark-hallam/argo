//! POST /sessions/<uuid>/messages handler, split out of
//! `routes_post.rs` for the 500-LOC budget.
//!
//! Public API stays stable: `routes_post.rs` re-exports
//! `append_message_response` so existing call sites compile
//! unchanged.
use argo_store::SessionStore;
use argo_types::{MessageRole, SessionId};
use std::sync::Arc;

use super::{body_from_request, store_unavailable};

pub(super) async fn append_message_response(
    sid: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(sid) {
        Ok(u) => SessionId(u),
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid session id: {e}\"}}"),
            );
        }
    };
    let body = body_from_request(req);
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"parse body: {e}\"}}"),
            );
        }
    };
    let role = match parsed["role"].as_str() {
        Some("user") => MessageRole::User,
        Some("assistant") => MessageRole::Assistant,
        Some("system") => MessageRole::System,
        Some("tool") => MessageRole::Tool,
        Some(other) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"unknown role '{other}': use user|assistant|system|tool\"}}"),
            );
        }
        None => {
            return (
                "400 Bad Request",
                "{\"error\":\"required field: role\"}".to_string(),
            );
        }
    };
    let content_json = match parsed.get("content") {
        Some(c) => c.to_string(),
        None => {
            return (
                "400 Bad Request",
                "{\"error\":\"required field: content (typed JSON value)\"}".to_string(),
            );
        }
    };
    match store.append_message(id, role, &content_json).await {
        Ok(msg_id) => (
            "201 Created",
            serde_json::to_string(&serde_json::json!({
                "id": msg_id.to_string(),
                "session_id": sid,
            }))
            .unwrap_or_else(|_| "{}".to_string()),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"append_message: {e}\"}}"),
        ),
    }
}
