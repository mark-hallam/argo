//! HTTP route dispatch + GET response builders. POST handlers +
//! the heavier query helpers (audit-session, costs-breakdown)
//! live in the sibling `routes_post` module to keep this file
//! under the 500-LOC budget. Each handler returns a
//! `(status_line, body)` pair the listener loop formats into a
//! single-line HTTP/1.1 response.
use super::routes_meta::{
    artifacts_response, forge_run_show_response, forge_runs_response, providers_response,
    routes_response, search_response, tools_response,
};
use super::routes_post::{
    append_message_response, audit_session_response, costs_breakdown_response,
    create_session_response, replay_response, run_turn_response, steer_response,
};
use super::routes_session::{session_show_response, session_spend_response};
use super::store_unavailable;
use argo_store::SessionStore;
use argo_types::SessionId;
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
    if req.starts_with("GET /sessions ") || req.starts_with("GET /sessions?") {
        let since = parse_since_minutes_from_get(req, "/sessions");
        return sessions_response(store, since).await;
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
    if req.starts_with("GET /doctor ") {
        return doctor_response().await;
    }
    if req.starts_with("GET /health/full ") {
        return health_full_response().await;
    }
    if req.starts_with("GET /providers ") {
        return providers_response().await;
    }
    if req.starts_with("GET /tools ") {
        return tools_response().await;
    }
    if req.starts_with("GET /routes ") {
        return routes_response();
    }
    if req.starts_with("GET /search ") || req.starts_with("GET /search?") {
        let (q, limit, session) = parse_search_args(req);
        return search_response(q.as_deref(), limit, session.as_deref(), store).await;
    }
    if req.starts_with("GET /workspace/artifacts ")
        || req.starts_with("GET /workspace/artifacts?")
    {
        let limit: u32 = req
            .strip_prefix("GET ")
            .and_then(|tail| parse_query_kv(tail, "limit"))
            .and_then(|v| v.parse().ok())
            .unwrap_or(25);
        return artifacts_response(limit, store).await;
    }
    if req.starts_with("GET /forge/runs ") || req.starts_with("GET /forge/runs?") {
        let limit: u32 = req
            .strip_prefix("GET ")
            .and_then(|tail| parse_query_kv(tail, "limit"))
            .and_then(|v| v.parse().ok())
            .unwrap_or(50);
        return forge_runs_response(limit, store).await;
    }
    if let Some(rest) = req.strip_prefix("GET /forge/runs/") {
        if let Some((id, _)) = rest.split_once(' ') {
            return forge_run_show_response(id, store).await;
        }
    }
    if let Some(rest) = req.strip_prefix("GET /audit/sessions/") {
        if let Some((sid, _)) = rest.split_once(' ') {
            return audit_session_response(sid, store).await;
        }
    }
    if let Some(rest) = req.strip_prefix("GET /sessions/") {
        if let Some((sid, tail)) = rest.split_once('/') {
            if tail.starts_with("spend ") {
                return session_spend_response(sid, store).await;
            }
        }
    }
    if req.starts_with("GET /costs ") {
        return costs_response();
    }
    if req.starts_with("GET /costs/by-model ") {
        return costs_breakdown_response("by_model", store).await;
    }
    if req.starts_with("GET /costs/by-session ") {
        return costs_breakdown_response("by_session", store).await;
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
            if tail.starts_with("steer ") {
                return steer_response(sid, req, store).await;
            }
            if tail.starts_with("replay ") {
                return replay_response(sid, req, store).await;
            }
        }
    }
    // GET /sessions/<uuid> — fetch one session header
    // GET /sessions/<uuid>/messages — list every message on a session
    if let Some(rest) = req.strip_prefix("GET /sessions/") {
        if let Some((sid, tail)) = rest.split_once('/') {
            if tail.starts_with("messages ") || tail.starts_with("messages?") {
                let limit = parse_limit(tail);
                let role = parse_role(tail);
                return list_messages_response(sid, store, limit, role).await;
            }
        }
        // Strip trailing " HTTP/..." → just the session id
        if let Some((sid, _)) = rest.split_once(' ') {
            return session_show_response(sid, store).await;
        }
    }
    ("404 Not Found", "{\"error\":\"not found\"}".to_string())
}

/// Pull `since-minutes=<N>` out of `GET <path>?since-minutes=N HTTP/1.1`.
fn parse_since_minutes_from_get(req: &str, path: &str) -> Option<i64> {
    let prefix = format!("GET {path}?");
    let rest = req.strip_prefix(&prefix)?;
    let q = rest.split_once(' ').map(|(q, _)| q).unwrap_or(rest);
    for kv in q.split('&') {
        let (k, v) = kv.split_once('=')?;
        if k == "since-minutes" {
            return v.parse().ok();
        }
    }
    None
}

async fn sessions_response(
    store: Option<&Arc<SessionStore>>,
    since_minutes: Option<i64>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    match store.list_all_sessions(100).await {
        Ok(mut rows) => {
            if let Some(minutes) = since_minutes {
                let cutoff = chrono::Utc::now() - chrono::Duration::minutes(minutes);
                rows.retain(|s| s.started_at >= cutoff);
            }
            let spend_by_sid = super::spend_per_session(store).await.unwrap_or_default();
            let arr: Vec<serde_json::Value> = rows
                .iter()
                .map(|s| {
                    let spent = spend_by_sid
                        .get(s.id.0.as_bytes().as_slice())
                        .copied()
                        .unwrap_or(0);
                    serde_json::json!({
                        "id": s.id.0.to_string(),
                        "model": s.model,
                        "provider": s.provider,
                        "title": s.title,
                        "started_at": s.started_at.to_rfc3339(),
                        "ended_at": s.ended_at.map(|d| d.to_rfc3339()),
                        "spent_usd_micros": spent,
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

async fn doctor_response() -> (&'static str, String) {
    match crate::handlers::doctor::collect().await {
        Ok(v) => (
            "200 OK",
            serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string()),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"doctor: {e}\"}}"),
        ),
    }
}

async fn health_full_response() -> (&'static str, String) {
    match crate::handlers::health::collect().await {
        Ok(v) => {
            let status = if v["ok"].as_bool().unwrap_or(false) {
                "200 OK"
            } else {
                "500 Internal Server Error"
            };
            (
                status,
                serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"health: {e}\"}}"),
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

/// Pull `limit=<N>` out of a path tail like `messages?limit=10 HTTP/1.1`.
/// Returns `None` if absent or unparseable; caller treats `None` as
/// "all rows".
fn parse_limit(tail: &str) -> Option<usize> {
    parse_query_kv(tail, "limit").and_then(|v| v.parse().ok())
}

/// Pull `role=<s>` out of the path tail (e.g. `?role=user`).
fn parse_role(tail: &str) -> Option<String> {
    parse_query_kv(tail, "role")
}

/// Pull `(q, limit, session)` out of a `/search?…` request.
/// `limit` defaults to 20 when missing or unparseable. `q` is
/// URL-decoded for `+` only — full % decoding is left to the
/// CLI client (the smoke test sends pre-decoded queries).
fn parse_search_args(req: &str) -> (Option<String>, u32, Option<String>) {
    let tail = req.strip_prefix("GET ").unwrap_or(req);
    let q = parse_query_kv(tail, "q").map(|s| s.replace('+', " "));
    let limit = parse_query_kv(tail, "limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);
    let session = parse_query_kv(tail, "session");
    (q, limit, session)
}

fn parse_query_kv(tail: &str, key: &str) -> Option<String> {
    let q = tail.split_once('?')?.1;
    let q = q.split_once(' ').map(|(q, _)| q).unwrap_or(q);
    for kv in q.split('&') {
        let (k, v) = kv.split_once('=')?;
        if k == key {
            return Some(v.to_string());
        }
    }
    None
}

async fn list_messages_response(
    sid: &str,
    store: Option<&Arc<SessionStore>>,
    limit: Option<usize>,
    role: Option<String>,
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
        Ok(mut rows) => {
            if let Some(want) = role.as_deref() {
                rows.retain(|m| m.role == want);
            }
            if let Some(n) = limit {
                if rows.len() > n {
                    rows.truncate(n);
                }
            }
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
