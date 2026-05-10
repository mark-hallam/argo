//! #769 sold-item emission gate tests, split out of
//! `worker_actions_emit_tests.rs` to keep that file under the
//! 500-LOC budget. Loaded as a child mod via `#[path]` from the
//! parent test file.

use super::super::*;
use super::fixture_vertical;
use std::collections::HashSet;

/// Build a sold-set fixture for the gate tests.
fn sold(items: &[&str]) -> HashSet<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

/// Headline #769 contract: an emitted approval-gated action whose
/// `item_id` is in the sold set must land `blocked`, not
/// `pending`. The worker should not draft new listings for items
/// the operator has already closed.
#[test]
fn emit_orchestrator_blocks_approval_action_for_sold_item() {
    let v = fixture_vertical();
    let text = "```action\n\
        {\"action_type\": \"post_listing\", \"payload\": {\"title\":\"x\"}, \"item_id\": \"skates-A\"}\n\
        ```";
    let records = emit_actions_from_text(&v, "job-1", argo_identity::WorkerId::nil(), text, true, &sold(&["skates-A"]));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].state, "blocked");
    let reason = records[0].blocked_reason.as_deref().unwrap_or("");
    assert!(
        reason.contains("already marked sold") && reason.contains("skates-A"),
        "blocked_reason should name the sold item; got: {reason}"
    );
}

/// Symmetric to the evidence gate: the sold-item gate also
/// blocks autonomous-safe actions for sold items. A
/// long-running worker should not even verify_price against an
/// item that's already been sold — there's no inventory to
/// price-check.
#[test]
fn emit_orchestrator_blocks_autonomous_action_for_sold_item() {
    let v = fixture_vertical();
    let text = "```action\n\
        {\"action_type\": \"verify_price\", \"payload\": {\"target\": 250}, \"item_id\": \"skates-A\"}\n\
        ```";
    let records = emit_actions_from_text(&v, "job-1", argo_identity::WorkerId::nil(), text, true, &sold(&["skates-A"]));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].state, "blocked");
    assert!(records[0]
        .blocked_reason
        .as_deref()
        .unwrap_or("")
        .contains("already marked sold"));
}

/// Idempotency: if the worker re-emits a `mark_item_sold` for an
/// already-sold item, the gate must NOT block it (so a redundant
/// confirmation still flows through the normal pipeline).
#[test]
fn emit_orchestrator_does_not_block_redundant_mark_item_sold() {
    let v = argo_vertical::parse(
        r#"
schema_version = "1"
[vertical]
id = "fix"
name = "fixture"
version = "0.0.1"
[actions.mark_item_sold]
risk = "medium"
requires_approval = true
description = "Close the inventory item."
executor = "human"
"#,
    )
    .expect("manifest parses");
    let text = "```action\n\
        {\"action_type\": \"mark_item_sold\", \"payload\": {}, \"item_id\": \"skates-A\"}\n\
        ```";
    let records = emit_actions_from_text(&v, "job-1", argo_identity::WorkerId::nil(), text, true, &sold(&["skates-A"]));
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].state, "pending",
        "mark_item_sold must pass through the gate even when the item is already sold (idempotent)"
    );
}

/// Item-less actions are unaffected by the gate. Verifies the
/// gate only triggers on `Some(item_id)`.
#[test]
fn emit_orchestrator_does_not_block_item_less_action_for_sold_set() {
    let v = fixture_vertical();
    let text = "```action\n\
        {\"action_type\": \"verify_price\", \"payload\": {\"target\": 250}}\n\
        ```";
    // Same sold set, but THIS action has no item_id — must not
    // be gated.
    let records = emit_actions_from_text(&v, "job-1", argo_identity::WorkerId::nil(), text, true, &sold(&["skates-A"]));
    assert_eq!(records[0].state, "pending");
    assert!(records[0].blocked_reason.is_none());
}

/// Mixed emission in one assistant text: one action for a sold
/// item (blocks), one for an unsold item (passes). Ensures the
/// gate is per-action, not per-batch.
#[test]
fn emit_orchestrator_per_action_sold_gate_decision() {
    let v = fixture_vertical();
    let text = "\
```action
{\"action_type\": \"verify_price\", \"payload\": {\"target\": 250}, \"item_id\": \"skates-A\"}
```

```action
{\"action_type\": \"verify_price\", \"payload\": {\"target\": 175}, \"item_id\": \"skates-B\"}
```
";
    let records = emit_actions_from_text(&v, "job-1", argo_identity::WorkerId::nil(), text, true, &sold(&["skates-A"]));
    assert_eq!(records.len(), 2);
    let by_item: std::collections::HashMap<String, &ActionRecord> = records
        .iter()
        .map(|r| (r.item_id.clone().unwrap_or_default(), r))
        .collect();
    assert_eq!(by_item["skates-A"].state, "blocked");
    assert_eq!(by_item["skates-B"].state, "pending");
}
