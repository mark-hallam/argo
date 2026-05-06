//! Unit tests for `worker_actions_followup` (#771). Sibling of
//! the impl module so the impl file stays under the per-file
//! 500-LOC budget.

use super::*;
use crate::handlers::worker_actions::ActionRecord;
use serde_json::json;
use std::collections::HashSet;

fn make_source_record() -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: "src-1".to_string(),
        job_id: "job-1".to_string(),
        item_id: Some("skates-A".to_string()),
        action_type: "post_listing".to_string(),
        risk_level: "medium".to_string(),
        requires_approval: true,
        state: "completed".to_string(),
        payload: json!({}),
        approved_by: Some("op".to_string()),
        approved_at: Some("2026-05-06T10:00:00Z".to_string()),
        executed_at: Some("2026-05-06T10:01:00Z".to_string()),
        platform: Some("ebay".to_string()),
        result: Some(json!({"summary": "posted"})),
        evidence: vec![],
        blocked_reason: None,
        created_at: "2026-05-06T10:02:00Z".to_string(),
    }
}

fn fixture_manifest() -> argo_vertical::VerticalManifest {
    argo_vertical::parse(
        r#"
schema_version = "1"

[vertical]
id      = "fix"
name    = "fixture"
version = "0.0.1"

[actions.post_listing]
risk              = "medium"
requires_approval = true
description       = "Post a listing."
executor          = "human"
on_complete_emit  = ["boost_listing", "verify_price"]

[actions.boost_listing]
risk              = "medium"
requires_approval = true
description       = "Boost a listing."
executor          = "human"

[actions.verify_price]
risk              = "low"
requires_approval = false
description       = "Cross-check price."
executor          = "human"
"#,
    )
    .expect("manifest parses")
}

#[test]
fn build_followup_inherits_item_and_platform_and_references_source() {
    let manifest = fixture_manifest();
    let source = LocatedAction {
        record: make_source_record(),
        vertical_id: "fix".to_string(),
    };
    let rec = build_followup_record(&manifest, &source, "boost_listing");
    assert_eq!(rec.action_type, "boost_listing");
    assert_eq!(rec.item_id.as_deref(), Some("skates-A"));
    assert_eq!(rec.platform.as_deref(), Some("ebay"));
    // The follow-up's evidence chain references the source via
    // the canonical argo://action/<id> URL so the operator can
    // trace the lineage in `argo worker actions show`.
    assert_eq!(
        rec.evidence,
        vec!["argo://action/src-1".to_string()],
        "follow-up must reference the source action in evidence"
    );
    // Manifest classification carries through — boost_listing is
    // approval-gated, so the follow-up lands `pending` awaiting
    // operator review.
    assert_eq!(rec.state, "pending");
    assert!(rec.requires_approval);
    assert_eq!(rec.risk_level, "medium");
}

#[test]
fn build_followup_for_autonomous_safe_type_lands_pending_no_approval() {
    let manifest = fixture_manifest();
    let source = LocatedAction {
        record: make_source_record(),
        vertical_id: "fix".to_string(),
    };
    let rec = build_followup_record(&manifest, &source, "verify_price");
    assert_eq!(rec.action_type, "verify_price");
    assert!(!rec.requires_approval);
    assert_eq!(rec.state, "pending");
    assert_eq!(rec.risk_level, "low");
}

#[test]
fn apply_sold_gate_blocks_follow_up_for_sold_item() {
    let manifest = fixture_manifest();
    let source = LocatedAction {
        record: make_source_record(),
        vertical_id: "fix".to_string(),
    };
    let mut rec = build_followup_record(&manifest, &source, "boost_listing");
    let mut sold = HashSet::new();
    sold.insert("skates-A".to_string());
    apply_sold_gate(&mut rec, &sold);
    assert_eq!(rec.state, "blocked");
    let reason = rec.blocked_reason.as_deref().unwrap_or("");
    assert!(
        reason.contains("already marked sold") && reason.contains("skates-A"),
        "blocked_reason should name the sold item; got: {reason}"
    );
}

#[test]
fn apply_sold_gate_does_not_block_mark_item_sold_followup() {
    // mark_item_sold is exempt from the sold gate even when
    // emitted as a follow-up. A redundant mark-sold for an
    // already-sold item is idempotent.
    let manifest = argo_vertical::parse(
        r#"
schema_version = "1"
[vertical]
id = "fix"
name = "fixture"
version = "0.0.1"
[actions.mark_item_sold]
risk = "medium"
requires_approval = true
description = "Mark sold."
executor = "human"
"#,
    )
    .expect("manifest parses");
    let source = LocatedAction {
        record: make_source_record(),
        vertical_id: "fix".to_string(),
    };
    let mut rec = build_followup_record(&manifest, &source, "mark_item_sold");
    let mut sold = HashSet::new();
    sold.insert("skates-A".to_string());
    apply_sold_gate(&mut rec, &sold);
    assert_eq!(rec.state, "pending", "mark_item_sold must pass through the sold gate");
}

#[test]
fn apply_sold_gate_passes_through_when_item_not_sold() {
    let manifest = fixture_manifest();
    let source = LocatedAction {
        record: make_source_record(),
        vertical_id: "fix".to_string(),
    };
    let mut rec = build_followup_record(&manifest, &source, "boost_listing");
    let sold = HashSet::new();
    apply_sold_gate(&mut rec, &sold);
    assert_eq!(rec.state, "pending");
    assert!(rec.blocked_reason.is_none());
}

#[test]
fn apply_sold_gate_does_not_re_block_already_blocked_record() {
    // build_emission may already mark a record as blocked
    // (e.g. undeclared action_type). The sold gate should leave
    // it alone — don't overwrite the original blocked_reason.
    let manifest = fixture_manifest();
    let source = LocatedAction {
        record: make_source_record(),
        vertical_id: "fix".to_string(),
    };
    let mut rec = build_followup_record(&manifest, &source, "boost_listing");
    rec.state = "blocked".to_string();
    rec.blocked_reason = Some("undeclared action type".to_string());
    let mut sold = HashSet::new();
    sold.insert("skates-A".to_string());
    apply_sold_gate(&mut rec, &sold);
    assert_eq!(
        rec.blocked_reason.as_deref(),
        Some("undeclared action type"),
        "sold gate must not overwrite an existing blocked_reason"
    );
}
