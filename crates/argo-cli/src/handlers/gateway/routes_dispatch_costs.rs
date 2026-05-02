//! `/costs/*` GET dispatch lifted out of `routes.rs` so the
//! main dispatcher stays under the 500-LOC budget. Pure router
//! — every match arm forwards to a handler defined in another
//! sibling module. Returns `Some(resp)` on a match, `None`
//! otherwise so `routes.rs` falls through to other branches.
use super::routes_meta::{costs_response, costs_total_response};
use super::routes_post::costs_breakdown_response;
use super::routes_query::parse_query_kv;
use argo_store::SessionStore;
use std::sync::Arc;

/// Dispatch every `GET /costs[/...]` request. Order matters —
/// the more-specific suffixed paths have to win against the
/// bare `/costs ` prefix, so they're matched first here.
pub(super) async fn dispatch_costs(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("GET /costs/total ") || req.starts_with("GET /costs/total?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return Some(costs_total_response(store, format.as_deref()).await);
    }
    if req.starts_with("GET /costs/by-model ") || req.starts_with("GET /costs/by-model?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return Some(costs_breakdown_response("by_model", store, format.as_deref()).await);
    }
    if req.starts_with("GET /costs/by-session ") || req.starts_with("GET /costs/by-session?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return Some(
            costs_breakdown_response("by_session", store, format.as_deref()).await,
        );
    }
    if req.starts_with("GET /costs/by-day ") || req.starts_with("GET /costs/by-day?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let days = parse_query_kv(tail, "days")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(30);
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_costs_by_day::costs_by_day_response(
                days,
                store,
                format.as_deref(),
            )
            .await,
        );
    }
    if req.starts_with("GET /costs ") || req.starts_with("GET /costs?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return Some(costs_response(format.as_deref()));
    }
    None
}
