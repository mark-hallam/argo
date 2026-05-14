//! GET dispatcher for forge run views, legacy audit-session views,
//! and cost rollups.
use super::routes_audit::{audit_session_response, audit_session_verify_response};
use super::routes_meta::forge_runs_response;
use super::routes_query::{parse_query_kv, parse_since_minutes_from_get};
use argo_store::SessionStore;
use std::sync::Arc;

pub(super) async fn dispatch_forge_audit_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("GET /forge/runs ") || req.starts_with("GET /forge/runs?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let limit: u32 = parse_query_kv(tail, "limit")
            .and_then(|v| v.parse().ok())
            .unwrap_or(50);
        let since = parse_since_minutes_from_get(req, "/forge/runs");
        let status = parse_query_kv(tail, "status");
        let score_min = parse_query_kv(tail, "score-min").and_then(|v| v.parse::<f64>().ok());
        let score_max = parse_query_kv(tail, "score-max").and_then(|v| v.parse::<f64>().ok());
        let skill = parse_query_kv(tail, "skill");
        let has_candidate = parse_query_kv(tail, "has-candidate").map(|v| v == "true" || v == "1");
        let iterations_min =
            parse_query_kv(tail, "iterations-min").and_then(|v| v.parse::<i64>().ok());
        let iterations_max =
            parse_query_kv(tail, "iterations-max").and_then(|v| v.parse::<i64>().ok());
        let format = parse_query_kv(tail, "format");
        return Some(
            forge_runs_response(
                limit,
                since,
                status.as_deref(),
                score_min,
                score_max,
                skill.as_deref(),
                has_candidate,
                iterations_min,
                iterations_max,
                store,
                format.as_deref(),
            )
            .await,
        );
    }
    if req.starts_with("GET /forge/runs/count ") || req.starts_with("GET /forge/runs/count?") {
        return Some(super::routes_counts::dispatch_forge_runs_count(req, store).await);
    }
    if let Some(resp) = super::routes_dispatch_forge::dispatch_forge_run_get(req, store).await {
        return Some(resp);
    }
    if let Some(rest) = req.strip_prefix("GET /audit/sessions/") {
        // GET /audit/sessions/<id>/verify takes priority over the
        // bare GET /audit/sessions/<id> rows listing.
        if let Some((sid, tail)) = rest.split_once('/') {
            if tail.starts_with("verify ") || tail.starts_with("verify?") {
                let include_breaks = parse_query_kv(tail, "include-breaks")
                    .map(|v| v == "true" || v == "1")
                    .unwrap_or(false);
                let format = parse_query_kv(tail, "format");
                return Some(
                    audit_session_verify_response(sid, include_breaks, store, format.as_deref())
                        .await,
                );
            }
        }
        // Strip the (optional) ?query and trailing " HTTP/...".
        let (sid_with_q, _) = rest.split_once(' ').unwrap_or((rest, ""));
        let (sid, query_part) = sid_with_q
            .split_once('?')
            .map_or((sid_with_q, ""), |(s, q)| (s, q));
        let (mut kind, mut since, mut format): (Option<String>, Option<i64>, Option<String>) =
            (None, None, None);
        if !query_part.is_empty() {
            for kv in query_part.split('&') {
                if let Some(v) = kv.strip_prefix("kind=") {
                    kind = Some(v.to_string());
                } else if let Some(v) = kv.strip_prefix("since-minutes=") {
                    since = v.parse().ok();
                } else if let Some(v) = kv.strip_prefix("format=") {
                    format = Some(v.to_string());
                }
            }
        }
        return Some(
            audit_session_response(sid, store, kind.as_deref(), since, format.as_deref()).await,
        );
    }
    // /costs/* dispatch lives in the sibling `routes_dispatch_costs`
    // module — keeps the main GET dispatcher under the 500-LOC ceiling.
    if let Some(resp) = super::routes_dispatch_costs::dispatch_costs(req, store).await {
        return Some(resp);
    }
    None
}
