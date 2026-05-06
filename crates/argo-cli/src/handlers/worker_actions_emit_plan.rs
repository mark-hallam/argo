//! `execution_plan.md` renderer for the action-emission pipeline
//! (#729). Pure transformation: takes a slice of `ActionRecord`s
//! and returns the markdown string.
//!
//! Pulled out of `worker_actions_emit.rs` so that file stays
//! under the per-file 500-LOC budget as more emission gates land
//! (evidence #729, sold-item #769, future …).

use crate::handlers::worker_actions::ActionRecord;

/// Render the `execution_plan.md` artifact alongside the rest
/// of the per-job bundle. Groups emitted actions by state with
/// the operator's reading order:
///
///   1. **Pending approval** — the queue the human reviews next
///   2. **Pending (autonomous)** — safe-autonomous actions ready
///      to run without approval
///   3. **Blocked** — undeclared action_types, evidence-gated
///      approval-gated emissions, or sold-item gated emissions,
///      with the `blocked_reason` so the operator sees the gap
///
/// When the worker emitted no actions, returns a clear empty-
/// state placeholder rather than an empty file. The renderer is
/// pure — takes a slice of records, returns a string — so it's
/// trivial to unit-test alongside the orchestrator.
pub(crate) fn render_execution_plan(records: &[ActionRecord]) -> String {
    if records.is_empty() {
        return "# Execution plan\n\n_(no actions emitted this run)_\n".to_string();
    }
    let mut pending_approval: Vec<&ActionRecord> = Vec::new();
    let mut pending_safe: Vec<&ActionRecord> = Vec::new();
    let mut blocked: Vec<&ActionRecord> = Vec::new();
    for r in records {
        match (r.state.as_str(), r.requires_approval) {
            ("pending", true) => pending_approval.push(r),
            ("pending", false) => pending_safe.push(r),
            ("blocked", _) => blocked.push(r),
            _ => {} // approved/executing/completed/failed not seen at emission time
        }
    }

    let mut out = String::from("# Execution plan\n\n");
    out.push_str(&format!(
        "Emitted {} action(s) — {} pending approval, {} \
         autonomous-safe, {} blocked.\n\n",
        records.len(),
        pending_approval.len(),
        pending_safe.len(),
        blocked.len(),
    ));

    if !pending_approval.is_empty() {
        out.push_str("## Pending approval\n\n");
        out.push_str(
            "These actions require an explicit `argo worker actions approve <id>` \
             before they advance.\n\n",
        );
        for r in &pending_approval {
            out.push_str(&render_action_line(r));
        }
        out.push('\n');
    }

    if !pending_safe.is_empty() {
        out.push_str("## Pending (autonomous-safe)\n\n");
        out.push_str(
            "Autonomous-safe actions — the worker may advance these without \
             operator approval per the bound vertical's manifest.\n\n",
        );
        for r in &pending_safe {
            out.push_str(&render_action_line(r));
        }
        out.push('\n');
    }

    if !blocked.is_empty() {
        out.push_str("## Blocked\n\n");
        out.push_str(
            "These emissions were rejected by the OS — undeclared action types, \
             approval-gated emissions on top of insufficient research evidence, \
             or new actions for items already marked sold. The operator must \
             address the cause before re-emitting.\n\n",
        );
        for r in &blocked {
            out.push_str(&render_action_line(r));
            if let Some(reason) = r.blocked_reason.as_deref() {
                out.push_str(&format!("    - reason: {reason}\n"));
            }
        }
        out.push('\n');
    }

    out
}

fn render_action_line(r: &ActionRecord) -> String {
    let platform = r
        .platform
        .as_deref()
        .map(|p| format!(" · platform `{p}`"))
        .unwrap_or_default();
    let item = r
        .item_id
        .as_deref()
        .map(|i| format!(" · item `{i}`"))
        .unwrap_or_default();
    format!(
        "- `{}` — `{}` (risk: {}){platform}{item}\n",
        r.action_id, r.action_type, r.risk_level,
    )
}
