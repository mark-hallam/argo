//! HTTP route dispatch + GET response builders. POST handlers +
//! the heavier query helpers (audit-session, costs-breakdown)
//! live in the sibling `routes_post` module to keep this file
//! under the 500-LOC budget. Each handler returns a
//! `(status_line, body)` pair the listener loop formats into a
//! single-line HTTP/1.1 response.
use super::routes_meta::{
    costs_response, db_stats_response, forge_run_show_response, forge_runs_response,
    memory_show_response, routes_response, search_response, skill_show_response,
    skill_versions_response, tool_show_response, tools_response,
};
use super::routes_providers::{
    provider_models_response, provider_show_response, providers_response,
};
use super::routes_lists::{memory_response, memory_versions_response, skills_response};
use super::routes_workspace::{artifacts_response, workspace_info_response};
use super::routes_audit::{
    audit_recent_response, audit_session_response, audit_session_verify_response,
};
use super::routes_post::{
    append_message_response, costs_breakdown_response, create_session_response, replay_response,
    run_turn_response, session_judge_response, steer_response,
};
use super::routes_cron::{
    cron_create_response, cron_delete_response, cron_list_response, cron_show_response,
};
use super::routes_msgs::{last_message_response, list_messages_response};
use super::routes_post_admin::{
    judge_override_response, memory_edit_response, memory_forget_response, memory_lock_response,
    memory_unlock_response, skill_lock_response, skill_unlock_response,
};
use super::routes_skill_refine::skill_refine_response;
use super::routes_top::{audit_response, doctor_response, health_full_response, sessions_response};
use super::routes_session_skills::{
    session_attach_skill_response, session_clear_skills_response, session_delete_response,
    session_detach_skill_response, session_end_all_response, session_end_response,
    session_set_title_response, session_skills_list_response,
};
use super::routes_query::{
    parse_limit, parse_query_kv, parse_role, parse_search_args, parse_since_minutes_from_get,
};
use super::routes_forge_promote::{forge_abort_all_response, forge_promote_response};
use super::routes_post_writes::{
    append_memory_response, db_prune_response, db_vacuum_response, forge_abort_response,
    forge_bench_response, forge_optimize_response, install_skill_response, skill_promote_response,
    workspace_init_response,
};
use super::routes_session::{
    session_show_response, session_spend_response, session_spend_total_response,
};
use argo_store::SessionStore;
use std::sync::Arc;

pub(super) async fn dispatch_route(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
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
    if req.starts_with("GET /health ") {
        let now = chrono::Utc::now();
        return (
            "200 OK",
            format!(
                "{{\"status\":\"ok\",\"time\":\"{}\",\"epoch_micros\":{}}}",
                now.to_rfc3339(),
                now.timestamp_micros()
            ),
        );
    }
    if req.starts_with("GET /sessions ") || req.starts_with("GET /sessions?") {
        let since = parse_since_minutes_from_get(req, "/sessions");
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let status = parse_query_kv(tail, "status");
        let provider = parse_query_kv(tail, "provider");
        let model = parse_query_kv(tail, "model");
        let title_contains = parse_query_kv(tail, "title-contains").map(|s| s.replace('+', " "));
        let limit: u32 = parse_query_kv(tail, "limit")
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);
        return sessions_response(
            store,
            since,
            status.as_deref(),
            provider.as_deref(),
            model.as_deref(),
            title_contains.as_deref(),
            limit,
        )
        .await;
    }
    if req.starts_with("GET /skills ") || req.starts_with("GET /skills?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let include_superseded = parse_query_kv(tail, "include-superseded")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        let name_contains = parse_query_kv(tail, "name-contains").map(|s| s.replace('+', " "));
        let lock = parse_query_kv(tail, "lock");
        let body_contains = parse_query_kv(tail, "body-contains").map(|s| s.replace('+', " "));
        return skills_response(
            store,
            include_superseded,
            name_contains.as_deref(),
            lock.as_deref(),
            body_contains.as_deref(),
        )
        .await;
    }
    if let Some(rest) = req.strip_prefix("GET /skills/") {
        // /skills/<name>/versions takes priority over the bare
        // /skills/<name> single-skill view.
        if let Some((name, tail)) = rest.split_once('/') {
            if tail.starts_with("versions ") {
                return skill_versions_response(name, store).await;
            }
        }
        if let Some((name, _)) = rest.split_once(' ') {
            return skill_show_response(name, store).await;
        }
    }
    if req.starts_with("GET /memory ") || req.starts_with("GET /memory?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let file = parse_query_kv(tail, "file");
        let since = parse_since_minutes_from_get(req, "/memory");
        let lock = parse_query_kv(tail, "lock");
        let body_contains = parse_query_kv(tail, "body-contains").map(|s| s.replace('+', " "));
        return memory_response(store, file, since, lock, body_contains).await;
    }
    if let Some(rest) = req.strip_prefix("GET /memory/") {
        if let Some((id, tail)) = rest.split_once('/') {
            if tail.starts_with("versions ") {
                return memory_versions_response(id, store).await;
            }
        }
        if let Some((id, _)) = rest.split_once(' ') {
            return memory_show_response(id, store).await;
        }
    }
    if req.starts_with("GET /audit ") {
        return audit_response(store).await;
    }
    if req.starts_with("GET /audit/recent ") || req.starts_with("GET /audit/recent?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let limit: u32 = parse_query_kv(tail, "limit")
            .and_then(|v| v.parse().ok())
            .unwrap_or(50);
        let kind = parse_query_kv(tail, "kind");
        let since = parse_since_minutes_from_get(req, "/audit/recent");
        let session = parse_query_kv(tail, "session");
        return audit_recent_response(
            limit,
            kind.as_deref(),
            since,
            session.as_deref(),
            store,
        )
        .await;
    }
    if req.starts_with("GET /doctor ") {
        return doctor_response().await;
    }
    if req.starts_with("GET /health/full ") {
        return health_full_response().await;
    }
    if req.starts_with("GET /providers ") || req.starts_with("GET /providers?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let family = parse_query_kv(tail, "family");
        return providers_response(family.as_deref()).await;
    }
    if let Some(rest) = req.strip_prefix("GET /providers/") {
        // `/providers/<id>/models` takes priority over the bare
        // `/providers/<id>` manifest view.
        if let Some((id, tail)) = rest.split_once('/') {
            if tail.starts_with("models ") {
                return provider_models_response(id).await;
            }
        }
        if let Some((id, _)) = rest.split_once(' ') {
            return provider_show_response(id).await;
        }
    }
    if req.starts_with("GET /tools ") || req.starts_with("GET /tools?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let name_contains = parse_query_kv(tail, "name-contains").map(|s| s.replace('+', " "));
        return tools_response(name_contains.as_deref()).await;
    }
    if let Some(rest) = req.strip_prefix("GET /tools/") {
        if let Some((name, _)) = rest.split_once(' ') {
            return tool_show_response(name).await;
        }
    }
    if req.starts_with("GET /db/stats ") {
        return db_stats_response(store).await;
    }
    if req.starts_with("GET /routes ") || req.starts_with("GET /routes?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let method = parse_query_kv(tail, "method");
        return routes_response(method.as_deref());
    }
    if req.starts_with("GET /search ") || req.starts_with("GET /search?") {
        let (q, limit, session) = parse_search_args(req);
        return search_response(q.as_deref(), limit, session.as_deref(), store).await;
    }
    if req.starts_with("GET /workspace/info ") {
        return workspace_info_response(store).await;
    }
    if req.starts_with("GET /workspace/artifacts ")
        || req.starts_with("GET /workspace/artifacts?")
    {
        let limit: u32 = req
            .strip_prefix("GET ")
            .and_then(|tail| parse_query_kv(tail, "limit"))
            .and_then(|v| v.parse().ok())
            .unwrap_or(25);
        let since = parse_since_minutes_from_get(req, "/workspace/artifacts");
        return artifacts_response(limit, since, store).await;
    }
    if req.starts_with("GET /cron ") || req.starts_with("GET /cron?") {
        return cron_list_response().await;
    }
    if let Some(rest) = req.strip_prefix("GET /cron/") {
        if let Some((name, _)) = rest.split_once(' ') {
            return cron_show_response(name).await;
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
        let skill = parse_query_kv(tail, "skill");
        let has_candidate = parse_query_kv(tail, "has-candidate")
            .map(|v| v == "true" || v == "1");
        return forge_runs_response(
            limit,
            since,
            status.as_deref(),
            score_min,
            skill.as_deref(),
            has_candidate,
            store,
        )
        .await;
    }
    if let Some(rest) = req.strip_prefix("GET /forge/runs/") {
        if let Some((id, _)) = rest.split_once(' ') {
            return forge_run_show_response(id, store).await;
        }
    }
    if let Some(rest) = req.strip_prefix("GET /audit/sessions/") {
        // GET /audit/sessions/<id>/verify takes priority over the
        // bare GET /audit/sessions/<id> rows listing.
        if let Some((sid, tail)) = rest.split_once('/') {
            if tail.starts_with("verify ") {
                return audit_session_verify_response(sid, store).await;
            }
        }
        // Strip the (optional) ?query and trailing " HTTP/...".
        let (sid_with_q, _) = rest.split_once(' ').unwrap_or((rest, ""));
        let (sid, query_part) = sid_with_q
            .split_once('?')
            .map_or((sid_with_q, ""), |(s, q)| (s, q));
        let (mut kind, mut since): (Option<String>, Option<i64>) = (None, None);
        if !query_part.is_empty() {
            for kv in query_part.split('&') {
                if let Some(v) = kv.strip_prefix("kind=") {
                    kind = Some(v.to_string());
                } else if let Some(v) = kv.strip_prefix("since-minutes=") {
                    since = v.parse().ok();
                }
            }
        }
        return audit_session_response(sid, store, kind.as_deref(), since).await;
    }
    if let Some(rest) = req.strip_prefix("GET /sessions/") {
        if let Some((sid, tail)) = rest.split_once('/') {
            if tail.starts_with("spend/total ") {
                return session_spend_total_response(sid, store).await;
            }
            if tail.starts_with("spend ") {
                return session_spend_response(sid, store).await;
            }
        }
    }
    if req.starts_with("GET /costs ") {
        return costs_response();
    }
    if req.starts_with("GET /costs/by-model ") {
        return costs_breakdown_response("by_model", store).await;
    }
    if req.starts_with("GET /costs/by-session ") {
        return costs_breakdown_response("by_session", store).await;
    }
    if req.starts_with("POST /sessions ") {
        return create_session_response(req, store).await;
    }
    if req.starts_with("POST /skills ") {
        return install_skill_response(req, store).await;
    }
    if req.starts_with("POST /memory ") {
        return append_memory_response(req, store).await;
    }
    if req.starts_with("POST /cron ") {
        return cron_create_response(req).await;
    }
    if req.starts_with("POST /workspace/init ") {
        return workspace_init_response(req, store).await;
    }
    if req.starts_with("POST /db/vacuum ") {
        return db_vacuum_response(store).await;
    }
    if req.starts_with("POST /metrics/reset ") {
        return super::metrics::metrics_reset_response();
    }
    if req.starts_with("POST /db/prune ") {
        return db_prune_response(req, store).await;
    }
    if let Some(rest) = req.strip_prefix("POST /judge/") {
        if let Some((rid, tail)) = rest.split_once('/') {
            if tail.starts_with("override ") {
                return judge_override_response(rid, req, store).await;
            }
        }
    }
    if req.starts_with("POST /forge/optimize ") {
        return forge_optimize_response(req).await;
    }
    if req.starts_with("POST /forge/bench ") {
        return forge_bench_response(req).await;
    }
    if req.starts_with("POST /forge/runs/abort-all ") {
        return forge_abort_all_response(store).await;
    }
    if req.starts_with("POST /sessions/end-all ") {
        return session_end_all_response(store).await;
    }
    // POST /forge/runs/<id>/abort — flip status to "aborted".
    if let Some(rest) = req.strip_prefix("POST /forge/runs/") {
        if let Some((id, tail)) = rest.split_once('/') {
            if tail.starts_with("abort ") {
                return forge_abort_response(id, store).await;
            }
            if tail.starts_with("promote ") {
                return forge_promote_response(id, store).await;
            }
        }
    }
    // POST /skills/<name>/{promote,lock,unlock}
    if let Some(rest) = req.strip_prefix("POST /skills/") {
        if let Some((name, tail)) = rest.split_once('/') {
            if tail.starts_with("promote ") {
                return skill_promote_response(name, req, store).await;
            }
            if tail.starts_with("lock ") {
                return skill_lock_response(name, req, store).await;
            }
            if tail.starts_with("unlock ") {
                return skill_unlock_response(name, store).await;
            }
            if tail.starts_with("refine ") {
                return skill_refine_response(name, req, store).await;
            }
        }
    }
    // POST /cron/<name>/delete — remove the on-disk job file.
    if let Some(rest) = req.strip_prefix("POST /cron/") {
        if let Some((name, tail)) = rest.split_once('/') {
            if tail.starts_with("delete ") {
                return cron_delete_response(name).await;
            }
        }
    }
    // POST /memory/<id>/{lock,unlock,forget,edit}
    if let Some(rest) = req.strip_prefix("POST /memory/") {
        if let Some((id, tail)) = rest.split_once('/') {
            if tail.starts_with("lock ") {
                return memory_lock_response(id, req, store).await;
            }
            if tail.starts_with("unlock ") {
                return memory_unlock_response(id, store).await;
            }
            if tail.starts_with("forget ") {
                return memory_forget_response(id, store).await;
            }
            if tail.starts_with("edit ") {
                return memory_edit_response(id, req, store).await;
            }
        }
    }
    // POST /sessions/<uuid>/messages — append a message to a
    // recorded session. We do the URL parse here to keep
    // dispatch_route as the single entry-point.
    if let Some(rest) = req.strip_prefix("POST /sessions/") {
        if let Some((sid, tail)) = rest.split_once('/') {
            if tail.starts_with("messages ") {
                return append_message_response(sid, req, store).await;
            }
            if tail.starts_with("runs ") {
                return run_turn_response(sid, req, store).await;
            }
            if tail.starts_with("steer ") {
                return steer_response(sid, req, store).await;
            }
            if tail.starts_with("replay ") {
                return replay_response(sid, req, store).await;
            }
            if tail.starts_with("skills/attach ") {
                return session_attach_skill_response(sid, req, store).await;
            }
            if tail.starts_with("skills/detach ") {
                return session_detach_skill_response(sid, req, store).await;
            }
            if tail.starts_with("skills/clear ") {
                return session_clear_skills_response(sid, store).await;
            }
            if tail.starts_with("delete ") {
                return session_delete_response(sid, store).await;
            }
            if tail.starts_with("title ") {
                return session_set_title_response(sid, req, store).await;
            }
            if tail.starts_with("end ") {
                return session_end_response(sid, store).await;
            }
        }
    }
    // GET /sessions/<uuid> — fetch one session header
    // GET /sessions/<uuid>/messages — list every message on a session
    if let Some(rest) = req.strip_prefix("GET /sessions/") {
        if let Some((sid, tail)) = rest.split_once('/') {
            if tail.starts_with("messages/last ") {
                return last_message_response(sid, store).await;
            }
            if tail.starts_with("messages ") || tail.starts_with("messages?") {
                let limit = parse_limit(tail);
                let role = parse_role(tail);
                let since = super::routes_query::parse_query_kv(tail, "since-minutes")
                    .and_then(|v| v.parse::<i64>().ok());
                let seq_min = super::routes_query::parse_query_kv(tail, "seq-min")
                    .and_then(|v| v.parse::<i64>().ok());
                let seq_max = super::routes_query::parse_query_kv(tail, "seq-max")
                    .and_then(|v| v.parse::<i64>().ok());
                let content_contains = super::routes_query::parse_query_kv(tail, "content-contains")
                    .map(|s| s.replace('+', " "));
                return list_messages_response(
                    sid,
                    store,
                    limit,
                    role,
                    since,
                    seq_min,
                    seq_max,
                    content_contains,
                )
                .await;
            }
            if tail.starts_with("skills ") {
                return session_skills_list_response(sid, store).await;
            }
            if tail.starts_with("judge ") {
                return session_judge_response(sid, store).await;
            }
        }
        // Strip trailing " HTTP/..." → just the session id
        if let Some((sid, _)) = rest.split_once(' ') {
            return session_show_response(sid, store).await;
        }
    }
    ("404 Not Found", "{\"error\":\"not found\"}".to_string())
}

