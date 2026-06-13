//! `/api/v1/approvals` — JSON wrapper over the worker-action
//! approval state machine. Replaces the HTML `/inbox` page with a
//! cursor-paginated JSON list + per-id decide / undo endpoints
//! that the SPA frontend (E1-a) consumes.
//!
//! The HTML `/inbox` route (`routes_channels_webinbox`) is preserved
//! verbatim for legacy operators — this module is additive.
//!
//! State-machine mapping (sensible deviations from the brief, which
//! assumed a `pending → approved/refused → undone` state-machine
//! that does not exist):
//!
//! - `decide: "approve"` → `worker_actions_transitions::approve()`
//!   (state `pending → approved`).
//! - `decide: "refuse"`  → `worker_actions_transitions::cancel()`
//!   with a `"decision":"refuse"` note (state `pending → cancelled`).
//!   The `cancelled` row is what the API reports back as `"refuse"`.
//! - `POST /undo` within a 30-second window of the decision row's
//!   `created_at` appends a new `pending` memory row directly
//!   (bypassing the state-machine validator, as the existing
//!   `reject_action` helper in `routes_channels.rs` already does for
//!   non-validator append paths). This restores the action to the
//!   pending queue. Outside the window, returns `410 Gone`.
//!
//! Tenant scoping is workspace-derived (single-tenant per
//! workspace). Per-tenant filtering across a multi-tenant gateway
//! is closed by `mission-enterprise-approvals-rbac`.

use super::role_gate::{enforce_role, resolve_role, Action as RbacAction};
use super::routes_approvals_json_helpers::{
    bad_request, conflict, decode_cursor, encode_cursor, gone, not_found, ok_decide, parse_limit,
    parse_query, server_error, short_payload,
};
use super::routes_auth_principal::principal_id_for_audit;
use crate::handlers::worker_actions::{ActionRecord, ActionState};
use crate::handlers::worker_actions_query::{
    collect_latest_actions, collect_latest_actions_for_tenant, workspace_for_tenant,
};
use crate::handlers::worker_actions_transitions::{
    self, find_action, open_workspace_memory_with_store,
};
use argo_audit::{AuditKind, AuditSink};
use argo_memory::MemoryFile;
use argo_redact::{
    active_grants_for_principal, audit_payload, redact_approval_row, resolve_policy,
    ViewerPermissions,
};
use argo_store::SessionStore;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

/// Window during which a decision is reversible via `POST /undo`.
const UNDO_WINDOW_SECS: i64 = 30;

/// `GET /api/v1/approvals?cursor=<c>&limit=<n>` — paginated list of
/// approvals visible to the caller's tenant. Returns
/// `{ items: ApprovalRow[], next_cursor: <opt> }`.
///
/// Two data sources are merged behind this surface:
///
/// 1. **`worker_action` memory rows** (the legacy state-machine path,
///    used by real agent runs) — filtered to `state = pending` AND
///    `requires_approval = true`.
/// 2. **`approval_request` table rows** (first-class approval store,
///    used by `argo demo seed` and any future writer that doesn't go
///    through the worker-action transitions) — surfaced for ALL
///    statuses so the SPA's 7-day approval-rate ring can compute from
///    the decided rows alongside the pending inbox.
///
/// The two stores remain disjoint at the data level; this endpoint is
/// the read-side union. Decide / undo continues to flow through the
/// worker-action transitions only — out of scope for the fix-brief
/// `fix-approvals-list-tenant-scope-empty-despite-seeded-rows` that
/// surfaced this gap.
pub(super) async fn list_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let tail = req.strip_prefix("GET ").unwrap_or(req);
    let limit = match parse_limit(tail) {
        Ok(n) => n,
        Err(resp) => return resp,
    };
    let cursor = parse_query(tail, "cursor").and_then(|c| decode_cursor(&c));
    // cu1g (DEC-050): read the CALLER's tenant workspace (genesis-aware)
    // when a store is available — the cwd+flat-only collect can never see
    // a Genesis teammate's held actions (wrong workspace AND its
    // `.genesis/<tenant>/<vid>` id is never enumerated). No store (legacy
    // test harness path) keeps the old behavior.
    let actions = match store {
        Some(s) => {
            let tenant = match super::resolve_tenant::resolve_tenant_for_request(req, s).await {
                Ok(t) => t,
                Err(e) => return server_error(&format!("resolve tenant: {e}")),
            };
            match collect_latest_actions_for_tenant(s, tenant, None).await {
                Ok(v) => v,
                Err(e) => return server_error(&format!("collect actions: {e}")),
            }
        }
        None => match collect_latest_actions(None).await {
            Ok(v) => v,
            Err(e) => return server_error(&format!("collect actions: {e}")),
        },
    };
    let pending: Vec<ActionRecord> = actions
        .into_iter()
        .filter(|a| a.state == ActionState::Pending.as_str() && a.requires_approval)
        .collect();
    let actor_id = principal_id_for_audit(req).unwrap_or_else(|| "anonymous".to_string());
    // v1 viewer-permissions resolution: the gateway has no
    // principal→permission mapping yet (RBAC permissions row lands
    // with mission-enterprise-approvals-rbac). Until then the fence
    // applies the strict default to every viewer — defence in depth
    // matches what E1-e's client layer already shows users.
    let viewer = ViewerPermissions::strict();
    let grants = active_grants_for_principal(&actor_id);
    let policy = resolve_policy(&viewer, &grants);

    // Memory-row arm: keep existing audit-on-redaction telemetry path.
    let mut entries: Vec<(String, String, Value)> = pending
        .iter()
        .map(|rec| {
            let row = approval_row_json(rec);
            let redaction =
                redact_approval_row(row, &policy, grants.first_id().map(str::to_string));
            if redaction.was_masked() {
                let payload = audit_payload(&actor_id, &rec.action_id, &redaction);
                tracing::info!(
                    target: "audit::approval_row_redacted",
                    event_type = argo_redact::AUDIT_EVENT_TYPE,
                    actor_id = %actor_id,
                    approval_id = %rec.action_id,
                    payload = %payload,
                    "approval row redacted by server fence"
                );
            }
            (rec.created_at.clone(), rec.action_id.clone(), redaction.row)
        })
        .collect();

    // First-class `approval_request` arm — surface seeded rows. Reads
    // through the gateway's already-open shared store (no second open /
    // re-migrate per request — that was the empty-list race). Empty when
    // no store is threaded or on a tenant-resolver miss; never errors
    // back to the caller, because the memory-row arm above is the
    // primary list source and this bridge is purely additive.
    let seeded = match store {
        Some(store) => {
            super::routes_approvals_json_seeded::fetch_seeded_approval_rows(req, store).await
        }
        None => Vec::new(),
    };
    for row in seeded {
        let redaction = redact_approval_row(
            row.json,
            &policy,
            grants.first_id().map(str::to_string),
        );
        if redaction.was_masked() {
            let payload = audit_payload(&actor_id, &row.id, &redaction);
            tracing::info!(
                target: "audit::approval_row_redacted",
                event_type = argo_redact::AUDIT_EVENT_TYPE,
                actor_id = %actor_id,
                approval_id = %row.id,
                payload = %payload,
                "approval row redacted by server fence"
            );
        }
        entries.push((row.received_at, row.id, redaction.row));
    }

    // Probation-hold arm (shakedown F10) — surface a probationary
    // teammate's HELD tools so the inbox is no longer empty/0-pending on
    // the first journey every new user takes. Read-side union over the
    // `probation.hold` audit rows (no parallel store); empty without a
    // store. Same redaction fence + audit telemetry as the arms above.
    let holds = match store {
        Some(store) => {
            super::routes_approvals_probation::fetch_probation_hold_rows(req, store).await
        }
        None => Vec::new(),
    };
    for row in holds {
        let redaction =
            redact_approval_row(row.json, &policy, grants.first_id().map(str::to_string));
        if redaction.was_masked() {
            let payload = audit_payload(&actor_id, &row.id, &redaction);
            tracing::info!(
                target: "audit::approval_row_redacted",
                event_type = argo_redact::AUDIT_EVENT_TYPE,
                actor_id = %actor_id,
                approval_id = %row.id,
                payload = %payload,
                "approval row redacted by server fence"
            );
        }
        entries.push((row.received_at, row.id, redaction.row));
    }

    // Newest-first by received_at, ties broken by id for determinism.
    entries.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    if let Some((ts, id)) = cursor.as_ref() {
        // Resume strictly after the prior page's last row (timestamp DESC,
        // id ASC) — append `< ts` OR (`== ts` AND `> id`).
        entries.retain(|(rec_ts, rec_id, _)| {
            rec_ts.as_str() < ts.as_str()
                || (rec_ts.as_str() == ts.as_str() && rec_id.as_str() > id.as_str())
        });
    }
    let take = entries.len().min(limit);
    let page: Vec<(String, String, Value)> = entries.into_iter().take(take).collect();
    let next_cursor = if page.len() == limit {
        page.last().map(|(ts, id, _)| encode_cursor(ts, id))
    } else {
        None
    };
    let items: Vec<Value> = page.into_iter().map(|(_, _, v)| v).collect();
    let mut body = json!({ "items": items });
    if let Some(c) = next_cursor {
        body["next_cursor"] = Value::String(c);
    }
    ("200 OK", body.to_string())
}

#[derive(Deserialize)]
struct DecideBody {
    decision: String,
}

/// `POST /api/v1/approvals/<id>/decide` — record approve/refuse.
/// Returns `{ ok, approval_id, decision }`. Idempotent: re-deciding
/// with the *same* decision returns 200; with a *different* decision
/// returns 409.
pub(super) async fn decide_response(approval_id: &str, req: &str) -> (&'static str, String) {
    if let Some(deny) = enforce_role(req, RbacAction::ApprovalDecide) {
        return deny;
    }
    let body = super::body_from_request(req);
    let Ok(parsed): Result<DecideBody, _> = serde_json::from_str(body) else {
        return bad_request("invalid JSON body");
    };
    let decision = match parsed.decision.as_str() {
        "approve" => "approve",
        "refuse" => "refuse",
        _ => return bad_request("decision must be \"approve\" or \"refuse\""),
    };
    // Probation-hold rows (shakedown F10) carry a synthetic `phold:` id and
    // resolve via the worker-page "Approve & run" control, not the
    // worker-action state machine below.
    if approval_id.starts_with("phold:") {
        return super::routes_approvals_probation::decide_probation_hold(approval_id, decision, req)
            .await;
    }
    let (memory, _cwd_ws, store) = match open_workspace_memory_with_store().await {
        Ok(p) => p,
        Err(e) => return server_error(&format!("open workspace: {e}")),
    };
    // cu1g (DEC-050): decide in the CALLER's tenant workspace with the
    // genesis-aware lookup — a Genesis teammate's held action lives in
    // the tenant workspace under a `.genesis` vertical id, both invisible
    // to the cwd+flat-only pair this handler used before.
    let tenant = match super::resolve_tenant::resolve_tenant_for_request(req, &store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let ws = match workspace_for_tenant(&store, tenant).await {
        // cu1h (DEC-050 #1): no workspace for this tenant → the action
        // cannot live anywhere this caller may touch. Fail closed as a
        // not-found rather than falling through to the demo workspace.
        Ok(Some(w)) => w,
        Ok(None) => return not_found(approval_id),
        Err(e) => return server_error(&format!("workspace: {e}")),
    };
    let Some(located) =
        (match find_action(&memory, ws, approval_id).await {
            Ok(v) => v,
            Err(e) => return server_error(&format!("find_action: {e}")),
        })
    else {
        return not_found(approval_id);
    };
    if !located.record.requires_approval {
        return bad_request("approval does not require approval");
    }
    let actor = principal_id_for_audit(req).unwrap_or_else(|| "gateway".to_string());
    let current_state = located.record.state.as_str();
    let current_decision = decision_from_record(&located.record);
    // Idempotency: same decision → no-op 200. Different decision → 409.
    if current_state != ActionState::Pending.as_str() {
        if current_decision.as_deref() == Some(decision) {
            return ok_decide(approval_id, decision);
        }
        return conflict(
            approval_id,
            current_decision.as_deref().unwrap_or(current_state),
        );
    }
    // RBAC procurement audit (mission-rbac-approvals-decide-role-gate):
    // stamp the decider's resolved role into the audit memory row.
    // `enforce_role` already short-circuited Guest above, so the only
    // roles reaching this point are owner/admin/user (or unknown in
    // dev-open mode). The role rides as a JSON-encoded note string —
    // the existing `decision_from_record` parser below handles both
    // the new structured form and the legacy `"decision:refuse"`
    // prefix so older memory rows keep round-tripping.
    let decider_role = resolve_role(req)
        .map(|r| r.as_str().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let result = if decision == "approve" {
        let note = serde_json::json!({"decider_role": decider_role}).to_string();
        worker_actions_transitions::approve(&memory, ws, approval_id, &actor, Some(&note)).await
    } else {
        let note = serde_json::json!({
            "decision": "refuse",
            "decider_role": decider_role,
        })
        .to_string();
        worker_actions_transitions::cancel(&memory, ws, approval_id, &actor, Some(&note)).await
    };
    match result {
        // Refuse → audit the decline. Approve → audit the grant AND
        // re-dispatch the held action through its executor (cu5, DEC-050
        // G5 — approval becomes real work in one click). Both branches
        // resolve the request tenant first.
        Ok(decided) => {
            super::routes_approvals_execute::finalize_decision(
                tenant, &store, &memory, ws, decision, &located, &decided, &actor, approval_id,
            )
            .await
        }
        Err(e) => server_error(&format!("decide: {e}")),
    }
}

pub(super) async fn emit_approval_declined(
    store: &Arc<SessionStore>,
    tenant_id: &str,
    actor: &str,
    pending: &ActionRecord,
    decided: &ActionRecord,
) -> Result<(), argo_types::ArgoError> {
    let payload = json!({
        "approval_id": decided.action_id,
        "decided_by": actor,
        "decision": "refuse",
        "decided_at": decided.created_at,
        "request_summary": request_summary(pending),
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string());
    sink.write(
        None,
        AuditKind::ApprovalDeclined,
        payload,
        Some(actor.to_string()),
    )
    .await
}

fn request_summary(rec: &ActionRecord) -> String {
    let raw = rec
        .payload
        .as_object()
        .and_then(|m| m.get("summary").or_else(|| m.get("text")))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| short_payload(&rec.payload));
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `POST /api/v1/approvals/<id>/undo` — reverse the decision when
/// within the [`UNDO_WINDOW_SECS`] window. Outside the window the
/// transition is terminal, so the API returns `410 Gone`.
pub(super) async fn undo_response(approval_id: &str, req: &str) -> (&'static str, String) {
    if let Some(deny) = enforce_role(req, RbacAction::ApprovalUndo) {
        return deny;
    }
    let (memory, _cwd_ws, store) = match open_workspace_memory_with_store().await {
        Ok(p) => p,
        Err(e) => return server_error(&format!("open workspace: {e}")),
    };
    // cu1g (DEC-050): same tenant-aware resolution as decide — undo must
    // find (and write back) the row in the caller's tenant workspace.
    let tenant = match super::resolve_tenant::resolve_tenant_for_request(req, &store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let ws = match workspace_for_tenant(&store, tenant).await {
        // cu1h (DEC-050 #1): no workspace for this tenant → the action
        // cannot live anywhere this caller may touch. Fail closed as a
        // not-found rather than falling through to the demo workspace.
        Ok(Some(w)) => w,
        Ok(None) => return not_found(approval_id),
        Err(e) => return server_error(&format!("workspace: {e}")),
    };
    let Some(located) =
        (match find_action(&memory, ws, approval_id).await {
            Ok(v) => v,
            Err(e) => return server_error(&format!("find_action: {e}")),
        })
    else {
        return not_found(approval_id);
    };
    if located.record.state == ActionState::Pending.as_str() {
        return bad_request("approval has not been decided yet");
    }
    if decision_from_record(&located.record).is_none() {
        return bad_request("approval was not decided via the JSON API");
    }
    let decided_at = chrono::DateTime::parse_from_rfc3339(&located.record.created_at)
        .map(|dt| dt.with_timezone(&chrono::Utc));
    let now = chrono::Utc::now();
    let within_window = matches!(decided_at, Ok(t) if (now - t).num_seconds() <= UNDO_WINDOW_SECS);
    if !within_window {
        return gone(approval_id);
    }
    // Restore to pending. Append a memory row directly — the state-machine
    // validator forbids `approved/cancelled → pending`, but appending a
    // newer row is the audit-clean way to revert (parallels the
    // `reject_action` direct-append in `routes_channels.rs`).
    let mut next = located.record.clone();
    next.state = ActionState::Pending.as_str().to_string();
    next.created_at = now.to_rfc3339();
    next.approved_by = None;
    next.approved_at = None;
    next.result = Some(json!({"undone_at": next.created_at}));
    let body = match serde_json::to_string(&next) {
        Ok(s) => s,
        Err(e) => return server_error(&format!("serialize: {e}")),
    };
    if let Err(e) = memory
        .append_for_vertical(ws, MemoryFile::Memory, body, None, &located.vertical_id)
        .await
    {
        return server_error(&format!("undo append: {e}"));
    }
    (
        "200 OK",
        json!({"ok": true, "approval_id": approval_id}).to_string(),
    )
}

#[path = "routes_approvals_json_format.rs"]
mod fmt;
use fmt::{approval_row_json, decision_from_record};
#[cfg(test)]
use fmt::decider_role_from_record;

#[cfg(test)]
#[path = "routes_approvals_json_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "routes_approvals_execute_tests.rs"]
mod execute_tests;

#[cfg(test)]
#[path = "routes_approvals_role_gate_tests.rs"]
mod role_gate_tests;
