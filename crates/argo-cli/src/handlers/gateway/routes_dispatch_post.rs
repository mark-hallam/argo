//! POST-route dispatch lifted out of `routes.rs` to keep that
//! file under the 500-LOC budget. Pure router — every match
//! arm forwards to a handler defined in another sibling
//! module. Returns `Some(resp)` on a match, `None` otherwise so
//! `routes.rs` can fall through to its 404 default.
use super::routes_auth::check_bearer_auth;
use super::routes_auth_principal::{parse_principal_header, principal_id_for_audit};
use super::routes_cron::{cron_create_response, cron_delete_response};
use super::routes_forge_promote::{
    forge_abort_all_response, forge_promote_response, forge_requeue_response, forge_sweep_response,
};
use super::routes_post::{
    append_message_response, create_session_for_vertical_response, create_session_response,
    replay_response, run_turn_response, steer_response,
};
use super::routes_post_admin::{
    judge_override_response, memory_edit_response, memory_forget_response, memory_lock_response,
    memory_unlock_response, skill_lock_response, skill_unlock_response,
};
use super::routes_post_writes::{
    append_memory_response, db_prune_response, db_vacuum_response, forge_abort_response,
    forge_bench_response, forge_optimize_response, install_skill_response, skill_promote_response,
    workspace_init_response,
};
use super::routes_query::parse_query_kv;
use super::routes_session_approval::{session_approve_response, session_revoke_approval_response};
use super::routes_session_clone::{session_branch_response, session_clone_response};
use super::routes_session_skills::{
    session_attach_skill_response, session_clear_skills_response, session_delete_response,
    session_detach_skill_response, session_end_all_response, session_end_response,
    session_reopen_response, session_set_title_response,
};
use super::routes_skill_refine::skill_refine_response;
use argo_store::tenant_quota::TenantQuotaStore;
use argo_store::tenant_quota_kinds::{QuotaActionKind, QuotaError};
use argo_store::SessionStore;
use std::sync::Arc;
use tracing::Instrument;

/// HTTP header carrying a caller-supplied request correlation id
/// for log/audit cross-referencing. Optional — emissions get an
/// empty string when the header is absent.
const REQUEST_ID_HEADER: &str = "X-Request-Id";

/// Extract the request line's `<verb> <path>` (no HTTP version).
/// `req` is the raw HTTP wire form; the first line is always the
/// request line per RFC 9112. Returns `""` on a malformed prefix.
fn request_route(req: &str) -> &str {
    let line = req.lines().next().unwrap_or("");
    line.rsplit_once(' ').map_or(line, |(head, _)| head)
}

/// Read `X-Request-Id` from the header block. Returns the trimmed
/// value or an empty string when the header is missing.
fn request_id_header(req: &str) -> &str {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers
        .lines()
        .skip(1)
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case(REQUEST_ID_HEADER)
                .then(|| value.trim())
        })
        .unwrap_or("")
}

/// Resolve the cwd-derived tenant id and ask the quota store
/// whether one more `kind` action would exceed the configured
/// cap. Returns `Some((status, body))` to short-circuit dispatch
/// with HTTP 429; returns `None` on pass or on internal error
/// (fail-open — the gate is not authoritative when the store is
/// unreachable; observability still records the underlying
/// failure via the existing summary read path).
async fn check_quota_or_reject(
    store: &Arc<SessionStore>,
    kind: QuotaActionKind,
) -> Option<(&'static str, String)> {
    let cwd = std::env::current_dir().ok()?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await.ok()?;
    let tenant_id = tenant.0.to_string();
    let quotas = TenantQuotaStore::new(store.pool());
    match quotas
        .check_or_reject(&tenant_id, kind, 1, chrono::Utc::now())
        .await
    {
        Ok(()) => None,
        Err(QuotaError::Exceeded {
            kind,
            current,
            limit,
        }) => Some((
            "429 Too Many Requests",
            format!(
                "{{\"error\":\"quota_exceeded\",\"kind\":\"{kind}\",\"current\":{current},\"limit\":{limit}}}"
            ),
        )),
        Err(QuotaError::Internal(_)) => None,
    }
}

/// Dispatch every POST verb. Returns `None` when no match,
/// letting `dispatch_route` fall through to its other branches
/// (and ultimately the 404 default).
///
/// Wrapped in `tracing::info_span!("request", …)` so every
/// `info!`/`warn!`/`error!` emission below this point inherits
/// `route`, `principal_id`, and `request_id` fields without a
/// per-callsite change (mvp-structured-logging).
pub(super) async fn dispatch_post(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let span = tracing::info_span!(
        "request",
        route = %request_route(req),
        principal_id = %principal_id_for_audit(req).unwrap_or_default(),
        request_id = %request_id_header(req),
    );
    dispatch_post_inner(req, store).instrument(span).await
}

async fn dispatch_post_inner(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if let Some(resp) = check_bearer_auth(req) {
        return Some(resp);
    }
    if parse_principal_header(req).is_err() {
        return Some((
            "400 Bad Request",
            r#"{"error":"bad principal"}"#.to_string(),
        ));
    }
    // POST /api/v1/genesis/{classify,draft-worker} — Stream J4.
    // Thin HTTP wrappers around `argo-genesis`; bearer-gated above.
    if req.starts_with("POST /api/v1/genesis/classify ") {
        return Some(super::routes_genesis::classify_response(req, store).await);
    }
    if req.starts_with("POST /api/v1/genesis/draft-worker ") {
        return Some(super::routes_genesis::draft_worker_response(req, store).await);
    }
    // J5 — hire-commit endpoint. Bearer auth applied above; body
    // shape is `{worker_draft, problem, answers}` per the brief.
    if req.starts_with("POST /api/v1/workers ") {
        return Some(super::routes_workers_hire::hire_response(req, store).await);
    }
    // J1 — JSON approvals API: per-id `decide` and `undo`. The list
    // GET arm is wired in `routes_dispatch_get`. The HTML `/inbox`
    // surface in `routes_channels_webinbox` is preserved separately.
    if let Some(rest) = req.strip_prefix("POST /api/v1/approvals/") {
        if let Some((id, tail)) = rest.split_once('/') {
            if tail.starts_with("decide ") || tail.starts_with("decide?") {
                return Some(super::routes_approvals_json::decide_response(id, req).await);
            }
            if tail.starts_with("undo ") || tail.starts_with("undo?") {
                return Some(super::routes_approvals_json::undo_response(id, req).await);
            }
        }
    }
    if req.starts_with("POST /sessions ") {
        if let Some(s) = store {
            if let Some(resp) = check_quota_or_reject(s, QuotaActionKind::Sessions).await {
                return Some(resp);
            }
        }
        let resp = create_session_response(req, store).await;
        // Peer-sync hint: when ARGO_GATEWAY_PEERS is set, fan out a
        // POST /peer/poke so peers fast-resync instead of waiting
        // for the next periodic poll. No-op when the mesh is not
        // configured.
        if resp.0.starts_with("200") || resp.0.starts_with("201") {
            super::peer::notify_peers_of_local_change();
        }
        return Some(resp);
    }
    // POST /verticals/<id>/{sessions,run} — path-bound vertical
    // operations.
    if let Some(rest) = req.strip_prefix("POST /verticals/") {
        if let Some((id, tail)) = rest.split_once('/') {
            // /sessions (#651) — bind a session to the vertical.
            if tail.starts_with("sessions ") || tail.starts_with("sessions?") {
                return Some(create_session_for_vertical_response(id, req, store).await);
            }
            // /run (#670) — one-shot composite mirror of `argo
            // vertical run`: create session + attach skills + run
            // the named workflow against the new session.
            if tail.starts_with("run ") || tail.starts_with("run?") {
                return Some(
                    super::routes_verticals_run::vertical_run_response(id, req, store).await,
                );
            }
        }
    }
    if req.starts_with("POST /skills ") {
        return Some(install_skill_response(req, store).await);
    }
    if req.starts_with("POST /memory ") {
        return Some(append_memory_response(req, store).await);
    }
    if req.starts_with("POST /cron ") {
        return Some(cron_create_response(req).await);
    }
    if req.starts_with("POST /workspace/init ") {
        return Some(workspace_init_response(req, store).await);
    }
    if req.starts_with("POST /db/vacuum ") {
        return Some(db_vacuum_response(store).await);
    }
    if req.starts_with("POST /metrics/reset ") {
        return Some(super::metrics::metrics_reset_response());
    }
    if req.starts_with("POST /db/prune ") {
        return Some(db_prune_response(req, store).await);
    }
    if let Some(rest) = req.strip_prefix("POST /judge/") {
        if let Some((rid, tail)) = rest.split_once('/') {
            if tail.starts_with("override ") {
                return Some(judge_override_response(rid, req, store).await);
            }
        }
    }
    if req.starts_with("POST /forge/optimize ") || req.starts_with("POST /forge/optimize?") {
        if let Some(s) = store {
            if let Some(resp) = check_quota_or_reject(s, QuotaActionKind::ForgeRuns).await {
                return Some(resp);
            }
        }
        return Some(forge_optimize_response(req, store).await);
    }
    if req.starts_with("POST /forge/bench ") {
        return Some(forge_bench_response(req).await);
    }
    if req.starts_with("POST /forge/runs/abort-all ") {
        return Some(forge_abort_all_response(store).await);
    }
    if req.starts_with("POST /forge/runs/sweep ") || req.starts_with("POST /forge/runs/sweep?") {
        let tail = req.strip_prefix("POST ").unwrap_or(req);
        let older_than_minutes = parse_query_kv(tail, "older-than-minutes")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(60);
        return Some(forge_sweep_response(older_than_minutes, store).await);
    }
    if req.starts_with("POST /sessions/end-all ") || req.starts_with("POST /sessions/end-all?") {
        let tail = req.strip_prefix("POST ").unwrap_or(req);
        let older_than =
            parse_query_kv(tail, "older-than-minutes").and_then(|v| v.parse::<i64>().ok());
        return Some(session_end_all_response(older_than, store).await);
    }
    // POST /forge/runs/<id>/abort — flip status to "aborted".
    if let Some(rest) = req.strip_prefix("POST /forge/runs/") {
        if let Some((id, tail)) = rest.split_once('/') {
            if tail.starts_with("abort ") {
                return Some(forge_abort_response(id, store).await);
            }
            if tail.starts_with("promote ") {
                return Some(forge_promote_response(id, store).await);
            }
            if tail.starts_with("requeue ") {
                return Some(forge_requeue_response(id, store).await);
            }
        }
    }
    // POST /skills/<name>/{promote,lock,unlock,refine}
    if let Some(rest) = req.strip_prefix("POST /skills/") {
        if let Some((name, tail)) = rest.split_once('/') {
            if tail.starts_with("promote ") {
                return Some(skill_promote_response(name, req, store).await);
            }
            if tail.starts_with("lock ") {
                return Some(skill_lock_response(name, req, store).await);
            }
            if tail.starts_with("unlock ") {
                return Some(skill_unlock_response(name, store).await);
            }
            if tail.starts_with("refine ") {
                if let Some(s) = store {
                    if let Some(resp) =
                        check_quota_or_reject(s, QuotaActionKind::SkillInvocations).await
                    {
                        return Some(resp);
                    }
                }
                return Some(skill_refine_response(name, req, store).await);
            }
        }
    }
    // POST /cron/<name>/delete — remove the on-disk job file.
    if let Some(rest) = req.strip_prefix("POST /cron/") {
        if let Some((name, tail)) = rest.split_once('/') {
            if tail.starts_with("delete ") {
                return Some(cron_delete_response(name).await);
            }
        }
    }
    // POST /memory/<id>/{lock,unlock,forget,edit}
    if let Some(rest) = req.strip_prefix("POST /memory/") {
        if let Some((id, tail)) = rest.split_once('/') {
            if tail.starts_with("lock ") {
                return Some(memory_lock_response(id, req, store).await);
            }
            if tail.starts_with("unlock ") {
                return Some(memory_unlock_response(id, store).await);
            }
            if tail.starts_with("forget ") {
                return Some(memory_forget_response(id, store).await);
            }
            if tail.starts_with("edit ") {
                return Some(memory_edit_response(id, req, store).await);
            }
        }
    }
    // POST /sessions/<uuid>/{messages,runs,steer,replay,skills/*,delete,title,end,reopen}
    if let Some(rest) = req.strip_prefix("POST /sessions/") {
        if let Some((sid, tail)) = rest.split_once('/') {
            if tail.starts_with("messages ") {
                if let Some(s) = store {
                    if let Some(resp) =
                        check_quota_or_reject(s, QuotaActionKind::Messages).await
                    {
                        return Some(resp);
                    }
                }
                return Some(append_message_response(sid, req, store).await);
            }
            if tail.starts_with("runs ") {
                return Some(run_turn_response(sid, req, store).await);
            }
            if tail.starts_with("steer ") {
                return Some(steer_response(sid, req, store).await);
            }
            if tail.starts_with("replay ") {
                return Some(replay_response(sid, req, store).await);
            }
            if tail.starts_with("skills/attach ") {
                return Some(session_attach_skill_response(sid, req, store).await);
            }
            if tail.starts_with("skills/detach ") {
                return Some(session_detach_skill_response(sid, req, store).await);
            }
            if tail.starts_with("skills/clear ") {
                return Some(session_clear_skills_response(sid, store).await);
            }
            if tail.starts_with("delete ") {
                return Some(session_delete_response(sid, store).await);
            }
            if tail.starts_with("title ") {
                return Some(session_set_title_response(sid, req, store).await);
            }
            if tail.starts_with("end ") {
                return Some(session_end_response(sid, store).await);
            }
            if tail.starts_with("reopen ") {
                return Some(session_reopen_response(sid, store).await);
            }
            if tail.starts_with("clone ") {
                return Some(session_clone_response(sid, store).await);
            }
            if tail.starts_with("branch ") || tail.starts_with("branch?") {
                let through_seq =
                    parse_query_kv(tail, "through-seq").and_then(|v| v.parse::<i64>().ok());
                return Some(session_branch_response(sid, through_seq, store).await);
            }
            // #665 — escalation grant flow mirror of `argo
            // session approve` / `revoke-approval`. Same body
            // shape (`{tool, note?}`), same audit kinds, same
            // SQLite write path.
            if tail.starts_with("approve ") {
                return Some(session_approve_response(sid, req, store).await);
            }
            if tail.starts_with("revoke-approval ") {
                return Some(session_revoke_approval_response(sid, req, store).await);
            }
        }
    }
    None
}

#[cfg(test)]
#[path = "routes_dispatch_post_tests.rs"]
mod tests;
