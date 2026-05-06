//! `argo worker actions list` filter chain. Sibling of
//! `worker_actions.rs` so that file stays under the 500-LOC
//! budget as more filters land (#766).
//!
//! Pure transformation — takes a `Vec<ActionRecord>` and the
//! per-flag filters and returns the filtered, unsorted result.
//! No I/O. Easy to unit-test the entire matrix of filter
//! combinations without spinning up the CLI + memory store.

use super::worker_actions::ActionRecord;

/// Apply the list filter chain to the latest-snapshot action
/// vector.
///
/// Semantics:
/// * `state_filter == "all"` matches every state; otherwise
///   exact string match on `state`.
/// * `job_id`, `action_type_filter`, `platform_filter`,
///   `item_id_filter`: when `Some(_)`, demand exact match;
///   `None` is the no-op identity. Rows whose `item_id` is
///   `None` are EXCLUDED whenever `item_id_filter` is set —
///   the operator's "show me skate-pair-A" query should not
///   surface item-less actions.
pub(crate) fn apply_list_filters(
    actions: Vec<ActionRecord>,
    state_filter: &str,
    job_id: Option<&str>,
    item_id_filter: Option<&str>,
    action_type_filter: Option<&str>,
    platform_filter: Option<&str>,
) -> Vec<ActionRecord> {
    actions
        .into_iter()
        .filter(|a| match state_filter {
            "all" => true,
            other => a.state == other,
        })
        .filter(|a| job_id.is_none_or(|j| a.job_id == j))
        .filter(|a| item_id_filter.is_none_or(|want| a.item_id.as_deref() == Some(want)))
        .filter(|a| action_type_filter.is_none_or(|t| a.action_type == t))
        .filter(|a| platform_filter.is_none_or(|p| a.platform.as_deref() == Some(p)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rec(action_id: &str, state: &str, item_id: Option<&str>) -> ActionRecord {
        ActionRecord {
            schema_key: ActionRecord::SCHEMA_KEY.to_string(),
            action_id: action_id.to_string(),
            job_id: "job-1".to_string(),
            worker_id: argo_identity::WorkerId::nil(),
            item_id: item_id.map(String::from),
            action_type: "verify_price".to_string(),
            risk_level: "low".to_string(),
            requires_approval: false,
            state: state.to_string(),
            payload: json!({}),
            approved_by: None,
            approved_at: None,
            executed_at: None,
            platform: None,
            result: None,
            evidence: vec![],
            blocked_reason: None,
            created_at: "2026-05-06T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn item_id_filter_matches_only_named_item() {
        // Three rows: skates-A, skates-A, skates-B. Filter on
        // skates-A returns the two A rows.
        let rows = vec![
            rec("a-1", "pending", Some("skates-A")),
            rec("a-2", "completed", Some("skates-A")),
            rec("a-3", "pending", Some("skates-B")),
        ];
        let out = apply_list_filters(rows, "all", None, Some("skates-A"), None, None);
        assert_eq!(out.len(), 2);
        for r in out {
            assert_eq!(r.item_id.as_deref(), Some("skates-A"));
        }
    }

    #[test]
    fn item_id_filter_excludes_rows_with_no_item_id() {
        // Operator querying for `skates-A` must not see
        // item-less rows leaking through (e.g. system-level
        // notifications, vertical-scoped maintenance actions).
        let rows = vec![
            rec("a-1", "pending", Some("skates-A")),
            rec("a-2", "pending", None),
            rec("a-3", "completed", None),
        ];
        let out = apply_list_filters(rows, "all", None, Some("skates-A"), None, None);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].action_id, "a-1");
    }

    #[test]
    fn item_id_none_is_no_op_passthrough() {
        // Default behaviour (no --item-id passed) keeps all
        // rows; locks the back-compat contract.
        let rows = vec![
            rec("a-1", "pending", Some("skates-A")),
            rec("a-2", "pending", None),
        ];
        let out = apply_list_filters(rows, "all", None, None, None, None);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn item_id_combines_with_state_and_job_filters() {
        // The interesting operator query: "what's awaiting
        // approval for skates-A right now?" — state=pending,
        // item_id=skates-A, no other filters. Combined filters
        // must AND together, not OR.
        let rows = vec![
            rec("a-1", "pending", Some("skates-A")),
            rec("a-2", "completed", Some("skates-A")),
            rec("a-3", "pending", Some("skates-B")),
        ];
        let out = apply_list_filters(rows, "pending", None, Some("skates-A"), None, None);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].action_id, "a-1");
    }

    #[test]
    fn item_id_unknown_returns_empty_not_error() {
        // Querying for an item_id that doesn't exist must
        // return an empty vec, not panic / error. The CLI
        // surfaces this as `(no actions)` to the operator.
        let rows = vec![rec("a-1", "pending", Some("skates-A"))];
        let out = apply_list_filters(rows, "all", None, Some("skates-Z"), None, None);
        assert!(out.is_empty());
    }
}
