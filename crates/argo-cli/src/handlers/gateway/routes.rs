//! HTTP route dispatch + GET response builders. POST handlers +
//! the heavier query helpers (audit-session, costs-breakdown)
//! live in the sibling `routes_post` module to keep this file
//! under the 500-LOC budget. Each handler returns a
//! `(status_line, body)` pair the listener loop formats into a
//! single-line HTTP/1.1 response.
use super::routes_meta::{
    db_stats_response, forge_runs_response, routes_response, search_response,
    skill_versions_response, tool_show_response, tools_response,
};
use super::routes_providers::{
    provider_models_response, providers_response,
};
use super::routes_lists::{memory_response, memory_versions_response, skills_response};
use super::routes_workspace::{artifacts_response, workspace_info_response};
use super::routes_audit::{audit_session_response, audit_session_verify_response};
use super::routes_cron::{cron_list_response, cron_show_response};
use super::routes_session_get::dispatch_session_get;
use super::routes_top::{
    audit_breaks_response, audit_response, dispatch_sessions, doctor_response,
    health_full_response,
};
use super::routes_query::{parse_query_kv, parse_search_args, parse_since_minutes_from_get};
use super::routes_dispatch_post::dispatch_post;
use argo_store::SessionStore;
use std::sync::Arc;

pub(super) async fn dispatch_route(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    if req.starts_with("GET / ") {
        // Root landing — closes the discoverability papercut where
        // pointing a browser at the gateway returned a bare 404 with
        // no hint of what to try next. Sends users straight to /routes
        // for the full catalog and names a few high-frequency
        // entry-points so a curl at `/` is immediately useful.
        let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
        return (
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
        );
    }
    if req.starts_with("GET /version ") {
        let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
        return (
            "200 OK",
            format!(
                "{{\"version\":\"{}\",\"profile\":\"{profile}\"}}",
                env!("CARGO_PKG_VERSION")
            ),
        );
    }
    if req.starts_with("GET /health/live ") {
        return super::routes_health::live_response();
    }
    if req.starts_with("GET /health/ready ") {
        return super::routes_health::ready_response(store).await;
    }
    if req.starts_with("GET /health ") {
        return super::routes_health::health_response();
    }
    if req.starts_with("GET /verticals ") || req.starts_with("GET /verticals?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return super::routes_verticals::list_response(format.as_deref()).await;
    }
    if let Some(rest) = req.strip_prefix("GET /verticals/") {
        if let Some((id_with_query, _)) = rest.split_once(' ') {
            // /verticals/<id>/{lint,doctor} mirrors of CLI #671/#673.
            if let Some(id) = id_with_query.strip_suffix("/lint") {
                return super::routes_verticals_lint::vertical_lint_response(id).await;
            }
            if let Some(id) = id_with_query.strip_suffix("/doctor") {
                return super::routes_verticals_doctor::vertical_doctor_response(id).await;
            }
            let (id, query) = match id_with_query.split_once('?') {
                Some((i, q)) => (i, Some(q)),
                None => (id_with_query, None),
            };
            let format = query
                .and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("format=")))
                .map(str::to_string);
            return super::routes_verticals::show_response(id, format.as_deref()).await;
        }
    }
    if req.starts_with("GET /sessions/count ") || req.starts_with("GET /sessions/count?") {
        return super::routes_counts::dispatch_sessions_count(req, store).await;
    }
    if let Some(resp) =
        super::routes_dispatch_rollups::dispatch_rollups(req, store).await
    {
        return resp;
    }
    if req.starts_with("GET /sessions ") || req.starts_with("GET /sessions?") {
        return dispatch_sessions(req, store).await;
    }
    if req.starts_with("GET /skills/count ") || req.starts_with("GET /skills/count?") {
        return super::routes_counts::dispatch_skills_count(req, store).await;
    }
    if req.starts_with("GET /skills ") || req.starts_with("GET /skills?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let include_superseded = parse_query_kv(tail, "include-superseded")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        let name_contains = parse_query_kv(tail, "name-contains").map(|s| s.replace('+', " "));
        let lock = parse_query_kv(tail, "lock");
        let body_contains = parse_query_kv(tail, "body-contains").map(|s| s.replace('+', " "));
        let limit = parse_query_kv(tail, "limit").and_then(|v| v.parse::<usize>().ok());
        let format = parse_query_kv(tail, "format");
        return skills_response(
            store,
            include_superseded,
            name_contains.as_deref(),
            lock.as_deref(),
            body_contains.as_deref(),
            limit,
            format.as_deref(),
        )
        .await;
    }
    if let Some(rest) = req.strip_prefix("GET /skills/") {
        // /skills/<name>/versions takes priority over the bare
        // /skills/<name> single-skill view.
        if let Some((name, tail)) = rest.split_once('/') {
            if tail.starts_with("versions ") || tail.starts_with("versions?") {
                let format = parse_query_kv(tail, "format");
                return skill_versions_response(name, store, format.as_deref()).await;
            }
        }
        if let Some((name_with_query, _)) = rest.split_once(' ') {
            let (name, query) = match name_with_query.split_once('?') {
                Some((n, q)) => (n, Some(q)),
                None => (name_with_query, None),
            };
            let format = query
                .and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("format=")))
                .map(str::to_string);
            return super::routes_meta::skill_show_response_with_format(
                name,
                store,
                format.as_deref(),
            )
            .await;
        }
    }
    if req.starts_with("GET /memory/count ") || req.starts_with("GET /memory/count?") {
        return super::routes_counts::dispatch_memory_count(req, store).await;
    }
    if req.starts_with("GET /memory ") || req.starts_with("GET /memory?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let file = parse_query_kv(tail, "file");
        let since = parse_since_minutes_from_get(req, "/memory");
        let lock = parse_query_kv(tail, "lock");
        let body_contains = parse_query_kv(tail, "body-contains").map(|s| s.replace('+', " "));
        let limit = parse_query_kv(tail, "limit").and_then(|v| v.parse::<usize>().ok());
        let format = parse_query_kv(tail, "format");
        // #687 — vertical-aware filter mirrors CLI #686.
        let vertical = parse_query_kv(tail, "vertical");
        return memory_response(
            store,
            file,
            since,
            lock,
            body_contains,
            limit,
            format.as_deref(),
            vertical,
        )
        .await;
    }
    if let Some(rest) = req.strip_prefix("GET /memory/") {
        if let Some((id, tail)) = rest.split_once('/') {
            if tail.starts_with("versions ") || tail.starts_with("versions?") {
                let format = parse_query_kv(tail, "format");
                return memory_versions_response(id, store, format.as_deref()).await;
            }
        }
        if let Some((id_with_query, _)) = rest.split_once(' ') {
            let (id, query) = match id_with_query.split_once('?') {
                Some((i, q)) => (i, Some(q)),
                None => (id_with_query, None),
            };
            let format = query
                .and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("format=")))
                .map(str::to_string);
            return super::routes_meta::memory_show_response_with_format(
                id,
                store,
                format.as_deref(),
            )
            .await;
        }
    }
    if req.starts_with("GET /audit ") || req.starts_with("GET /audit?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return audit_response(store, format.as_deref()).await;
    }
    if req.starts_with("GET /audit/breaks ") || req.starts_with("GET /audit/breaks?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return audit_breaks_response(store, format.as_deref()).await;
    }
    if req.starts_with("GET /audit/count ") || req.starts_with("GET /audit/count?") {
        return super::routes_counts::dispatch_audit_count(req, store).await;
    }
    if req.starts_with("GET /audit/recent ") || req.starts_with("GET /audit/recent?") {
        return super::routes_audit::dispatch_audit_recent(req, store).await;
    }
    if req.starts_with("GET /doctor ") || req.starts_with("GET /doctor?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return doctor_response(format.as_deref()).await;
    }
    if req.starts_with("GET /health/full ") {
        return health_full_response().await;
    }
    if req.starts_with("GET /providers/count ") || req.starts_with("GET /providers/count?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let family = parse_query_kv(tail, "family");
        let format = parse_query_kv(tail, "format");
        return super::routes_providers::providers_count_response(
            family.as_deref(),
            format.as_deref(),
        )
        .await;
    }
    if req.starts_with("GET /providers ") || req.starts_with("GET /providers?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let family = parse_query_kv(tail, "family");
        let format = parse_query_kv(tail, "format");
        return providers_response(family.as_deref(), format.as_deref()).await;
    }
    if let Some(rest) = req.strip_prefix("GET /providers/") {
        // `/providers/<id>/models` takes priority over the bare
        // `/providers/<id>` manifest view.
        if let Some((id, tail)) = rest.split_once('/') {
            if tail.starts_with("models ") || tail.starts_with("models?") {
                let format = parse_query_kv(tail, "format");
                return provider_models_response(id, format.as_deref()).await;
            }
        }
        if let Some((id_with_query, _)) = rest.split_once(' ') {
            let (id, query) = match id_with_query.split_once('?') {
                Some((i, q)) => (i, Some(q)),
                None => (id_with_query, None),
            };
            let format = query
                .and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("format=")))
                .map(str::to_string);
            return super::routes_providers::provider_show_response_with_format(
                id,
                format.as_deref(),
            )
            .await;
        }
    }
    if req.starts_with("GET /tools/count ") || req.starts_with("GET /tools/count?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let name_contains = parse_query_kv(tail, "name-contains").map(|s| s.replace('+', " "));
        let format = parse_query_kv(tail, "format");
        return super::routes_meta::tools_count_response(
            name_contains.as_deref(),
            format.as_deref(),
        )
        .await;
    }
    if req.starts_with("GET /tools ") || req.starts_with("GET /tools?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let name_contains = parse_query_kv(tail, "name-contains").map(|s| s.replace('+', " "));
        let format = parse_query_kv(tail, "format");
        return tools_response(name_contains.as_deref(), format.as_deref()).await;
    }
    if let Some(rest) = req.strip_prefix("GET /tools/") {
        if let Some((name_with_query, _)) = rest.split_once(' ') {
            // Strip the query string before passing to the show
            // handler — it handles `?format` parsing of the full
            // request path itself.
            let (name, query) = match name_with_query.split_once('?') {
                Some((n, q)) => (n, Some(q)),
                None => (name_with_query, None),
            };
            let format = query
                .and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("format=")))
                .map(str::to_string);
            return tool_show_response(name, format.as_deref()).await;
        }
    }
    if req.starts_with("GET /db/stats ") || req.starts_with("GET /db/stats?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return db_stats_response(store, format.as_deref()).await;
    }
    if req.starts_with("GET /db/version ") || req.starts_with("GET /db/version?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return super::routes_meta::db_version_response(store, format.as_deref()).await;
    }
    if req.starts_with("GET /db/integrity ") || req.starts_with("GET /db/integrity?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return super::routes_meta::db_integrity_response(store, format.as_deref()).await;
    }
    if req.starts_with("GET /metrics/json ") || req.starts_with("GET /metrics/json?") {
        let body = super::metrics::render_json(store).await;
        return ("200 OK", body);
    }
    if req.starts_with("GET /routes ") || req.starts_with("GET /routes?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let method = parse_query_kv(tail, "method");
        let format = parse_query_kv(tail, "format");
        if matches!(format.as_deref(), Some("markdown")) {
            return super::routes_catalog::routes_markdown(method.as_deref());
        }
        return routes_response(method.as_deref());
    }
    if req.starts_with("GET /search/count ") || req.starts_with("GET /search/count?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let q = parse_query_kv(tail, "q").map(|s| s.replace('+', " "));
        let session = parse_query_kv(tail, "session");
        let role = parse_query_kv(tail, "role");
        let format = parse_query_kv(tail, "format");
        return super::routes_search::search_count_response(
            q.as_deref(),
            session.as_deref(),
            role.as_deref(),
            store,
            format.as_deref(),
        )
        .await;
    }
    if req.starts_with("GET /search ") || req.starts_with("GET /search?") {
        let (q, limit, session) = parse_search_args(req);
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let role = parse_query_kv(tail, "role");
        let format = parse_query_kv(tail, "format");
        return search_response(
            q.as_deref(),
            limit,
            session.as_deref(),
            role.as_deref(),
            store,
            format.as_deref(),
        )
        .await;
    }
    if req.starts_with("GET /workspace/info ") || req.starts_with("GET /workspace/info?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return workspace_info_response(store, format.as_deref()).await;
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
        return artifacts_response(limit, since, store, format.as_deref()).await;
    }
    if req.starts_with("GET /cron ") || req.starts_with("GET /cron?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return cron_list_response(format.as_deref()).await;
    }
    if let Some(rest) = req.strip_prefix("GET /cron/") {
        if let Some((name_with_query, _)) = rest.split_once(' ') {
            let (name, query) = match name_with_query.split_once('?') {
                Some((n, q)) => (n, Some(q)),
                None => (name_with_query, None),
            };
            let format = query
                .and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("format=")))
                .map(str::to_string);
            return cron_show_response(name, format.as_deref()).await;
        }
    }
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
        let has_candidate = parse_query_kv(tail, "has-candidate")
            .map(|v| v == "true" || v == "1");
        let iterations_min =
            parse_query_kv(tail, "iterations-min").and_then(|v| v.parse::<i64>().ok());
        let iterations_max =
            parse_query_kv(tail, "iterations-max").and_then(|v| v.parse::<i64>().ok());
        let format = parse_query_kv(tail, "format");
        return forge_runs_response(
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
        .await;
    }
    if req.starts_with("GET /forge/runs/count ")
        || req.starts_with("GET /forge/runs/count?")
    {
        return super::routes_counts::dispatch_forge_runs_count(req, store).await;
    }
    if let Some(resp) = super::routes_dispatch_forge::dispatch_forge_run_get(req, store).await {
        return resp;
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
                return audit_session_verify_response(
                    sid,
                    include_breaks,
                    store,
                    format.as_deref(),
                )
                .await;
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
        return audit_session_response(
            sid,
            store,
            kind.as_deref(),
            since,
            format.as_deref(),
        )
        .await;
    }
    // /costs/* dispatch lives in the sibling
    // `routes_dispatch_costs` module — keeps this file under the
    // 500-LOC ceiling.
    if let Some(resp) = super::routes_dispatch_costs::dispatch_costs(req, store).await {
        return resp;
    }
    // All POST verbs live in the sibling `routes_dispatch_post`
    // module so this file stays under the 500-LOC budget.
    if let Some(resp) = dispatch_post(req, store).await {
        return resp;
    }
    // GET /sessions/<uuid>/{spend,spend/total,messages,messages/last,skills,judge}
    // and bare GET /sessions/<uuid> are all dispatched in one helper.
    if let Some(resp) = dispatch_session_get(req, store).await {
        return resp;
    }
    ("404 Not Found", "{\"error\":\"not found\"}".to_string())
}

