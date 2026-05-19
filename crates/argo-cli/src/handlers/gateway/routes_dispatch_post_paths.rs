//! Path-bound POST sub-dispatchers grouped from
//! [`super::routes_dispatch_post::dispatch_post_inner`]. Each
//! function owns a disjoint URL prefix and returns `Some(resp)`
//! on a hit so the parent dispatch can early-return.

use super::routes_cron::cron_delete_response;
use super::routes_forge_promote::{
    forge_abort_all_response, forge_promote_response, forge_requeue_response, forge_sweep_response,
};
use super::routes_post::{
    append_message_response, replay_response, run_turn_response, steer_response,
};
use super::routes_post_admin::{
    judge_override_response, skill_lock_response, skill_unlock_response,
};
use super::routes_post_writes::{
    db_prune_response, db_vacuum_response, forge_abort_response, forge_bench_response,
    forge_optimize_response, skill_promote_response, workspace_init_response,
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
use argo_store::tenant_quota_kinds::QuotaActionKind;
use argo_store::SessionStore;
use std::sync::Arc;

/// `/api/v1/workers/<id>/{pause,resume,archive}` lifecycle routes.
pub(super) async fn dispatch_worker_lifecycle(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let rest = req.strip_prefix("POST /api/v1/workers/")?;
    let (head, _) = rest.split_once(' ')?;
    let head_no_query = head.split('?').next().unwrap_or(head);
    let parts: Vec<&str> = head_no_query.split('/').collect();
    if parts.len() != 2 {
        return None;
    }
    let worker_id = parts[0];
    match parts[1] {
        "pause" => Some(super::routes_worker_settings::post_pause(worker_id, req, store).await),
        "resume" => Some(super::routes_worker_settings::post_resume(worker_id, req, store).await),
        "archive" => Some(super::routes_worker_settings::post_archive(worker_id, req, store).await),
        _ => None,
    }
}

/// `/api/v1/approvals/bundles[/decide]` create + decide routes.
pub(super) async fn dispatch_approval_bundles(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("POST /api/v1/approvals/bundles ")
        || req.starts_with("POST /api/v1/approvals/bundles?")
    {
        return Some(super::routes_approval_bundles::create_response(req, store).await);
    }
    let rest = req.strip_prefix("POST /api/v1/approvals/bundles/")?;
    let (id, tail) = rest.split_once('/')?;
    if tail.starts_with("decide ") || tail.starts_with("decide?") {
        return Some(super::routes_approval_bundles::decide_response(id, req, store).await);
    }
    None
}

/// `/api/v1/orchestration/runs/<id>/<action>` orchestration actions.
pub(super) async fn dispatch_orchestration(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let rest = req.strip_prefix("POST /api/v1/orchestration/runs/")?;
    let (head, _) = rest.split_once(' ')?;
    let parts: Vec<&str> = head.split('/').collect();
    if parts.len() != 2 {
        return None;
    }
    Some(super::routes_orchestration::action_response(parts[0], parts[1], store).await)
}

/// Top-level singletons: `/verticals/<id>/{sessions,run}`,
/// `/skills`, `/cron`, `/workspace/init`, `/db/vacuum`,
/// `/metrics/reset`, `/db/prune`, `/judge/<rid>/override`.
pub(super) async fn dispatch_simple_admin(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if let Some(rest) = req.strip_prefix("POST /verticals/") {
        if let Some((id, tail)) = rest.split_once('/') {
            if tail.starts_with("sessions ") || tail.starts_with("sessions?") {
                return Some(
                    super::routes_post::create_session_for_vertical_response(id, req, store).await,
                );
            }
            if tail.starts_with("run ") || tail.starts_with("run?") {
                return Some(
                    super::routes_verticals_run::vertical_run_response(id, req, store).await,
                );
            }
        }
    }
    if req.starts_with("POST /skills ") {
        return Some(super::routes_post_writes::install_skill_response(req, store).await);
    }
    if req.starts_with("POST /cron ") {
        return Some(super::routes_cron::cron_create_response(req).await);
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
    None
}

/// Forge endpoints: `/forge/optimize`, `/forge/bench`,
/// `/forge/runs/abort-all`, `/forge/runs/sweep`,
/// `/forge/runs/<id>/{abort,promote,requeue}`. The optimize path
/// is quota-gated; the caller passes `optimize_quota_ok` to express
/// whether the quota check should be invoked.
pub(super) async fn dispatch_forge(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("POST /forge/optimize ") || req.starts_with("POST /forge/optimize?") {
        if let Some(s) = store {
            if let Some(resp) =
                super::routes_dispatch_post::check_quota_or_reject(s, QuotaActionKind::ForgeRuns).await
            {
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
    None
}

/// `/skills/<name>/{promote,lock,unlock,refine}` and
/// `/cron/<name>/delete`. The `refine` path is quota-gated against
/// `SkillInvocations`.
pub(super) async fn dispatch_skill_named(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
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
                    if let Some(resp) = super::routes_dispatch_post::check_quota_or_reject(
                        s,
                        QuotaActionKind::SkillInvocations,
                    )
                    .await
                    {
                        return Some(resp);
                    }
                }
                return Some(skill_refine_response(name, req, store).await);
            }
        }
    }
    if let Some(rest) = req.strip_prefix("POST /cron/") {
        if let Some((name, tail)) = rest.split_once('/') {
            if tail.starts_with("delete ") {
                return Some(cron_delete_response(name).await);
            }
        }
    }
    None
}

/// `/sessions/<sid>/<verb>` — every operation on a single session
/// (append message, run turn, steer, replay, skill attach/detach,
/// delete, title, end/reopen, clone/branch, approve/revoke).
pub(super) async fn dispatch_session_path(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let rest = req.strip_prefix("POST /sessions/")?;
    let (sid, tail) = rest.split_once('/')?;
    if tail.starts_with("messages ") {
        if let Some(s) = store {
            if let Some(resp) =
                super::routes_dispatch_post::check_quota_or_reject(s, QuotaActionKind::Messages)
                    .await
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
        let through_seq = parse_query_kv(tail, "through-seq").and_then(|v| v.parse::<i64>().ok());
        return Some(session_branch_response(sid, through_seq, store).await);
    }
    if tail.starts_with("approve ") {
        return Some(session_approve_response(sid, req, store).await);
    }
    if tail.starts_with("revoke-approval ") {
        return Some(session_revoke_approval_response(sid, req, store).await);
    }
    None
}

/// `/sessions/end-all` collection-level POST. Separate from
/// [`dispatch_session_path`] because the prefix lacks an
/// `<sid>/` segment.
pub(super) async fn dispatch_session_end_all(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if !(req.starts_with("POST /sessions/end-all ") || req.starts_with("POST /sessions/end-all?")) {
        return None;
    }
    let tail = req.strip_prefix("POST ").unwrap_or(req);
    let older_than = parse_query_kv(tail, "older-than-minutes").and_then(|v| v.parse::<i64>().ok());
    Some(session_end_all_response(older_than, store).await)
}

/// `POST /api/v1/support/tickets` + `POST /api/v1/support/tickets/<id>/reply`
/// — claude-support-inbox-spa-page. Bearer auth + principal-header
/// validation already ran in the parent `preflight_dispatch`.
pub(super) async fn dispatch_support_tickets(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("POST /api/v1/support/tickets ")
        || req.starts_with("POST /api/v1/support/tickets?")
    {
        return Some(super::routes_support_tickets::create_response(req, store).await);
    }
    let rest = req.strip_prefix("POST /api/v1/support/tickets/")?;
    let (head, _) = rest.split_once(' ')?;
    let head_no_query = head.split('?').next().unwrap_or(head);
    let (id, tail) = head_no_query.split_once('/')?;
    if tail != "reply" {
        return None;
    }
    Some(super::routes_support_tickets::reply_response(id, req, store).await)
}
