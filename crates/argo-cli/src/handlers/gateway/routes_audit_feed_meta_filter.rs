//! Default `/audit` view hides system-meta rows (SPA self-traffic
//! like `audit.detail.viewed`, `audit.feed.queried`, `http.request`,
//! `worker.kpi_rolled`). The parent route opts callers back in with
//! `?include_meta=1` for forensics.
//!
//! Mirrors `routes_worker_activity::is_system_meta_row` so both
//! user-facing audit endpoints share the same
//! `argo_audit::METADATA_VERBS` filter list — `argo-audit`'s module
//! doc on `metadata_verbs` is the single source of truth.

use argo_audit::{is_metadata_verb, ExportedRow};
use serde_json::Value;

/// Match `row` against `argo_audit::METADATA_VERBS` along the same
/// dimensions `/api/v1/workers/<id>/activity` checks: the resolved
/// verb (`payload.verb` then `row.kind`), plus the `endpoint` and
/// `action_type` payload keys. Returns `true` when the row is system
/// meta and should be hidden from the default feed.
pub(super) fn is_system_meta_row(row: &ExportedRow) -> bool {
    let resolved_verb = payload_str(&row.payload, "verb").unwrap_or_else(|| row.kind.clone());
    if is_metadata_verb(&resolved_verb) {
        return true;
    }
    for key in ["endpoint", "action_type"] {
        if let Some(v) = payload_str(&row.payload, key) {
            if is_metadata_verb(&v) {
                return true;
            }
        }
    }
    false
}

fn payload_str(p: &Value, key: &str) -> Option<String> {
    p.get(key).and_then(Value::as_str).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row(kind: &str, payload: Value) -> ExportedRow {
        ExportedRow {
            seq: 0,
            id: "test".into(),
            session_id: None,
            vertical_id: None,
            kind: kind.into(),
            payload,
            parent_hash: String::new(),
            self_hash: String::new(),
            created_at_micros: 0,
        }
    }

    #[test]
    fn http_request_kind_is_meta() {
        assert!(is_system_meta_row(&row("http.request", json!({}))));
    }

    #[test]
    fn audit_detail_viewed_endpoint_is_meta() {
        let r = row("http.request", json!({"endpoint": "audit.detail.viewed"}));
        assert!(is_system_meta_row(&r));
    }

    #[test]
    fn audit_feed_queried_action_type_is_meta() {
        let r = row("http.request", json!({"action_type": "audit.feed.queried"}));
        assert!(is_system_meta_row(&r));
    }

    #[test]
    fn skill_invoked_action_type_is_not_meta() {
        let r = row(
            "tool_call",
            json!({"action_type": "skill.invoked", "verb": "skill.invoked"}),
        );
        assert!(!is_system_meta_row(&r));
    }

    #[test]
    fn worker_kpi_rolled_kind_is_meta_via_verb_fallback() {
        let r = row("worker.kpi_rolled", json!({}));
        assert!(is_system_meta_row(&r));
    }

    #[test]
    fn approval_granted_real_row_is_not_meta() {
        let r = row(
            "approval.granted",
            json!({"verb": "approval.granted", "approver_id": "op-1"}),
        );
        assert!(!is_system_meta_row(&r));
    }
}
