//! GET dispatcher for catalog-style resources: verticals, sessions,
//! skills, memory, providers, tools, and cron definitions.
use super::routes_cron::{cron_list_response, cron_show_response};
use super::routes_lists::{memory_response, memory_versions_response, skills_response};
use super::routes_meta::{skill_versions_response, tool_show_response, tools_response};
use super::routes_providers::{provider_models_response, providers_response};
use super::routes_query::{parse_query_kv, parse_since_minutes_from_get};
use super::routes_top::dispatch_sessions;
use argo_store::SessionStore;
use std::sync::Arc;

pub(super) async fn dispatch_catalog_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
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
                super::routes_meta::skill_show_response_with_format(name, store, format.as_deref())
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
                super::routes_meta::memory_show_response_with_format(id, store, format.as_deref())
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
                super::routes_providers::provider_show_response_with_format(id, format.as_deref())
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
    None
}
