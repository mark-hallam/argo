//! JSON serialization helpers for the approvals routes, split out of
//! `routes_approvals_json.rs` to keep that file under the 500-LOC ceiling
//! (cu1g DEC-050 tenant/genesis wiring pushed it over). These build the
//! frontend `ApprovalItem` row and parse the recorded decision /
//! decider-role back out of an `ActionRecord`.

use crate::handlers::gateway::routes_approvals_json_helpers::short_payload;
use crate::handlers::worker_actions::{ActionRecord, ActionState};
use crate::handlers::worker_actions_auto_approval::draft_content_from_payload;
use serde_json::{json, Value};

pub(super) fn approval_row_json(rec: &ActionRecord) -> Value {
    // The frontend `ApprovalItem` shape (web/lib/sample-approvals.ts)
    // carries UX-specific fields (verb / detail / consequences / reason)
    // that don't yet exist on `ActionRecord`. v1 derives placeholders
    // from the record fields available; J3+ ship the structured prompt
    // pack that feeds these directly.
    let action_type = rec.action_type.replace('_', " ");
    let detail = rec
        .payload
        .as_object()
        .and_then(|m| m.get("summary").or_else(|| m.get("text")))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| short_payload(&rec.payload));
    let draft_content = draft_content_from_payload(&rec.payload)
        .map(Value::String)
        .unwrap_or(Value::Null);
    json!({
        "id": rec.action_id,
        "worker": {
            "id": rec.worker_id.to_string(),
            "name": rec.worker_id.to_string(),
            "avatar_url": Value::Null,
        },
        "verb": format!("wants to {action_type}"),
        "detail": detail,
        "consequence_if_yes": "",
        "consequence_if_no": "",
        "reason": Value::Null,
        "draft_content": draft_content,
        "channel": "web",
        "received_at": rec.created_at,
        "decision": Value::Null,
        "risk_level": rec.risk_level,
    })
}

pub(super) fn decision_from_record(rec: &ActionRecord) -> Option<String> {
    // An approval-gated action that was approved carries `approved_by`,
    // and the cu5 approve-and-run path then advances it through
    // executing → completed|failed — transitions that PRESERVE
    // `approved_by`. Treat any `approved_by`-stamped row as an "approve"
    // decision so a re-decide stays idempotent (200, not a spurious 409)
    // and undo still recognises the row as JSON-API-decided even after it
    // has executed.
    if rec.approved_by.is_some() {
        return Some("approve".to_string());
    }
    match rec.state.as_str() {
        s if s == ActionState::Cancelled.as_str() => rec
            .result
            .as_ref()
            .and_then(|r| r.pointer("/cancellation/note"))
            .and_then(Value::as_str)
            .and_then(|n| {
                // Structured form (post-RBAC-role-gate brief): JSON
                // string with `"decision":"refuse"` and the
                // `decider_role` claim.
                if let Ok(v) = serde_json::from_str::<Value>(n) {
                    if v.get("decision").and_then(Value::as_str) == Some("refuse") {
                        return Some("refuse".to_string());
                    }
                }
                // Legacy form: plain `"decision:refuse"` prefix.
                n.starts_with("decision:refuse")
                    .then(|| "refuse".to_string())
            }),
        _ => None,
    }
}

/// Extract the recorded decider role from an action's audit memory row.
///
/// Returns `None` for legacy rows that pre-date the
/// `mission-rbac-approvals-decide-role-gate` audit stamp (where the
/// `approval_note` / `cancellation.note` field carried only a
/// free-form string). New rows carry a JSON-encoded note containing
/// `decider_role`; this helper deserialises that field for the
/// audit-feed and SPA caller layers.
#[cfg(test)]
pub(super) fn decider_role_from_record(rec: &ActionRecord) -> Option<String> {
    let note = match rec.state.as_str() {
        s if s == ActionState::Approved.as_str() => rec
            .result
            .as_ref()
            .and_then(|r| r.get("approval_note"))
            .and_then(Value::as_str)?,
        s if s == ActionState::Cancelled.as_str() => rec
            .result
            .as_ref()
            .and_then(|r| r.pointer("/cancellation/note"))
            .and_then(Value::as_str)?,
        _ => return None,
    };
    serde_json::from_str::<Value>(note)
        .ok()
        .and_then(|v| {
            v.get("decider_role")
                .and_then(Value::as_str)
                .map(String::from)
        })
}
