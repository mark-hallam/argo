//! `/sessions/by-day`, `/messages/by-day`, `/audit/by-day`,
//! `/audit/by-kind` — analytics rollup dispatchers lifted out
//! of `routes.rs` so the main dispatcher stays under the
//! 500-LOC budget. Pure router — every match arm forwards to
//! the corresponding sibling-module handler. Returns
//! `Some(resp)` on a match, `None` otherwise so `routes.rs`
//! falls through to other branches.
use super::routes_query::parse_query_kv;
use argo_store::SessionStore;
use std::sync::Arc;

/// Dispatch the four analytics rollup endpoints. Caller should
/// short-circuit on `Some(resp)`.
pub(super) async fn dispatch_rollups(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("GET /sessions/by-day ") || req.starts_with("GET /sessions/by-day?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let days = parse_query_kv(tail, "days")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(30);
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_sessions_rollups::sessions_by_day_response(
                days,
                store,
                format.as_deref(),
            )
            .await,
        );
    }
    if req.starts_with("GET /sessions/by-provider ")
        || req.starts_with("GET /sessions/by-provider?")
    {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let since = parse_query_kv(tail, "since-minutes")
            .and_then(|v| v.parse::<i64>().ok());
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_sessions_rollups::sessions_by_provider_response(
                since,
                store,
                format.as_deref(),
            )
            .await,
        );
    }
    if req.starts_with("GET /sessions/by-model ")
        || req.starts_with("GET /sessions/by-model?")
    {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let since = parse_query_kv(tail, "since-minutes")
            .and_then(|v| v.parse::<i64>().ok());
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_sessions_rollups::sessions_by_model_response(
                since,
                store,
                format.as_deref(),
            )
            .await,
        );
    }
    if req.starts_with("GET /messages/by-day ") || req.starts_with("GET /messages/by-day?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let days = parse_query_kv(tail, "days")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(30);
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_msgs::messages_by_day_response(days, store, format.as_deref())
                .await,
        );
    }
    if req.starts_with("GET /audit/by-day ") || req.starts_with("GET /audit/by-day?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let days = parse_query_kv(tail, "days")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(30);
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_audit::audit_by_day_response(days, store, format.as_deref())
                .await,
        );
    }
    if req.starts_with("GET /audit/by-kind ") || req.starts_with("GET /audit/by-kind?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let since = parse_query_kv(tail, "since-minutes")
            .and_then(|v| v.parse::<i64>().ok());
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_audit::audit_by_kind_response(since, store, format.as_deref())
                .await,
        );
    }
    if req.starts_with("GET /audit/by-session ")
        || req.starts_with("GET /audit/by-session?")
    {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let limit: u32 = parse_query_kv(tail, "limit")
            .and_then(|v| v.parse().ok())
            .unwrap_or(50);
        let since = parse_query_kv(tail, "since-minutes")
            .and_then(|v| v.parse::<i64>().ok());
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_audit_rollups::audit_by_session_response(
                limit,
                since,
                store,
                format.as_deref(),
            )
            .await,
        );
    }
    if req.starts_with("GET /forge/runs/by-day ")
        || req.starts_with("GET /forge/runs/by-day?")
    {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let days = parse_query_kv(tail, "days")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(30);
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_forge::forge_runs_by_day_response(
                days,
                store,
                format.as_deref(),
            )
            .await,
        );
    }
    if req.starts_with("GET /forge/runs/by-status ")
        || req.starts_with("GET /forge/runs/by-status?")
    {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let since = parse_query_kv(tail, "since-minutes")
            .and_then(|v| v.parse::<i64>().ok());
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_forge::forge_runs_by_status_response(
                since,
                store,
                format.as_deref(),
            )
            .await,
        );
    }
    if req.starts_with("GET /forge/runs/by-skill ")
        || req.starts_with("GET /forge/runs/by-skill?")
    {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let since = parse_query_kv(tail, "since-minutes")
            .and_then(|v| v.parse::<i64>().ok());
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_forge_rollups::forge_runs_by_skill_response(
                since,
                store,
                format.as_deref(),
            )
            .await,
        );
    }
    None
}
