//! Count-endpoint dispatch helpers. Extracted from `routes.rs`
//! so the dispatcher stays under the 500-LOC budget. Each fn
//! parses the query string for one /count path and delegates
//! to its underlying response handler.

use super::routes_audit::audit_count_response;
use super::routes_forge::forge_runs_count_response;
use super::routes_lists::{memory_count_response, skills_count_response};
use super::routes_query::{parse_query_kv, parse_since_minutes_from_get};
use super::routes_top::sessions_count_response;
use argo_store::SessionStore;
use std::sync::Arc;

/// `GET /sessions/count[?status&since-minutes&older-than-minutes&provider&model]`
/// dispatch.
pub(super) async fn dispatch_sessions_count(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let tail = req.strip_prefix("GET ").unwrap_or(req);
    let status = parse_query_kv(tail, "status");
    let since = parse_since_minutes_from_get(req, "/sessions/count");
    let older_than =
        parse_query_kv(tail, "older-than-minutes").and_then(|v| v.parse::<i64>().ok());
    let provider = parse_query_kv(tail, "provider");
    let model = parse_query_kv(tail, "model");
    sessions_count_response(
        status.as_deref(),
        since,
        older_than,
        provider.as_deref(),
        model.as_deref(),
        store,
    )
    .await
}

/// `GET /skills/count[?include-superseded]` dispatch.
pub(super) async fn dispatch_skills_count(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let tail = req.strip_prefix("GET ").unwrap_or(req);
    let include_superseded = parse_query_kv(tail, "include-superseded")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    skills_count_response(include_superseded, store).await
}

/// `GET /memory/count[?file&since-minutes]` dispatch.
pub(super) async fn dispatch_memory_count(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let tail = req.strip_prefix("GET ").unwrap_or(req);
    let file = parse_query_kv(tail, "file");
    let since = parse_since_minutes_from_get(req, "/memory/count");
    memory_count_response(file, since, store).await
}

/// `GET /audit/count[?kind&since-minutes&session]` dispatch.
pub(super) async fn dispatch_audit_count(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let tail = req.strip_prefix("GET ").unwrap_or(req);
    let kind = parse_query_kv(tail, "kind");
    let since = parse_since_minutes_from_get(req, "/audit/count");
    let session = parse_query_kv(tail, "session");
    audit_count_response(kind.as_deref(), since, session.as_deref(), store).await
}

/// `GET /forge/runs/count[?status&since-minutes]` dispatch.
pub(super) async fn dispatch_forge_runs_count(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let tail = req.strip_prefix("GET ").unwrap_or(req);
    let status = parse_query_kv(tail, "status");
    let since = parse_since_minutes_from_get(req, "/forge/runs/count");
    forge_runs_count_response(status.as_deref(), since, store).await
}
