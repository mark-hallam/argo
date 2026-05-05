//! Worker v3 action artifact renderers (#729).
//!
//! Sibling of `worker_actions_emit` so that file stays under
//! the per-file 500-LOC budget. This module owns the per-job
//! artifact bundle's machine-readable view (`actions.json`);
//! the human-readable markdown view (`execution_plan.md`)
//! still lives in `worker_actions_emit::render_execution_plan`
//! because the existing renderer tests are sized to live there.
//!
//! See `docs/design/worker_v3_execution.md` §6 for the full
//! artifact spec — the markdown is for human reviewers; the
//! JSON is the contract that downstream tooling, dashboards,
//! and integration tests consume.
//!
//! No I/O happens here — every function is pure (record slice
//! → string). The caller (`work_emission`) decides where to
//! write.

use super::worker_actions::ActionRecord;

/// Render the machine-readable `actions.json` mirror of
/// `execution_plan.md`. Tooling, dashboards, and integration
/// tests consume this instead of parsing the markdown — same
/// data, structured.
///
/// Shape:
///
/// ```json
/// {
///   "schema_version": "1",
///   "job_id": "<uuid>",
///   "vertical_id": "<id>",
///   "evidence_sufficient": true,
///   "counts": { "total": 3, "pending_approval": 1,
///               "pending_autonomous": 1, "blocked": 1 },
///   "actions": [ ActionRecord, ... ]
/// }
/// ```
///
/// The `actions` array preserves emission order so the caller
/// can match against the markdown's grouping if needed. Pure
/// function — no I/O — so it's trivial to unit-test.
pub(crate) fn render_actions_json(
    job_id: &str,
    vertical_id: &str,
    evidence_sufficient: bool,
    records: &[ActionRecord],
) -> String {
    let mut pending_approval = 0u32;
    let mut pending_autonomous = 0u32;
    let mut blocked = 0u32;
    for r in records {
        match (r.state.as_str(), r.requires_approval) {
            ("pending", true) => pending_approval += 1,
            ("pending", false) => pending_autonomous += 1,
            ("blocked", _) => blocked += 1,
            _ => {}
        }
    }
    let payload = serde_json::json!({
        "schema_version": "1",
        "job_id": job_id,
        "vertical_id": vertical_id,
        "evidence_sufficient": evidence_sufficient,
        "counts": {
            "total": records.len(),
            "pending_approval": pending_approval,
            "pending_autonomous": pending_autonomous,
            "blocked": blocked,
        },
        "actions": records,
    });
    serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
}

// ───────────── workspace per-state ledger snapshots (#730) ──────────

/// Render `approval_queue.md` — the operator's review queue.
/// Lists every action currently in `state = pending` with
/// `requires_approval = true`. Empty state gets a clear
/// placeholder rather than a zero-byte file.
pub(crate) fn render_approval_queue(records: &[ActionRecord]) -> String {
    let queue: Vec<&ActionRecord> = records
        .iter()
        .filter(|r| r.state == "pending" && r.requires_approval)
        .collect();
    let mut out = String::from("# Approval queue\n\n");
    if queue.is_empty() {
        out.push_str("_(no actions awaiting approval)_\n");
        return out;
    }
    out.push_str(&format!(
        "{} action(s) awaiting an explicit \
         `argo worker actions approve <id>` before they can advance.\n\n",
        queue.len()
    ));
    for r in queue {
        out.push_str(&render_action_summary(r));
    }
    out
}

/// Render `completed_actions.md` — the audit trail. Lists every
/// terminal `completed` action newest-first with the operator-
/// supplied summary (when present).
pub(crate) fn render_completed_actions(records: &[ActionRecord]) -> String {
    let mut completed: Vec<&ActionRecord> =
        records.iter().filter(|r| r.state == "completed").collect();
    completed.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let mut out = String::from("# Completed actions\n\n");
    if completed.is_empty() {
        out.push_str("_(no actions have completed yet)_\n");
        return out;
    }
    out.push_str(&format!("{} completed action(s).\n\n", completed.len()));
    for r in completed {
        out.push_str(&render_action_summary(r));
        if let Some(result) = r.result.as_ref() {
            if let Some(summary) = result.get("summary").and_then(|s| s.as_str()) {
                out.push_str(&format!("    - summary: {summary}\n"));
            }
        }
        if let Some(at) = r.executed_at.as_deref() {
            out.push_str(&format!("    - executed_at: {at}\n"));
        }
    }
    out
}

/// Render `blocked_actions.md` — the investigation queue. Lists
/// every `blocked` action with the rejection reason inline so
/// the operator sees what failed without drilling into JSON.
pub(crate) fn render_blocked_actions(records: &[ActionRecord]) -> String {
    let mut blocked: Vec<&ActionRecord> = records.iter().filter(|r| r.state == "blocked").collect();
    blocked.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let mut out = String::from("# Blocked actions\n\n");
    if blocked.is_empty() {
        out.push_str("_(no actions are blocked)_\n");
        return out;
    }
    out.push_str(&format!(
        "{} blocked action(s) — the operator must address the cause \
         before re-emitting.\n\n",
        blocked.len()
    ));
    for r in blocked {
        out.push_str(&render_action_summary(r));
        if let Some(reason) = r.blocked_reason.as_deref() {
            out.push_str(&format!("    - reason: {reason}\n"));
        }
    }
    out
}

/// Bullet line shared by all three ledger renderers. Keeps the
/// shape consistent so operators see the same identifier
/// information regardless of which file they're reading.
fn render_action_summary(r: &ActionRecord) -> String {
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
        "- `{}` — `{}` (risk: {}, job: `{}`){platform}{item}\n",
        r.action_id, r.action_type, r.risk_level, r.job_id,
    )
}

#[cfg(test)]
#[path = "worker_actions_render_tests.rs"]
mod tests;
