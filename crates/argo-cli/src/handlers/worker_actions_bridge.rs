//! The chat→delivery bridge (WAVE-2 ⑥a).
//!
//! # The gap this closes
//!
//! Before this module the **chat plane** and the **executor plane** never
//! touched. A teammate could *draft* an action mid-turn, but the only
//! runtime path that turned a draft into a persisted `worker_action` row was
//! the batch worker job-loop ([`super::work_emission`]), which scrapes
//! ` ```action ` fenced blocks out of finished assistant text. An interactive
//! teammate that wanted to actually *deliver* something (write a document,
//! call a connector, post over a channel) had no in-turn entry point: it
//! drafted and stopped. Worse, the one outbound primitive it *did* have
//! ([`argo_tool::SendMessage`]) is an unwired direct-send that — if wired —
//! would transmit with **no approval gate at all**.
//!
//! This module is the missing seam. It lets the runtime materialize a
//! teammate's proposed action into a real `worker_action` row that
//!
//! 1. **targets the REAL executor** the bound vertical declares for that
//!    `action_type` (document / mcp / browser / connector / …), NOT a dead-end
//!    `human` bundle, and
//! 2. is born **`state = "pending"` behind the approval fence** — exactly the
//!    same gate [`super::worker_actions_emit::build_emission`] stamps for the
//!    batch path. Nothing auto-delivers; nothing bypasses approval.
//!
//! # Why it cannot bypass approval (the safety spine)
//!
//! The row is built by the SAME [`build_emission`] the batch path uses, so
//! `risk_level` + `requires_approval` are stamped from the manifest, never
//! from the model. [`enqueue_action_for_delivery`] then makes the gate
//! explicit and fail-closed:
//!
//! * it **refuses** any action the manifest maps to `executor = "human"` —
//!   that is the born-`human`-and-stop bug, and an `enqueue` that produced a
//!   human bundle would be a no-op delivery;
//! * it **refuses** any action whose declaration does not require approval —
//!   the bridge is the *approval-gated* delivery path by construction, so a
//!   `requires_approval = false` action would be an auto-deliver, which is
//!   exactly what must not happen here;
//! * it **asserts** the built row is `state = "pending"` before persisting —
//!   a belt-and-braces check that the action is parked at the gate, not
//!   executing.
//!
//! On operator approval the EXISTING execute path
//! ([`super::worker_actions_executor::resolve_executor`]) resolves the same
//! declared executor and delivers. This module never runs an executor.
//!
//! # Truth-bounded return (DEC-029 NO THEATER)
//!
//! [`enqueue_action_for_delivery`] returns
//! [`BridgeOutcome::SubmittedForApproval`] — literally "submitted for
//! approval", never "done" / "delivered". The teammate that called it must
//! report the same.

use super::worker_actions::ActionRecord;
use super::worker_actions_emit::{build_emission, persist_action_record, ActionEmissionInputs};
use argo_memory::MemoryStore;
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use serde_json::Value;

/// Outcome of an [`enqueue_action_for_delivery`] call. There is no
/// `Delivered` / `Completed` variant by design — the bridge stops at the
/// approval gate, so the only success it can honestly report is
/// "submitted for approval".
#[derive(Debug, Clone)]
// Now wired: the in-turn `enqueue_action` tool reaches this through
// [`super::worker_actions_bridge_dispatcher::BridgeDeliverDispatcher`], the
// `argo-cli` [`argo_tool::DeliverDispatcher`] impl injected into the per-turn
// tool registry. The `#[allow(dead_code)]` came off when that real caller
// landed (WAVE-2 ⑥b).
pub(crate) enum BridgeOutcome {
    /// The action was materialized targeting a real executor and persisted as
    /// a `pending` / `requires_approval = true` row. It now awaits an operator
    /// decision in `/approvals`; the existing executor path delivers it on
    /// approval. NOT delivered.
    SubmittedForApproval {
        /// The persisted ledger row (state = `pending`).
        record: ActionRecord,
        /// The real executor the action targets (e.g. `document_csv`, `mcp`,
        /// `browser`). Never `human`.
        executor_kind: String,
    },
}

/// Materialize a teammate's proposed `action_type` (+ payload) into a
/// `worker_action` row that targets the REAL executor the bound vertical
/// declares for it, and submit it through the EXISTING approval gate as
/// **pending-approval**. Persists the row; does NOT execute it.
///
/// This is the in-turn entry point the chat plane was missing: it bridges a
/// drafted action to a real executor while keeping the operator in the loop.
///
/// # Safety (the #1 invariant)
///
/// The returned/persisted action is ALWAYS `state = "pending"` with
/// `requires_approval = true`, targeting a non-`human` executor. The function
/// fails closed rather than ever producing an auto-delivering or
/// human-dead-end row:
///
/// * `Validation("not declared")` — the `action_type` is not in the bound
///   vertical's `[actions]` table (same fail-closed posture as
///   [`super::worker_actions_emit::classify_for_emission`]'s `Forbidden`).
/// * `Validation("human dead-end")` — the manifest maps the action to
///   `executor = "human"`; the bridge refuses to enqueue a no-op human bundle
///   (that is the born-`human`-and-stop bug it exists to fix).
/// * `Validation("auto-deliver")` — the action does not require approval; the
///   bridge is the approval-gated path by construction and refuses to be the
///   vehicle for an un-gated send.
///
/// On success the row is persisted to the workspace's memory namespace as a
/// `worker_action` row scoped to `vertical_id`, identical to the batch
/// emission path, so it surfaces in `/approvals` and `argo worker actions
/// list` unchanged.
///
/// # Errors
///
/// Returns [`ArgoError::Schema`] for the fail-closed caller-data cases above,
/// [`ArgoError::Invariant`] for the belt-and-braces state assertion, or
/// propagates a persistence error from [`persist_action_record`].
///
/// The live caller is
/// [`super::worker_actions_bridge_dispatcher::BridgeDeliverDispatcher::enqueue`],
/// which the in-turn `enqueue_action` tool dispatches into.
// Each argument is load-bearing context the action row must carry (the
// memory store + workspace to persist into, the bound manifest to resolve
// the real executor, the vertical/job/worker scope, and the model's
// action_type + payload). Bundling them would just move the arity into a
// struct.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn enqueue_action_for_delivery(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    vertical: &argo_vertical::VerticalManifest,
    vertical_id: &str,
    job_id: &str,
    worker_id: argo_identity::WorkerId,
    action_type: &str,
    payload: Value,
) -> ArgoResult<BridgeOutcome> {
    // Steps 1–3 (declared? real executor? approval-gated?) — fail closed.
    let executor_kind = resolve_real_executor_kind(vertical, vertical_id, action_type)?;

    // Build the row through the SAME emitter the batch path uses: it stamps
    // risk_level + requires_approval from the manifest (the model cannot
    // override them) and sets state = "pending" — i.e. the approval gate.
    let inputs = ActionEmissionInputs {
        vertical,
        job_id,
        worker_id,
        action_type,
        payload,
        item_id: None,
        platform: None,
        evidence: vec!["argo://bridge/chat-delivery".to_string()],
        risk_override: None,
    };
    let record = build_emission(&inputs);

    // SAFETY (belt-and-braces): the persisted row MUST be parked at the gate.
    // Assert rather than trust, so a future emitter change can never silently
    // let an executing/blocked row out of the bridge.
    if record.state != "pending" || !record.requires_approval {
        return Err(ArgoError::Invariant(format!(
            "bridge invariant violated: action '{action_type}' did not \
             materialize as pending/requires_approval (got state='{}', \
             requires_approval={}); refusing to persist",
            record.state, record.requires_approval
        )));
    }

    // Persist as a worker_action row — identical to the batch path, so it
    // surfaces in /approvals + `argo worker actions list`. The existing
    // executor path delivers it ON APPROVAL; the bridge never executes.
    persist_action_record(memory, workspace, vertical_id, &record).await?;

    Ok(BridgeOutcome::SubmittedForApproval {
        record,
        executor_kind,
    })
}

/// Resolve the REAL executor the bound vertical declares for `action_type`,
/// fail-closed on the two unsafe shapes the bridge must never enqueue.
///
/// Reads the executor straight off the manifest (the same declaration
/// [`super::worker_actions_executor::resolve_executor`] routes on at execute
/// time), then enforces:
///
/// * declared — undeclared `action_type` → [`ArgoError::Schema`];
/// * non-`human` — refuse the born-`human`-and-stop dead-end (a `human` row
///   renders a bundle and stops, which is the bug, not a delivery);
/// * `requires_approval = true` — refuse to be the vehicle for an un-gated
///   send (a `requires_approval = false` action would auto-execute on the
///   dispatch path, bypassing the very gate this seam exists to enforce).
///
/// Returns the validated executor kind string (never `"human"`).
fn resolve_real_executor_kind(
    vertical: &argo_vertical::VerticalManifest,
    vertical_id: &str,
    action_type: &str,
) -> ArgoResult<String> {
    let decl = vertical
        .actions
        .as_ref()
        .and_then(|s| s.actions.get(action_type))
        .ok_or_else(|| {
            ArgoError::Schema(format!(
                "action_type '{action_type}' is not declared in vertical \
                 '{vertical_id}'; the bridge cannot route an undeclared action \
                 to an executor"
            ))
        })?;

    let executor_kind = decl.executor.kind_str().to_string();
    if executor_kind == "human" {
        return Err(ArgoError::Schema(format!(
            "action '{action_type}' targets executor=human (a draft→stop \
             dead-end); the chat→delivery bridge only enqueues actions bound \
             to a real executor. Bind a real executor in the vertical's \
             [actions.{action_type}] declaration first."
        )));
    }
    if !decl.requires_approval {
        return Err(ArgoError::Schema(format!(
            "action '{action_type}' is declared requires_approval=false; the \
             chat→delivery bridge only submits approval-gated actions so \
             nothing auto-delivers"
        )));
    }
    Ok(executor_kind)
}

#[cfg(test)]
#[path = "worker_actions_bridge_tests.rs"]
mod tests;
