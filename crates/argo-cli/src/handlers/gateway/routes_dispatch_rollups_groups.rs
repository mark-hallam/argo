//! Domain-grouped sub-dispatchers for
//! [`super::routes_dispatch_rollups::dispatch_rollups`]. Each
//! function owns one URL family (`/sessions/...`, `/messages/...`,
//! `/audit/...`, `/forge/...`) so the parent dispatcher stays a
//! thin fan-out and every helper stays well under the 80-LOC
//! ceiling.
use super::routes_query::parse_query_kv;
use argo_store::SessionStore;
use std::sync::Arc;

/// `/sessions/by-day`, `/sessions/by-provider`, `/sessions/by-model`.
pub(super) async fn dispatch_sessions_rollups(
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
        let since = parse_query_kv(tail, "since-minutes").and_then(|v| v.parse::<i64>().ok());
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
    if req.starts_with("GET /sessions/by-model ") || req.starts_with("GET /sessions/by-model?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let since = parse_query_kv(tail, "since-minutes").and_then(|v| v.parse::<i64>().ok());
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
    None
}

/// `/messages/by-day`, `/messages/by-role`, `/messages/by-session`,
/// `/messages/by-day-by-role`, `/messages/recent`.
pub(super) async fn dispatch_messages_rollups(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if let Some(resp) = dispatch_messages_by_day_and_role(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = dispatch_messages_session_and_recent(req, store).await {
        return Some(resp);
    }
    None
}

async fn dispatch_messages_by_day_and_role(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("GET /messages/by-day ") || req.starts_with("GET /messages/by-day?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let days = parse_query_kv(tail, "days")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(30);
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_msgs::messages_by_day_response(days, store, format.as_deref()).await,
        );
    }
    if req.starts_with("GET /messages/by-role ") || req.starts_with("GET /messages/by-role?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let since = parse_query_kv(tail, "since-minutes").and_then(|v| v.parse::<i64>().ok());
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_messages_rollups::messages_by_role_response(
                since,
                store,
                format.as_deref(),
            )
            .await,
        );
    }
    if req.starts_with("GET /messages/by-day-by-role ")
        || req.starts_with("GET /messages/by-day-by-role?")
    {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let days = parse_query_kv(tail, "days")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(30);
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_messages_rollups::messages_by_day_by_role_response(
                days,
                store,
                format.as_deref(),
            )
            .await,
        );
    }
    None
}

async fn dispatch_messages_session_and_recent(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("GET /messages/by-session ") || req.starts_with("GET /messages/by-session?")
    {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let limit: u32 = parse_query_kv(tail, "limit")
            .and_then(|v| v.parse().ok())
            .unwrap_or(50);
        let since = parse_query_kv(tail, "since-minutes").and_then(|v| v.parse::<i64>().ok());
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_messages_rollups::messages_by_session_response(
                limit,
                since,
                store,
                format.as_deref(),
            )
            .await,
        );
    }
    if req.starts_with("GET /messages/recent ") || req.starts_with("GET /messages/recent?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let limit: u32 = parse_query_kv(tail, "limit")
            .and_then(|v| v.parse().ok())
            .unwrap_or(50);
        let role = parse_query_kv(tail, "role");
        let since = parse_query_kv(tail, "since-minutes").and_then(|v| v.parse::<i64>().ok());
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_messages_rollups::messages_recent_response(
                limit,
                role.as_deref(),
                since,
                store,
                format.as_deref(),
            )
            .await,
        );
    }
    None
}

/// `/audit/by-day`, `/audit/by-kind`, `/audit/by-vertical`,
/// `/audit/by-session`, `/audit/by-tool`, `/audit/tool-errors`,
/// `/audit/by-day-by-kind`.
pub(super) async fn dispatch_audit_rollups(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if let Some(resp) = dispatch_audit_by_day_kind_vertical(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = dispatch_audit_by_session_tool(req, store).await {
        return Some(resp);
    }
    None
}

async fn dispatch_audit_by_day_kind_vertical(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("GET /audit/by-day ") || req.starts_with("GET /audit/by-day?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let days = parse_query_kv(tail, "days")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(30);
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_audit::audit_by_day_response(days, store, format.as_deref()).await,
        );
    }
    if req.starts_with("GET /audit/by-kind ") || req.starts_with("GET /audit/by-kind?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let since = parse_query_kv(tail, "since-minutes").and_then(|v| v.parse::<i64>().ok());
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_audit::audit_by_kind_response(since, store, format.as_deref()).await,
        );
    }
    if req.starts_with("GET /audit/by-vertical ") || req.starts_with("GET /audit/by-vertical?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let since = parse_query_kv(tail, "since-minutes").and_then(|v| v.parse::<i64>().ok());
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_audit_rollups::audit_by_vertical_response(
                since,
                store,
                format.as_deref(),
            )
            .await,
        );
    }
    if req.starts_with("GET /audit/by-day-by-kind ")
        || req.starts_with("GET /audit/by-day-by-kind?")
    {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let days = parse_query_kv(tail, "days")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(30);
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_audit_rollups::audit_by_day_by_kind_response(
                days,
                store,
                format.as_deref(),
            )
            .await,
        );
    }
    None
}

async fn dispatch_audit_by_session_tool(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("GET /audit/by-session ") || req.starts_with("GET /audit/by-session?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let limit: u32 = parse_query_kv(tail, "limit")
            .and_then(|v| v.parse().ok())
            .unwrap_or(50);
        let since = parse_query_kv(tail, "since-minutes").and_then(|v| v.parse::<i64>().ok());
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
    if req.starts_with("GET /audit/by-tool ") || req.starts_with("GET /audit/by-tool?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let since = parse_query_kv(tail, "since-minutes").and_then(|v| v.parse::<i64>().ok());
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_audit_rollups::audit_by_tool_response(since, store, format.as_deref())
                .await,
        );
    }
    if req.starts_with("GET /audit/tool-errors ") || req.starts_with("GET /audit/tool-errors?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let since = parse_query_kv(tail, "since-minutes").and_then(|v| v.parse::<i64>().ok());
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_audit_rollups::audit_tool_errors_response(
                since,
                store,
                format.as_deref(),
            )
            .await,
        );
    }
    None
}

/// `/forge/runs/by-day`, `/forge/runs/by-status`, `/forge/runs/by-skill`.
pub(super) async fn dispatch_forge_rollups(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("GET /forge/runs/by-day ") || req.starts_with("GET /forge/runs/by-day?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let days = parse_query_kv(tail, "days")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(30);
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_forge::forge_runs_by_day_response(days, store, format.as_deref()).await,
        );
    }
    if req.starts_with("GET /forge/runs/by-status ")
        || req.starts_with("GET /forge/runs/by-status?")
    {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let since = parse_query_kv(tail, "since-minutes").and_then(|v| v.parse::<i64>().ok());
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_forge::forge_runs_by_status_response(since, store, format.as_deref())
                .await,
        );
    }
    if req.starts_with("GET /forge/runs/by-skill ") || req.starts_with("GET /forge/runs/by-skill?")
    {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let since = parse_query_kv(tail, "since-minutes").and_then(|v| v.parse::<i64>().ok());
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
