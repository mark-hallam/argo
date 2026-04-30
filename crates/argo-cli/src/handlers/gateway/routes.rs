//! HTTP route dispatch + per-endpoint response builders. Each
//! handler returns a `(status_line, body)` pair the listener loop
//! formats into a single-line HTTP/1.1 response.
use argo_store::SessionStore;
use argo_types::{MessageRole, SessionId};
use std::sync::Arc;

pub(super) async fn dispatch_route(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    if req.starts_with("GET /version ") {
        return (
            "200 OK",
            format!("{{\"version\":\"{}\"}}", env!("CARGO_PKG_VERSION")),
        );
    }
    if req.starts_with("GET /health ") {
        return ("200 OK", "{\"status\":\"ok\"}".to_string());
    }
    if req.starts_with("GET /sessions ") {
        return sessions_response(store).await;
    }
    if req.starts_with("GET /skills ") {
        return skills_response(store).await;
    }
    if req.starts_with("GET /memory ") {
        return memory_response(store).await;
    }
    if req.starts_with("GET /audit ") {
        return audit_response(store).await;
    }
    if req.starts_with("GET /costs ") {
        return costs_response();
    }
    if req.starts_with("POST /sessions ") {
        return create_session_response(req, store).await;
    }
    // POST /sessions/<uuid>/messages — append a message to a
    // recorded session. We do the URL parse here to keep
    // dispatch_route as the single entry-point.
    if let Some(rest) = req.strip_prefix("POST /sessions/") {
        if let Some((sid, tail)) = rest.split_once('/') {
            if tail.starts_with("messages ") {
                return append_message_response(sid, req, store).await;
            }
            if tail.starts_with("runs ") {
                return run_turn_response(sid, req, store).await;
            }
        }
    }
    // GET /sessions/<uuid> — fetch one session header
    // GET /sessions/<uuid>/messages — list every message on a session
    if let Some(rest) = req.strip_prefix("GET /sessions/") {
        if let Some((sid, tail)) = rest.split_once('/') {
            if tail.starts_with("messages ") {
                return list_messages_response(sid, store).await;
            }
        }
        // Strip trailing " HTTP/..." → just the session id
        if let Some((sid, _)) = rest.split_once(' ') {
            return session_show_response(sid, store).await;
        }
    }
    ("404 Not Found", "{\"error\":\"not found\"}".to_string())
}

fn store_unavailable() -> (&'static str, String) {
    (
        "503 Service Unavailable",
        "{\"error\":\"session store unavailable\"}".to_string(),
    )
}

fn body_from_request(req: &str) -> &str {
    req.split_once("\r\n\r\n").map(|(_h, b)| b).unwrap_or("")
}

async fn sessions_response(store: Option<&Arc<SessionStore>>) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    match store.list_all_sessions(100).await {
        Ok(rows) => {
            let arr: Vec<serde_json::Value> = rows
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "id": s.id.0.to_string(),
                        "model": s.model,
                        "provider": s.provider,
                        "title": s.title,
                        "started_at": s.started_at.to_rfc3339(),
                        "ended_at": s.ended_at.map(|d| d.to_rfc3339()),
                    })
                })
                .collect();
            (
                "200 OK",
                serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"list sessions: {e}\"}}"),
        ),
    }
}

async fn skills_response(store: Option<&Arc<SessionStore>>) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let skills = argo_skill::SkillStore::new(Arc::clone(store));
    match skills.list_active().await {
        Ok(rows) => {
            let arr: Vec<serde_json::Value> = rows
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "id": s.id.0.to_string(),
                        "name": s.frontmatter.name,
                        "version": s.frontmatter.version,
                        "lock": format!("{:?}", s.frontmatter.lock),
                        "hash": hex::encode(s.hash),
                        "created_at": s.created_at.to_rfc3339(),
                    })
                })
                .collect();
            (
                "200 OK",
                serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"list skills: {e}\"}}"),
        ),
    }
}

async fn memory_response(store: Option<&Arc<SessionStore>>) -> (&'static str, String) {
    use argo_memory::{MemoryFile, MemoryStore};
    let Some(store) = store else {
        return store_unavailable();
    };
    let workspace = match store.first_workspace().await {
        Ok(Some(w)) => w,
        Ok(None) => return ("200 OK", "[]".to_string()),
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"first_workspace: {e}\"}}"),
            );
        }
    };
    let memory = MemoryStore::new(Arc::clone(store));
    let mut all: Vec<serde_json::Value> = Vec::new();
    for f in [
        MemoryFile::Memory,
        MemoryFile::User,
        MemoryFile::Soul,
        MemoryFile::Agents,
    ] {
        match memory.list_active(workspace, f).await {
            Ok(entries) => {
                for e in entries {
                    all.push(serde_json::json!({
                        "id": e.id.to_string(),
                        "file": format!("{:?}", e.file),
                        "bytes": e.body.len(),
                        "lock": format!("{:?}", e.lock),
                        "hash": hex::encode(e.hash),
                        "learned_at": e.learned_at.to_rfc3339(),
                    }));
                }
            }
            Err(e) => {
                return (
                    "500 Internal Server Error",
                    format!("{{\"error\":\"list {f:?}: {e}\"}}"),
                );
            }
        }
    }
    (
        "200 OK",
        serde_json::to_string(&all).unwrap_or_else(|_| "[]".to_string()),
    )
}

async fn create_session_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
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
    let model = parsed["model"].as_str().unwrap_or("");
    let provider = parsed["provider"].as_str().unwrap_or("");
    if model.is_empty() || provider.is_empty() {
        return (
            "400 Bad Request",
            "{\"error\":\"required fields: model, provider\"}".to_string(),
        );
    }
    let title = parsed["title"].as_str();
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"cwd: {e}\"}}"),
            );
        }
    };
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, workspace) = match store.ensure_workspace(name, &root).await {
        Ok(p) => p,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"ensure_workspace: {e}\"}}"),
            );
        }
    };
    match store
        .create_session(tenant, workspace, model, provider, title)
        .await
    {
        Ok(id) => (
            "201 Created",
            serde_json::to_string(&serde_json::json!({
                "id": id.0.to_string(),
                "model": model,
                "provider": provider,
                "title": title,
            }))
            .unwrap_or_else(|_| "{}".to_string()),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"create_session: {e}\"}}"),
        ),
    }
}

async fn append_message_response(
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

async fn session_show_response(
    sid: &str,
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
    let session = match store.get_session(id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                "404 Not Found",
                format!("{{\"error\":\"no session {sid}\"}}"),
            );
        }
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"get_session: {e}\"}}"),
            );
        }
    };
    let count = store.message_count(id).await.unwrap_or(0);
    let tool_calls = super::session_tool_call_count(store, id).await.unwrap_or(0);
    let spent = super::session_spend_micros(store, id).await.unwrap_or(0);
    let row = serde_json::json!({
        "id": session.id.0.to_string(),
        "tenant_id": session.tenant_id.0.to_string(),
        "workspace_id": session.workspace_id.0.to_string(),
        "model": session.model,
        "provider": session.provider,
        "title": session.title,
        "started_at": session.started_at.to_rfc3339(),
        "ended_at": session.ended_at.map(|d| d.to_rfc3339()),
        "message_count": count,
        "tool_call_count": tool_calls,
        "spent_usd_micros": spent,
    });
    (
        "200 OK",
        serde_json::to_string(&row).unwrap_or_else(|_| "{}".to_string()),
    )
}

async fn run_turn_response(
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
    // Body is optional; if present, may carry {user, stream}.
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
    // Streaming over HTTP is the daemon's job; for now we always
    // run non-streaming and return the final body. Token-by-token
    // streaming will be a follow-up that emits chunked JSON.
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"cwd: {e}\"}}"),
            );
        }
    };
    match crate::handlers::chat::run_turn(store, &cwd, id, user, false).await {
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

async fn audit_response(store: Option<&Arc<SessionStore>>) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let report = match argo_audit::verify_all(store).await {
        Ok(r) => r,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"verify_all: {e}\"}}"),
            );
        }
    };
    let mut kinds: Vec<(String, usize)> = report
        .kind_counts
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    kinds.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let kinds_obj: serde_json::Map<String, serde_json::Value> = kinds
        .into_iter()
        .map(|(k, v)| (k, serde_json::Value::from(v)))
        .collect();
    let payload = serde_json::json!({
        "ok": report.ok,
        "rows": report.rows,
        "breaks": report.breaks.len(),
        "kinds": kinds_obj,
    });
    (
        "200 OK",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
    )
}

fn costs_response() -> (&'static str, String) {
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"cwd: {e}\"}}"),
            );
        }
    };
    let cfg = argo_config::ArgoConfig::load(&cwd.join("argo.toml")).unwrap_or_default();
    let ceilings = cfg.budget.to_cost_ceilings();
    let payload = serde_json::json!({
        "per_session_usd_micros": ceilings.per_session_usd_micros,
        "per_day_usd_micros":     ceilings.per_day_usd_micros,
        "per_month_usd_micros":   ceilings.per_month_usd_micros,
        "notify_at_pct":          ceilings.notify_at_pct,
    });
    (
        "200 OK",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
    )
}

async fn list_messages_response(
    sid: &str,
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
    match store.list_messages(id).await {
        Ok(rows) => {
            let arr: Vec<serde_json::Value> = rows
                .iter()
                .map(|m| {
                    serde_json::json!({
                        "seq": m.seq,
                        "id": hex::encode(&m.id),
                        "role": m.role,
                        "content": serde_json::from_str::<serde_json::Value>(&m.content_json)
                            .unwrap_or(serde_json::Value::Null),
                        "created_at_micros": m.created_at,
                    })
                })
                .collect();
            (
                "200 OK",
                serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"list_messages: {e}\"}}"),
        ),
    }
}
