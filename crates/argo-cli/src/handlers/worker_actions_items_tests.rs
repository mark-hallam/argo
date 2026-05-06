//! Unit tests for `worker_actions_items` (#767, #768). Sibling of
//! the impl module so the impl file stays under the per-file
//! 500-LOC budget.
//!
//! Loaded via `#[cfg(test)] #[path = "worker_actions_items_tests.rs"]
//! mod tests;` at the bottom of `worker_actions_items.rs`.

use super::*;
use serde_json::json;

fn rec(
    action_id: &str,
    item_id: Option<&str>,
    action_type: &str,
    state: &str,
    created_at: &str,
) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: action_id.to_string(),
        job_id: "job-1".to_string(),
        item_id: item_id.map(String::from),
        action_type: action_type.to_string(),
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
        created_at: created_at.to_string(),
    }
}

#[test]
fn rollup_groups_per_item_id_within_vertical() {
    let tuples = vec![
        ("gtm".to_string(), rec("a-1", Some("skates-A"), "verify_price", "completed", "2026-05-06T10:00:00Z")),
        ("gtm".to_string(), rec("a-2", Some("skates-A"), "post_listing", "pending", "2026-05-06T10:01:00Z")),
        ("gtm".to_string(), rec("a-3", Some("skates-B"), "verify_price", "completed", "2026-05-06T10:02:00Z")),
    ];
    let mut summaries = rollup_items(tuples);
    summaries.sort_by(|a, b| a.item_id.cmp(&b.item_id));
    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries[0].item_id, "skates-A");
    assert_eq!(summaries[0].actions_total, 2);
    assert_eq!(summaries[0].latest_action_type, "post_listing");
    assert_eq!(summaries[0].latest_state, "pending");
    assert_eq!(summaries[1].item_id, "skates-B");
    assert_eq!(summaries[1].actions_total, 1);
}

#[test]
fn rollup_marks_sold_when_mark_item_sold_completed_exists() {
    let tuples = vec![
        ("gtm".to_string(), rec("a-1", Some("skates-A"), "post_listing", "completed", "2026-05-06T10:00:00Z")),
        ("gtm".to_string(), rec("a-2", Some("skates-A"), "mark_item_sold", "completed", "2026-05-06T10:01:00Z")),
    ];
    let summaries = rollup_items(tuples);
    assert_eq!(summaries.len(), 1);
    assert!(summaries[0].is_sold, "completed mark_item_sold must flip is_sold");
}

#[test]
fn rollup_does_not_mark_sold_for_pending_mark_item_sold() {
    // Operator has approved nothing yet — item is still in
    // inventory. Locks the "only completion flips the bit"
    // rule from the module docs.
    let tuples = vec![
        ("gtm".to_string(), rec("a-1", Some("skates-A"), "mark_item_sold", "pending", "2026-05-06T10:00:00Z")),
        ("gtm".to_string(), rec("a-2", Some("skates-A"), "mark_item_sold", "approved", "2026-05-06T10:01:00Z")),
    ];
    let summaries = rollup_items(tuples);
    assert_eq!(summaries.len(), 1);
    assert!(!summaries[0].is_sold, "pre-completion mark_item_sold rows must not flip is_sold");
}

#[test]
fn rollup_skips_item_less_rows() {
    // Item-less actions (e.g. system maintenance) are not
    // inventory; they shouldn't appear in the items view.
    let tuples = vec![
        ("gtm".to_string(), rec("a-1", None, "verify_price", "completed", "2026-05-06T10:00:00Z")),
        ("gtm".to_string(), rec("a-2", Some("skates-A"), "verify_price", "completed", "2026-05-06T10:01:00Z")),
    ];
    let summaries = rollup_items(tuples);
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].item_id, "skates-A");
}

#[test]
fn rollup_by_state_counts_match_per_action_states() {
    // 3 actions for skates-A: 1 completed, 1 pending, 1
    // blocked. Counts should reflect that.
    let tuples = vec![
        ("gtm".to_string(), rec("a-1", Some("skates-A"), "verify_price", "completed", "2026-05-06T10:00:00Z")),
        ("gtm".to_string(), rec("a-2", Some("skates-A"), "post_listing", "pending", "2026-05-06T10:01:00Z")),
        ("gtm".to_string(), rec("a-3", Some("skates-A"), "boost_listing", "blocked", "2026-05-06T10:02:00Z")),
    ];
    let summaries = rollup_items(tuples);
    assert_eq!(summaries.len(), 1);
    let s = &summaries[0];
    assert_eq!(s.by_state.get("completed"), Some(&1));
    assert_eq!(s.by_state.get("pending"), Some(&1));
    assert_eq!(s.by_state.get("blocked"), Some(&1));
    assert_eq!(s.by_state.get("approved"), Some(&0));
}

#[test]
fn rollup_separates_same_item_id_under_different_verticals() {
    // skates-A in two different vertical namespaces is two
    // separate items. Locks the cross-vertical isolation
    // contract for the items view.
    let tuples = vec![
        ("gtm".to_string(), rec("a-1", Some("skates-A"), "verify_price", "completed", "2026-05-06T10:00:00Z")),
        ("other".to_string(), rec("a-2", Some("skates-A"), "verify_price", "completed", "2026-05-06T10:01:00Z")),
    ];
    let summaries = rollup_items(tuples);
    assert_eq!(summaries.len(), 2);
}

/// Pure helper mirroring the sort step in
/// `collect_all_rows_for_item`. Lets the timeline-ordering
/// invariant be unit-tested without touching the store.
fn sort_for_timeline(rows: &mut [(String, ActionRecord)]) {
    rows.sort_by(|a, b| {
        a.1.created_at
            .cmp(&b.1.created_at)
            .then_with(|| a.1.action_id.cmp(&b.1.action_id))
    });
}

#[test]
fn show_timeline_sorts_chronologically_with_action_id_tiebreak() {
    // Out-of-order input. After sort: 10:00 → 10:01 → 10:02
    // (the two 10:01 rows tiebreak on action_id alphabetically).
    let mut rows = vec![
        ("gtm".to_string(), rec("z-1", Some("skates-A"), "post_listing", "approved", "2026-05-06T10:01:00Z")),
        ("gtm".to_string(), rec("a-1", Some("skates-A"), "verify_price", "completed", "2026-05-06T10:00:00Z")),
        ("gtm".to_string(), rec("a-2", Some("skates-A"), "post_listing", "completed", "2026-05-06T10:02:00Z")),
        ("gtm".to_string(), rec("b-1", Some("skates-A"), "post_listing", "pending", "2026-05-06T10:01:00Z")),
    ];
    sort_for_timeline(&mut rows);
    let order: Vec<&str> = rows.iter().map(|(_, r)| r.action_id.as_str()).collect();
    assert_eq!(order, vec!["a-1", "b-1", "z-1", "a-2"]);
}

#[test]
fn show_timeline_preserves_state_transition_history_for_one_action() {
    // The append-only ledger means one action has multiple
    // state rows (pending → approved → executing →
    // completed). `items show` must surface them ALL in
    // order, so the operator can audit transitions. Locks the
    // "no latest-per-action dedup" contract.
    let mut rows = vec![
        ("gtm".to_string(), rec("a-1", Some("skates-A"), "post_listing", "completed", "2026-05-06T10:03:00Z")),
        ("gtm".to_string(), rec("a-1", Some("skates-A"), "post_listing", "executing", "2026-05-06T10:02:00Z")),
        ("gtm".to_string(), rec("a-1", Some("skates-A"), "post_listing", "approved", "2026-05-06T10:01:00Z")),
        ("gtm".to_string(), rec("a-1", Some("skates-A"), "post_listing", "pending", "2026-05-06T10:00:00Z")),
    ];
    sort_for_timeline(&mut rows);
    let states: Vec<&str> = rows.iter().map(|(_, r)| r.state.as_str()).collect();
    assert_eq!(states, vec!["pending", "approved", "executing", "completed"]);
}
