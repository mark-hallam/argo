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

#[cfg(test)]
#[path = "worker_actions_render_tests.rs"]
mod tests;
