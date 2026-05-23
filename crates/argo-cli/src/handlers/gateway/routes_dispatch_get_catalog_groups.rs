//! Resource-grouped sub-dispatchers for
//! [`super::routes_dispatch_get_catalog::dispatch_catalog_get`].
//! Each function owns one URL family (verticals / sessions /
//! skills / memory / providers / tools / cron) so the parent
//! dispatcher stays a thin fan-out and every helper stays well
//! under the 80-LOC ceiling.
use super::routes_lists::{memory_response, memory_versions_response, skills_response};
use super::routes_meta::{skill_versions_response, tool_show_response, tools_response};
use super::routes_providers::{provider_models_response, providers_response};
use super::routes_query::{parse_query_kv, parse_since_minutes_from_get};
use super::routes_top::dispatch_sessions;
use argo_store::SessionStore;
use std::sync::Arc;

/// `/verticals`, `/verticals/<id>`, `/verticals/<id>/{lint,doctor}`.
pub(super) async fn dispatch_verticals(
    req: &str,
) -> Option<(&'static str, String)> {
    if req.starts_with("GET /verticals ") || req.starts_with("GET /verticals?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let format = parse_query_kv(tail, "format");
        return Some(super::routes_verticals::list_response(format.as_deref()).await);
    }
    if let Some(rest) = req.strip_prefix("GET /verticals/") {
        if let Some((id_with_query, _)) = rest.split_once(' ') {
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
    None
}

/// `/sessions/count`, `/sessions`, plus delegated rollups.
pub(super) async fn dispatch_sessions_family(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("GET /sessions/count ") || req.starts_with("GET /sessions/count?") {
        return Some(super::routes_counts::dispatch_sessions_count(req, store).await);
    }
    if let Some(resp) = super::routes_dispatch_rollups::dispatch_rollups(req, store).await {
        return Some(resp);
    }
    if req.starts_with("GET /sessions ") || req.starts_with("GET /sessions?") {
        return Some(dispatch_sessions(req, store).await);
    }
    None
}

/// `/skills/count`, `/skills`, `/skills/<name>`,
/// `/skills/<name>/{versions,quality}`.
pub(super) async fn dispatch_skills_family(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
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
    None
}

/// `/memory/count`, `/memory`, `/memory/<id>`,
/// `/memory/<id>/versions`.
pub(super) async fn dispatch_memory_family(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
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
    None
}

/// `/providers/count`, `/providers`, `/providers/<id>`,
/// `/providers/<id>/models`.
pub(super) async fn dispatch_providers_family(
    req: &str,
) -> Option<(&'static str, String)> {
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
    None
}

/// `/tools/count`, `/tools`, `/tools/<name>`.
pub(super) async fn dispatch_tools_family(
    req: &str,
) -> Option<(&'static str, String)> {
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
    None
}

/// `/cron`, `/cron/<name>`.
pub(super) async fn dispatch_cron_family(
    req: &str,
) -> Option<(&'static str, String)> {
    use super::routes_cron::{cron_list_response, cron_show_response};
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
