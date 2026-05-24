//! POST /sessions/<uuid>/{runs,replay,steer} — agent-turn-bound
//! mutations split out of `routes_post.rs` for the 500-LOC budget.
//!
//! Public API stays stable: `routes_post.rs` re-exports
//! `run_turn_response`, `replay_response`, and `steer_response`
//! so existing call sites compile unchanged.
use argo_store::SessionStore;
use argo_types::{MessageRole, SessionId};
use std::sync::Arc;

use super::tenant_header::{assert_session_belongs_to_tenant, request_tenant_id};
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
    let tenant_id = request_tenant_id(req);
    if let Err(resp) = assert_session_belongs_to_tenant(store, id, sid, &tenant_id).await {
        return resp;
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
    let tenant_id = request_tenant_id(req);
    if let Err(resp) = assert_session_belongs_to_tenant(store, id, sid, &tenant_id).await {
        return resp;
    }
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
    let content = serde_json::json!([{
        "type": "text",
        "text": format!("[steer] {text}"),
    }]);
    match store
        .append_message(id, MessageRole::System, &content.to_string())
        .await
    {
        Ok(msg_id) => ("201 Created", format!("{{\"message_id\":\"{msg_id}\"}}")),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"steer: {e}\"}}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_types::{TenantId, WorkspaceId};

    async fn mem_store() -> Arc<SessionStore> {
        let store = SessionStore::open(":memory:").await.expect("open mem db");
        store.migrate().await.expect("migrate");
        Arc::new(store)
    }

    async fn seed_session(store: &Arc<SessionStore>) -> (TenantId, SessionId) {
        let tenant = TenantId::new();
        let workspace = WorkspaceId::new();
        sqlx::query(
            "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
             VALUES (?, ?, 'cluster6-run', '.', strftime('%s','now'))",
        )
        .bind(workspace.0.as_bytes().as_slice())
        .bind(tenant.0.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .expect("seed workspace");
        let session = store
            .create_session(
                tenant,
                workspace,
                "mock",
                "mock",
                Some("cluster6-run"),
                None,
            )
            .await
            .expect("create session");
        (tenant, session)
    }

    fn req_with_tenant(path: &str, tenant: &TenantId, body: &str) -> String {
        format!(
            "POST {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Tenant-Id: {}\r\n\
             Content-Length: {}\r\n\r\n{body}",
            tenant.0,
            body.len()
        )
    }

    #[tokio::test]
    async fn run_turn_matching_tenant_reaches_body_parser() {
        let store = mem_store().await;
        let (tenant, session) = seed_session(&store).await;
        let sid = session.0.to_string();
        let req = req_with_tenant(&format!("/sessions/{sid}/runs"), &tenant, "{not-json");

        let (status, body) = run_turn_response(&sid, &req, Some(&store)).await;

        assert!(status.starts_with("400"), "status={status} body={body}");
        assert!(body.contains("parse body"), "body={body}");
    }
}
