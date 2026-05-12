//! Per-resource GET dispatch lifted out of `routes.rs` to keep
//! that file under the 500-LOC budget. Pure router — every match
//! arm forwards to a handler defined in another sibling
//! module. Returns `Some(resp)` on a match, `None` otherwise so
//! `routes.rs` can fall through to `dispatch_get_admin`,
//! `dispatch_post`, `dispatch_session_get`, and ultimately the
//! 404 default.
//!
//! Holds the GETs that map to a domain resource: sessions,
//! skills, memory, providers, tools, verticals, cron,
//! forge/runs, audit/sessions/<id>. The introspection/health/
//! catalog endpoints (/, /version, /health/*, /doctor, /db/*,
//! /metrics/json, /routes, /audit{,breaks,recent,stream,count},
//! /search{,/count}, /workspace/{info,artifacts}) live in the
//! sibling `routes_dispatch_get_admin`.
use super::routes_audit::{audit_session_response, audit_session_verify_response};
use super::routes_cron::{cron_list_response, cron_show_response};
use super::routes_lists::{memory_response, memory_versions_response, skills_response};
use super::routes_meta::{
    forge_runs_response, skill_versions_response, tool_show_response, tools_response,
};
use super::routes_providers::{provider_models_response, providers_response};
use super::routes_query::{parse_query_kv, parse_since_minutes_from_get};
use super::routes_top::dispatch_sessions;
use argo_store::SessionStore;
use std::sync::Arc;

/// Dispatch domain-resource GETs. Returns `None` when no match.
pub(super) async fn dispatch_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    // J1 — JSON approvals API. The HTML `/inbox` route stays mounted
    // in `routes_channels_webinbox` for legacy operators.
    if req.starts_with("GET /api/v1/approvals ") || req.starts_with("GET /api/v1/approvals?") {
        return Some(super::routes_approvals_json::list_response(req).await);
    }
    // J14 — audit feed with 7 filter dims; unblocks E5-a from mock.
    if req.starts_with("GET /api/v1/audit ") || req.starts_with("GET /api/v1/audit?") {
        return Some(super::routes_audit_feed::list_response(req, store).await);
    }
    if req.starts_with("GET /verticals ") || req.starts_with("GET /verticals?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return Some(super::routes_verticals::list_response(format.as_deref()).await);
    }
    if let Some(rest) = req.strip_prefix("GET /verticals/") {
        if let Some((id_with_query, _)) = rest.split_once(' ') {
            // /verticals/<id>/{lint,doctor} mirrors of CLI #671/#673.
            if let Some(id) = id_with_query.strip_suffix("/lint") {
                return Some(super::routes_verticals_lint::vertical_lint_response(id).await);
            }
            if let Some(id) = id_with_query.strip_suffix("/doctor") {
                return Some(super::routes_verticals_doctor::vertical_doctor_response(id).await);
            }
            let (id, query) = match id_with_query.split_once('?') {
                Some((i, q)) => (i, Some(q)),
                None => (id_with_query, None),
            };
            let format = query
                .and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("format=")))
                .map(str::to_string);
            return Some(super::routes_verticals::show_response(id, format.as_deref()).await);
        }
    }
    if req.starts_with("GET /sessions/count ") || req.starts_with("GET /sessions/count?") {
        return Some(super::routes_counts::dispatch_sessions_count(req, store).await);
    }
    if let Some(resp) = super::routes_dispatch_rollups::dispatch_rollups(req, store).await {
        return Some(resp);
    }
    if req.starts_with("GET /sessions ") || req.starts_with("GET /sessions?") {
        return Some(dispatch_sessions(req, store).await);
    }
    if req.starts_with("GET /skills/count ") || req.starts_with("GET /skills/count?") {
        return Some(super::routes_counts::dispatch_skills_count(req, store).await);
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
        return Some(
            skills_response(
                store,
                include_superseded,
                name_contains.as_deref(),
                lock.as_deref(),
                body_contains.as_deref(),
                limit,
                format.as_deref(),
            )
            .await,
        );
    }
    if let Some(rest) = req.strip_prefix("GET /skills/") {
        // /skills/<name>/versions and /skills/<name>/quality take
        // priority over the bare /skills/<name> single-skill view.
        if let Some((name, tail)) = rest.split_once('/') {
            if tail.starts_with("versions ") || tail.starts_with("versions?") {
                let format = parse_query_kv(tail, "format");
                return Some(skill_versions_response(name, store, format.as_deref()).await);
            }
            if tail.starts_with("quality ") || tail.starts_with("quality?") {
                let limit = parse_query_kv(tail, "limit").and_then(|v| v.parse::<usize>().ok());
                return Some(
                    super::routes_skills_quality::skill_quality_response(name, store, limit).await,
                );
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
            return Some(
                super::routes_meta::skill_show_response_with_format(
                    name,
                    store,
                    format.as_deref(),
                )
                .await,
            );
        }
    }
    if req.starts_with("GET /memory/count ") || req.starts_with("GET /memory/count?") {
        return Some(super::routes_counts::dispatch_memory_count(req, store).await);
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
        return Some(
            memory_response(
                store,
                file,
                since,
                lock,
                body_contains,
                limit,
                format.as_deref(),
                vertical,
            )
            .await,
        );
    }
    if let Some(rest) = req.strip_prefix("GET /memory/") {
        if let Some((id, tail)) = rest.split_once('/') {
            if tail.starts_with("versions ") || tail.starts_with("versions?") {
                let format = parse_query_kv(tail, "format");
                return Some(memory_versions_response(id, store, format.as_deref()).await);
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
            return Some(
                super::routes_meta::memory_show_response_with_format(
                    id,
                    store,
                    format.as_deref(),
                )
                .await,
            );
        }
    }
    if req.starts_with("GET /providers/count ") || req.starts_with("GET /providers/count?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let family = parse_query_kv(tail, "family");
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_providers::providers_count_response(family.as_deref(), format.as_deref())
                .await,
        );
    }
    if req.starts_with("GET /providers ") || req.starts_with("GET /providers?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let family = parse_query_kv(tail, "family");
        let format = parse_query_kv(tail, "format");
        return Some(providers_response(family.as_deref(), format.as_deref()).await);
    }
    if let Some(rest) = req.strip_prefix("GET /providers/") {
        // `/providers/<id>/models` takes priority over the bare
        // `/providers/<id>` manifest view.
        if let Some((id, tail)) = rest.split_once('/') {
            if tail.starts_with("models ") || tail.starts_with("models?") {
                let format = parse_query_kv(tail, "format");
                return Some(provider_models_response(id, format.as_deref()).await);
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
            return Some(
                super::routes_providers::provider_show_response_with_format(
                    id,
                    format.as_deref(),
                )
                .await,
            );
        }
    }
    if req.starts_with("GET /tools/count ") || req.starts_with("GET /tools/count?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let name_contains = parse_query_kv(tail, "name-contains").map(|s| s.replace('+', " "));
        let format = parse_query_kv(tail, "format");
        return Some(
            super::routes_meta::tools_count_response(name_contains.as_deref(), format.as_deref())
                .await,
        );
    }
    if req.starts_with("GET /tools ") || req.starts_with("GET /tools?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let name_contains = parse_query_kv(tail, "name-contains").map(|s| s.replace('+', " "));
        let format = parse_query_kv(tail, "format");
        return Some(tools_response(name_contains.as_deref(), format.as_deref()).await);
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
            return Some(tool_show_response(name, format.as_deref()).await);
        }
    }
    if req.starts_with("GET /cron ") || req.starts_with("GET /cron?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return Some(cron_list_response(format.as_deref()).await);
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
            return Some(cron_show_response(name, format.as_deref()).await);
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
        let has_candidate =
            parse_query_kv(tail, "has-candidate").map(|v| v == "true" || v == "1");
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
    // module — keeps this file under the 500-LOC ceiling.
    if let Some(resp) = super::routes_dispatch_costs::dispatch_costs(req, store).await {
        return Some(resp);
    }
    None
}
