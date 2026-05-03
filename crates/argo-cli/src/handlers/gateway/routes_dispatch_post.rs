//! POST-route dispatch lifted out of `routes.rs` to keep that
//! file under the 500-LOC budget. Pure router — every match
//! arm forwards to a handler defined in another sibling
//! module. Returns `Some(resp)` on a match, `None` otherwise so
//! `routes.rs` can fall through to its 404 default.
use super::routes_cron::{cron_create_response, cron_delete_response};
use super::routes_forge_promote::{
    forge_abort_all_response, forge_promote_response, forge_requeue_response,
    forge_sweep_response,
};
use super::routes_post::{
    append_message_response, create_session_for_vertical_response, create_session_response,
    replay_response, run_turn_response,
    steer_response,
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
use super::routes_session_approval::{
    session_approve_response, session_revoke_approval_response,
};
use super::routes_session_clone::{session_branch_response, session_clone_response};
use super::routes_session_skills::{
    session_attach_skill_response, session_clear_skills_response, session_delete_response,
    session_detach_skill_response, session_end_all_response, session_end_response,
    session_reopen_response, session_set_title_response,
};
use super::routes_skill_refine::skill_refine_response;
use argo_store::SessionStore;
use std::sync::Arc;

/// Dispatch every POST verb. Returns `None` when no match,
/// letting `dispatch_route` fall through to its other branches
/// (and ultimately the 404 default).
pub(super) async fn dispatch_post(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("POST /sessions ") {
        return Some(create_session_response(req, store).await);
    }
    // POST /verticals/<id>/sessions — path-bound session create
    // (#651). The id is the [vertical].id slug; the body is
    // {model, provider, title?} — same shape as POST /sessions.
    if let Some(rest) = req.strip_prefix("POST /verticals/") {
        if let Some((id, tail)) = rest.split_once('/') {
            if tail.starts_with("sessions ") || tail.starts_with("sessions?") {
                return Some(
                    create_session_for_vertical_response(id, req, store).await,
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
        return Some(forge_optimize_response(req, store).await);
    }
    if req.starts_with("POST /forge/bench ") {
        return Some(forge_bench_response(req).await);
    }
    if req.starts_with("POST /forge/runs/abort-all ") {
        return Some(forge_abort_all_response(store).await);
    }
    if req.starts_with("POST /forge/runs/sweep ")
        || req.starts_with("POST /forge/runs/sweep?")
    {
        let tail = req.strip_prefix("POST ").unwrap_or(req);
        let older_than_minutes = parse_query_kv(tail, "older-than-minutes")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(60);
        return Some(forge_sweep_response(older_than_minutes, store).await);
    }
    if req.starts_with("POST /sessions/end-all ")
        || req.starts_with("POST /sessions/end-all?")
    {
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
                let through_seq = parse_query_kv(tail, "through-seq")
                    .and_then(|v| v.parse::<i64>().ok());
                return Some(
                    session_branch_response(sid, through_seq, store).await,
                );
            }
            // #665 — escalation grant flow mirror of `argo
            // session approve` / `revoke-approval`. Same body
            // shape (`{tool, note?}`), same audit kinds, same
            // SQLite write path.
            if tail.starts_with("approve ") {
                return Some(session_approve_response(sid, req, store).await);
            }
            if tail.starts_with("revoke-approval ") {
                return Some(
                    session_revoke_approval_response(sid, req, store).await,
                );
            }
        }
    }
    None
}
