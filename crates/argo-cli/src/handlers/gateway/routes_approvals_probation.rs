//! Probation-hold bridge: surface a probationary teammate's HELD tools as
//! rows on `GET /api/v1/approvals`, and resolve them through `/decide`.
//!
//! ## The broken promise this closes (shakedown F10)
//!
//! A fresh Genesis hire's first run is held by the probation window
//! (DEC-025 #2 — every action of a brand-new teammate needs a human
//! sign-off until probation clears). The worker page already surfaces
//! those held tools (`routes_worker_run::held_tools_for_session`) and the
//! operator clears them with the "Approve & run" control. But `/approvals`
//! read NOTHING about probation holds — its list arm only sees
//! `worker_action` memory rows + seeded `approval_request` rows — so the
//! inbox showed `0 pending` while a teammate sat held. The inbox header
//! promises "Every decision your AI teammates queue for a human shows up
//! here"; on the FIRST journey every new user takes, it was a lie.
//!
//! ## No parallel store — read-side union over the audit chain
//!
//! Like the seeded-approval bridge ([`super::routes_approvals_json_seeded`]),
//! this is purely a read-side union: the held tools are sourced from the
//! `probation.hold` audit rows the dispatch gate already writes
//! (`argo_agent_runner::turn_helpers_gate::emit_probation_hold`) — the
//! honest source, never the model's say-so — joined to `session.worker_id`
//! for the teammate persona. A hold is treated as PENDING until a
//! resolution marker (an `approval.granted` / `approval.declined` audit row
//! carrying `probation_hold:true` + the held `session_id` + `tool`) exists.
//!
//! ## Resolution is the worker-page control, reused
//!
//! Approving a `phold:` row resolves it EXACTLY like the worker-page
//! "Approve & run" button: it re-runs the teammate with the held tool
//! pre-approved via [`super::routes_worker_run::run_worker_now`] (which
//! mints the same ONE-SHOT session grant the worker page mints), so the
//! next run proceeds. The resolution marker drops the row out of the
//! inbox. Refuse writes the decline marker and stops there.

use super::routes_approvals_json_helpers::{bad_request, ok_decide, server_error};
use super::routes_approvals_json_seeded::SeededRow;
use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, WorkerId};
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

/// Synthetic approval-id prefix for a probation hold. The id is
/// `phold:<session_uuid>:<tool>` — the session UUID carries no colon and
/// tool names carry no colon, so a single split round-trips cleanly.
const PHOLD_PREFIX: &str = "phold:";

/// Fetch every UNRESOLVED probation hold visible to the caller's tenant,
/// shaped as the SPA `ApprovalRow` envelope (via [`SeededRow`] so the
/// caller merges, sorts, and cursor-paginates it uniformly with the
/// memory-row + seeded arms). Returns an empty list on any resolver miss
/// or read error — never an error from the caller's view — because the
/// memory-row arm is the primary list source and this bridge is additive.
pub(super) async fn fetch_probation_hold_rows(
    req: &str,
    store: &Arc<SessionStore>,
) -> Vec<SeededRow> {
    let Ok(tenant) = super::resolve_tenant::resolve_tenant_for_request(req, store).await else {
        return Vec::new();
    };
    let holds = match probation_hold_rows(store.pool(), &tenant.to_string()).await {
        Ok(rows) => rows,
        Err(_) => return Vec::new(),
    };
    if holds.is_empty() {
        return Vec::new();
    }
    let resolved = resolved_hold_keys(store.pool(), &tenant.to_string())
        .await
        .unwrap_or_default();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut out = Vec::new();
    for hold in holds {
        let key = (hold.session_id.clone(), hold.tool.clone());
        if resolved.contains(&key) || !seen.insert(key) {
            continue;
        }
        let worker = worker_name_for_session(store, tenant, &hold.session_id).await;
        out.push(build_hold_row(&hold, worker));
    }
    out
}

/// One held-tool dispatch read back from a `probation.hold` audit row.
struct HoldRow {
    session_id: String,
    tool: String,
    /// Unix-seconds `created_at` of the hold audit row, for `received_at`.
    created_at: i64,
}

/// Read `probation.hold` audit rows for the tenant, newest first.
async fn probation_hold_rows(
    pool: &sqlx::SqlitePool,
    tenant_id_text: &str,
) -> Result<Vec<HoldRow>, sqlx::Error> {
    let raw: Vec<(Option<Vec<u8>>, String, i64)> = sqlx::query_as(
        "SELECT session_id, payload_json, created_at FROM audit_record \
         WHERE kind = 'probation.hold' AND tenant_id = ? \
         ORDER BY created_at DESC",
    )
    .bind(tenant_id_text)
    .fetch_all(pool)
    .await?;
    let mut out = Vec::with_capacity(raw.len());
    for (sid_blob, payload_json, created_at) in raw {
        let Some(sid) = sid_blob.and_then(|b| Uuid::from_slice(&b).ok()) else {
            continue;
        };
        let Some(tool) = serde_json::from_str::<Value>(&payload_json)
            .ok()
            .and_then(|v| v.get("tool").and_then(Value::as_str).map(str::to_string))
        else {
            continue;
        };
        out.push(HoldRow {
            session_id: sid.to_string(),
            tool,
            created_at,
        });
    }
    Ok(out)
}

/// Collect the `(session_id, tool)` keys of holds already resolved — any
/// `approval.granted` / `approval.declined` audit row stamped with
/// `probation_hold:true` plus the original held `session_id` + `tool`.
async fn resolved_hold_keys(
    pool: &sqlx::SqlitePool,
    tenant_id_text: &str,
) -> Result<HashSet<(String, String)>, sqlx::Error> {
    let raw: Vec<(String,)> = sqlx::query_as(
        "SELECT payload_json FROM audit_record \
         WHERE kind IN ('approval.granted', 'approval.declined') AND tenant_id = ?",
    )
    .bind(tenant_id_text)
    .fetch_all(pool)
    .await?;
    let mut out = HashSet::new();
    for (payload_json,) in raw {
        let Ok(v) = serde_json::from_str::<Value>(&payload_json) else {
            continue;
        };
        if v.get("probation_hold").and_then(Value::as_bool) != Some(true) {
            continue;
        }
        let (Some(sid), Some(tool)) = (
            v.get("session_id").and_then(Value::as_str),
            v.get("tool").and_then(Value::as_str),
        ) else {
            continue;
        };
        out.insert((sid.to_string(), tool.to_string()));
    }
    Ok(out)
}

/// Resolve the held session's teammate persona name (via
/// `session.worker_id`). `None` keeps the row visible with a placeholder.
async fn worker_name_for_session(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    session_id: &str,
) -> Option<(WorkerId, String)> {
    let sid = Uuid::parse_str(session_id).ok()?;
    let worker_id = worker_id_for_session(store.pool(), tenant, &sid).await?;
    let worker = IdentityStore::new(store.as_ref())
        .get_worker(tenant, worker_id)
        .await
        .ok()??;
    Some((worker_id, worker.persona.name))
}

/// Read `session.worker_id` for a session, tenant-scoped (fail-closed —
/// a session in another tenant never resolves a worker here).
async fn worker_id_for_session(
    pool: &sqlx::SqlitePool,
    tenant: TenantId,
    session_id: &Uuid,
) -> Option<WorkerId> {
    let blob: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT worker_id FROM session WHERE id = ? AND tenant_id = ?",
    )
    .bind(session_id.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()?;
    Uuid::from_slice(&blob?).ok().map(WorkerId)
}

/// Shape a held tool as the SPA `ApprovalRow`. P2-low severity band per
/// the brief: a probation hold is routine teammate onboarding, not a
/// high-stakes spend/send decision.
fn build_hold_row(hold: &HoldRow, worker: Option<(WorkerId, String)>) -> SeededRow {
    let received_at = chrono::DateTime::<chrono::Utc>::from_timestamp(hold.created_at, 0)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default();
    let (worker_id_str, worker_name) = match worker {
        Some((id, name)) => (id.to_string(), name),
        None => ("unknown".to_string(), "New teammate".to_string()),
    };
    let id = format!("{PHOLD_PREFIX}{}:{}", hold.session_id, hold.tool);
    let consequence_if_yes =
        format!("{worker_name} may use {} and the run proceeds.", hold.tool);
    let json = json!({
        "id": id,
        "worker": { "id": worker_id_str, "name": worker_name, "avatar_url": Value::Null },
        "verb": format!("wants to use {}", hold.tool),
        "detail": "Held while on probation — every action of a brand-new teammate needs your sign-off until the probation window clears.",
        "consequence_if_yes": consequence_if_yes,
        "consequence_if_no": "The action stays held; the teammate waits for probation to clear or another approval.",
        "reason": "probation_window_active",
        "draft_content": Value::Null,
        "channel": "web",
        "received_at": received_at.clone(),
        "decision": Value::Null,
        "risk_level": "low",
        "severity": "p2",
    });
    SeededRow {
        received_at,
        id,
        json,
    }
}

/// Resolve a `phold:` approval id through `/decide`. Approve re-runs the
/// teammate with the held tool pre-approved (the worker-page control,
/// reused); refuse records the decline. Both write the resolution marker
/// that drops the row out of the inbox.
pub(super) async fn decide_probation_hold(
    approval_id: &str,
    decision: &str,
    req: &str,
) -> (&'static str, String) {
    let Some((session_id, tool)) = parse_phold_id(approval_id) else {
        return bad_request("invalid probation-hold approval id");
    };
    let (_, _, store) =
        match crate::handlers::worker_actions_transitions::open_workspace_memory_with_store().await
        {
            Ok(p) => p,
            Err(e) => return server_error(&format!("open workspace: {e}")),
        };
    let tenant = match super::resolve_tenant::resolve_tenant_for_request(req, &store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let actor = principal_id_for_audit(req).unwrap_or_else(|| "gateway".to_string());
    // Resolution marker (drops the row out of the inbox on next list).
    if let Err(e) = write_resolution(&store, tenant, &actor, &session_id, &tool, decision).await {
        return server_error(&format!("resolution audit: {e}"));
    }
    if decision == "refuse" {
        return ok_decide(approval_id, decision);
    }
    // Approve → re-run the teammate with the held tool pre-approved.
    let execution = rerun_with_tool(&store, tenant, &session_id, &tool, &actor).await;
    (
        "200 OK",
        json!({
            "ok": true,
            "approval_id": approval_id,
            "decision": "approve",
            "execution": execution,
        })
        .to_string(),
    )
}

/// Write the `approval.granted` / `approval.declined` resolution marker
/// stamped with the original held `session_id` + `tool` so the list arm
/// treats this hold as resolved.
async fn write_resolution(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    actor: &str,
    session_id: &str,
    tool: &str,
    decision: &str,
) -> argo_types::ArgoResult<()> {
    let kind = if decision == "approve" {
        AuditKind::ApprovalGranted
    } else {
        AuditKind::ApprovalDeclined
    };
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    sink.write(
        None,
        kind,
        json!({
            "probation_hold": true,
            "session_id": session_id,
            "tool": tool,
            "decision": decision,
            "decided_by": actor,
            "decided_at": chrono::Utc::now().to_rfc3339(),
        }),
        Some(actor.to_string()),
    )
    .await?;
    // Flush synchronously so the inbox row clears on the operator's NEXT
    // list (the decision should reflect immediately, not on the next
    // batch tick).
    sink.flush_pending().await
}

/// Re-run the held teammate with `tool` pre-approved — the worker-page
/// "Approve & run" control, reused. Returns a `/decide` `execution`
/// value: `"dispatched"` on a completed run, else an honest non-success
/// string (NO THEATER — never claim a run we didn't make).
async fn rerun_with_tool(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    session_id: &str,
    tool: &str,
    actor: &str,
) -> &'static str {
    let Some((worker_id, _)) = worker_name_for_session(store, tenant, session_id).await else {
        return "granted_no_rerun";
    };
    let Ok(Some(worker)) = IdentityStore::new(store.as_ref())
        .get_worker(tenant, worker_id)
        .await
    else {
        return "granted_no_rerun";
    };
    let prompt = "Do your next standing task now and report what you did.";
    let approve = [tool.to_string()];
    match super::routes_worker_run::run_worker_now(store, &worker, prompt, &approve, Some(actor))
        .await
    {
        Ok(Ok(outcome)) => {
            // Convergence (Option A). The re-run minted a FRESH session
            // (`outcome.session_id`) that is still inside the teammate's 24h
            // probation window, so the dispatch gate may have re-emitted a
            // `probation.hold` for a tool on that new session. Our first
            // resolution marker (`write_resolution` above, in
            // `decide_probation_hold`) is keyed to the OLD held session, so
            // without this the re-born hold lingers as pending forever and
            // the `/approvals` PENDING counter climbs on every reload instead
            // of converging to 0 (the 2026-06-14 15x-soak defect). Propagate
            // the approval to the re-run's session so an Approve of `tool`
            // nets exactly −1 for it, regardless of the re-run re-holding.
            propagate_approval_to_rerun_session(store, tenant, actor, &outcome.session_id, tool)
                .await;
            if outcome.status == "completed" {
                "dispatched"
            } else {
                "failed"
            }
        }
        Ok(Err(_blocked)) => "granted_run_blocked",
        Err(_) => "failed",
    }
}

/// Propagate an approval to the re-run's fresh session so a re-born
/// probation hold for the same `tool` resolves and the inbox counter
/// converges (Option A — see [`rerun_with_tool`]). Best-effort: a
/// marker-write miss only delays that row one extra reload, never produces a
/// wrong decision, and the re-run already happened (the approval IS granted),
/// so a failure is logged and swallowed rather than surfaced as a `/decide`
/// 500. A no-op when the re-run recorded no session id.
async fn propagate_approval_to_rerun_session(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    actor: &str,
    rerun_session_id: &str,
    tool: &str,
) {
    if rerun_session_id.is_empty() {
        return;
    }
    if let Err(e) = write_resolution(store, tenant, actor, rerun_session_id, tool, "approve").await
    {
        tracing::warn!(
            target: "gateway::approvals_probation",
            tool = %tool,
            session = %rerun_session_id,
            error = %e,
            "could not propagate probation-hold approval to the re-run session; \
             inbox row may linger one reload"
        );
    }
}

/// Parse `phold:<session_uuid>:<tool>` into `(session_id, tool)`.
fn parse_phold_id(id: &str) -> Option<(String, String)> {
    let rest = id.strip_prefix(PHOLD_PREFIX)?;
    let (session_id, tool) = rest.split_once(':')?;
    if session_id.is_empty() || tool.is_empty() {
        return None;
    }
    Some((session_id.to_string(), tool.to_string()))
}

#[cfg(test)]
#[path = "routes_approvals_probation_tests.rs"]
mod tests;
