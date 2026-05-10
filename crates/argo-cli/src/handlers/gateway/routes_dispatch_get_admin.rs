//! Admin/meta GET dispatch lifted out of `routes.rs` to keep that
//! file under the 500-LOC budget. Pure router — every match
//! arm forwards to a handler defined in another sibling
//! module. Returns `Some(resp)` on a match, `None` otherwise so
//! `routes.rs` can fall through to the resource dispatcher and
//! eventually the 404 default.
//!
//! Holds the introspection/health/catalog endpoints that don't
//! map to a domain resource — the per-resource GETs (sessions,
//! skills, memory, providers, tools, verticals, cron, forge,
//! workspace, audit/sessions/<id>) live in `routes_dispatch_get`.
use super::routes_audit::dispatch_audit_recent;
use super::routes_audit_stream::dispatch_audit_stream;
use super::routes_meta::{
    db_integrity_response, db_stats_response, db_version_response, routes_response,
    search_response,
};
use super::routes_query::{parse_query_kv, parse_search_args, parse_since_minutes_from_get};
use super::routes_search::search_count_response;
use super::routes_top::{audit_breaks_response, audit_response, doctor_response, health_full_response};
use super::routes_workspace::{artifacts_response, workspace_info_response};
use argo_store::SessionStore;
use std::sync::Arc;

/// Dispatch admin/meta GETs. Returns `None` when no match.
pub(super) async fn dispatch_get_admin(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("GET / ") {
        // Root landing — closes the discoverability papercut where
        // pointing a browser at the gateway returned a bare 404 with
        // no hint of what to try next. Sends users straight to /routes
        // for the full catalog and names a few high-frequency
        // entry-points so a curl at `/` is immediately useful.
        let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
        return Some((
            "200 OK",
            format!(
                "{{\"name\":\"argo\",\"version\":\"{}\",\"profile\":\"{profile}\",\
                  \"docs\":\"https://github.com/mark-hallam/Argo-Agent\",\
                  \"hint\":\"this is a JSON API; no landing page. Try the entry points below.\",\
                  \"entry_points\":{{\
                    \"catalog\":\"/routes\",\
                    \"health\":\"/health\",\
                    \"version\":\"/version\",\
                    \"workspace_health\":\"/doctor\",\
                    \"recent\":[\"/sessions\",\"/audit/recent\",\"/messages/recent\",\"/costs/recent\"]\
                  }},\
                  \"tip\":\"append ?format=markdown to most GET endpoints for a paste-able human-readable view\"}}",
                env!("CARGO_PKG_VERSION")
            ),
        ));
    }
    if req.starts_with("GET /version ") {
        let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
        return Some((
            "200 OK",
            format!(
                "{{\"version\":\"{}\",\"profile\":\"{profile}\"}}",
                env!("CARGO_PKG_VERSION")
            ),
        ));
    }
    if req.starts_with("GET /health/live ") {
        return Some(super::routes_health::live_response());
    }
    if req.starts_with("GET /health/ready ") {
        return Some(super::routes_health::ready_response(store).await);
    }
    if req.starts_with("GET /health/full ") {
        return Some(health_full_response().await);
    }
    if req.starts_with("GET /health ") {
        return Some(super::routes_health::health_response());
    }
    if req.starts_with("GET /doctor ") || req.starts_with("GET /doctor?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return Some(doctor_response(format.as_deref()).await);
    }
    if req.starts_with("GET /metrics/json ") || req.starts_with("GET /metrics/json?") {
        let body = super::metrics_json::render_json(store).await;
        return Some(("200 OK", body));
    }
    if req.starts_with("GET /routes ") || req.starts_with("GET /routes?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let method = parse_query_kv(tail, "method");
        let format = parse_query_kv(tail, "format");
        if matches!(format.as_deref(), Some("markdown")) {
            return Some(super::routes_catalog::routes_markdown(method.as_deref()));
        }
        return Some(routes_response(method.as_deref()));
    }
    if req.starts_with("GET /db/stats ") || req.starts_with("GET /db/stats?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return Some(db_stats_response(store, format.as_deref()).await);
    }
    if req.starts_with("GET /db/version ") || req.starts_with("GET /db/version?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return Some(db_version_response(store, format.as_deref()).await);
    }
    if req.starts_with("GET /db/integrity ") || req.starts_with("GET /db/integrity?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return Some(db_integrity_response(store, format.as_deref()).await);
    }
    if req.starts_with("GET /audit ") || req.starts_with("GET /audit?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return Some(audit_response(store, format.as_deref()).await);
    }
    if req.starts_with("GET /audit/breaks ") || req.starts_with("GET /audit/breaks?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return Some(audit_breaks_response(store, format.as_deref()).await);
    }
    if req.starts_with("GET /audit/count ") || req.starts_with("GET /audit/count?") {
        return Some(super::routes_counts::dispatch_audit_count(req, store).await);
    }
    if req.starts_with("GET /audit/recent ") || req.starts_with("GET /audit/recent?") {
        return Some(dispatch_audit_recent(req, store).await);
    }
    if req.starts_with("GET /audit/stream ") || req.starts_with("GET /audit/stream?") {
        return Some(dispatch_audit_stream(req, store).await);
    }
    if req.starts_with("GET /workspace/info ") || req.starts_with("GET /workspace/info?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return Some(workspace_info_response(store, format.as_deref()).await);
    }
    if req.starts_with("GET /workspace/artifacts ")
        || req.starts_with("GET /workspace/artifacts?")
    {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let limit: u32 = parse_query_kv(tail, "limit")
            .and_then(|v| v.parse().ok())
            .unwrap_or(25);
        let since = parse_since_minutes_from_get(req, "/workspace/artifacts");
        let format = parse_query_kv(tail, "format");
        return Some(artifacts_response(limit, since, store, format.as_deref()).await);
    }
    if req.starts_with("GET /search/count ") || req.starts_with("GET /search/count?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let q = parse_query_kv(tail, "q").map(|s| s.replace('+', " "));
        let session = parse_query_kv(tail, "session");
        let role = parse_query_kv(tail, "role");
        let format = parse_query_kv(tail, "format");
        return Some(
            search_count_response(
                q.as_deref(),
                session.as_deref(),
                role.as_deref(),
                store,
                format.as_deref(),
            )
            .await,
        );
    }
    if req.starts_with("GET /search ") || req.starts_with("GET /search?") {
        let (q, limit, session) = parse_search_args(req);
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let role = parse_query_kv(tail, "role");
        let format = parse_query_kv(tail, "format");
        return Some(
            search_response(
                q.as_deref(),
                limit,
                session.as_deref(),
                role.as_deref(),
                store,
                format.as_deref(),
            )
            .await,
        );
    }
    None
}
