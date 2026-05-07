//! POST /sessions/<uuid>/{runs,replay,steer} — agent-turn-bound
//! mutations split out of `routes_post.rs` for the 500-LOC budget.
//!
//! Public API stays stable: `routes_post.rs` re-exports
//! `run_turn_response`, `replay_response`, and `steer_response`
//! so existing call sites compile unchanged.
use argo_store::SessionStore;
use argo_types::{MessageRole, SessionId};
use std::sync::Arc;

use super::{body_from_request, store_unavailable};

pub(super) async fn run_turn_response(
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
    let parsed: serde_json::Value = if body.is_empty() {
        serde_json::Value::Null
    } else {
        match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => {
                return (
                    "400 Bad Request",
                    format!("{{\"error\":\"parse body: {e}\"}}"),
                );
            }
        }
    };
    let user = parsed.get("user").and_then(|v| v.as_str());
    let max_rounds = parsed
        .get("max_tool_rounds")
        .and_then(serde_json::Value::as_u64)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(crate::handlers::chat::MAX_TOOL_ROUNDS);
    // The gateway always streams text to its own stdout (visible
    // when running `argo gateway start` in foreground); the
    // response body is still the final JSON. SSE / chunked
    // streaming over HTTP is its own follow-up.
    let stream = parsed
        .get("stream")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"cwd: {e}\"}}"),
            );
        }
    };
    match crate::handlers::chat::run_turn(store, &cwd, id, user, stream, max_rounds).await {
        Ok(result) => (
            "201 Created",
            serde_json::to_string(&serde_json::json!({
                "assistant_id": result.assistant_id.to_string(),
                "text": result.text,
                "tokens_in": result.tokens_in,
                "tokens_out": result.tokens_out,
            }))
            .unwrap_or_else(|_| "{}".to_string()),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"run_turn: {e}\"}}"),
        ),
    }
}

pub(super) async fn replay_response(
    sid: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    if store.is_none() {
        return store_unavailable();
    }
    let body = body_from_request(req);
    let parsed: serde_json::Value = if body.is_empty() {
        serde_json::Value::Null
    } else {
        match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => {
                return (
                    "400 Bad Request",
                    format!("{{\"error\":\"parse body: {e}\"}}"),
                );
            }
        }
    };
    let model = parsed.get("model").and_then(|v| v.as_str());
    let provider = parsed.get("provider").and_then(|v| v.as_str());
    match crate::handlers::session_replay::replay_core(sid, model, provider).await {
        Ok(r) => (
            "201 Created",
            serde_json::to_string(&serde_json::json!({
                "source_id": r.source_id.0.to_string(),
                "new_id": r.new_id.0.to_string(),
                "provider": r.provider,
                "model": r.model,
                "user_message_count": r.user_message_count,
                "final_text": r.final_text,
            }))
            .unwrap_or_else(|_| "{}".to_string()),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"replay: {e}\"}}"),
        ),
    }
}

pub(super) async fn steer_response(
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
    let Some(text) = parsed.get("text").and_then(|v| v.as_str()) else {
        return (
            "400 Bad Request",
            "{\"error\":\"missing 'text' field\"}".to_string(),
        );
    };
    if let Ok(None) = store.get_session(id).await {
        return ("404 Not Found", format!("{{\"error\":\"no session {sid}\"}}"));
    }
    let content = serde_json::json!([{
        "type": "text",
        "text": format!("[steer] {text}"),
    }]);
    match store
        .append_message(id, MessageRole::System, &content.to_string())
        .await
    {
        Ok(msg_id) => (
            "201 Created",
            format!("{{\"message_id\":\"{msg_id}\"}}"),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"steer: {e}\"}}"),
        ),
    }
}
