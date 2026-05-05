//! Tests for the action-artifact renderers (#729). Loaded via
//! `#[path = "worker_actions_render_tests.rs"] mod tests;`
//! at the bottom of `worker_actions_render.rs`.

use super::*;
use crate::handlers::worker_actions::ActionRecord;
use serde_json::Value;

fn rec(action_id: &str, action_type: &str, state: &str, requires_approval: bool) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: action_id.to_string(),
        job_id: "job-1".to_string(),
        item_id: None,
        action_type: action_type.to_string(),
        risk_level: "low".to_string(),
        requires_approval,
        state: state.to_string(),
        payload: serde_json::json!({}),
        approved_by: None,
        approved_at: None,
        executed_at: None,
        platform: None,
        result: None,
        evidence: Vec::new(),
        blocked_reason: None,
        created_at: "2026-05-05T00:00:00Z".to_string(),
    }
}

#[test]
fn render_actions_json_empty_state_carries_metadata_with_zero_counts() {
    let out = render_actions_json("job-1", "fix", true, &[]);
    let v: Value = serde_json::from_str(&out).expect("valid JSON");
    assert_eq!(v["schema_version"], "1");
    assert_eq!(v["job_id"], "job-1");
    assert_eq!(v["vertical_id"], "fix");
    assert_eq!(v["evidence_sufficient"], true);
    assert_eq!(v["counts"]["total"], 0);
    assert_eq!(v["counts"]["pending_approval"], 0);
    assert_eq!(v["counts"]["pending_autonomous"], 0);
    assert_eq!(v["counts"]["blocked"], 0);
    assert_eq!(v["actions"].as_array().map(|a| a.len()), Some(0));
}

#[test]
fn render_actions_json_groups_counts_by_state_and_approval_class() {
    let records = vec![
        rec("a-1", "verify_price", "pending", false),
        rec("a-2", "post_listing", "pending", true),
        rec("a-3", "drain_account", "blocked", false),
    ];
    let out = render_actions_json("job-7", "fix", false, &records);
    let v: Value = serde_json::from_str(&out).expect("valid JSON");
    assert_eq!(v["evidence_sufficient"], false);
    assert_eq!(v["counts"]["total"], 3);
    assert_eq!(v["counts"]["pending_autonomous"], 1);
    assert_eq!(v["counts"]["pending_approval"], 1);
    assert_eq!(v["counts"]["blocked"], 1);
    let arr = v["actions"].as_array().expect("actions array");
    assert_eq!(arr.len(), 3);
    // Emission order is preserved.
    assert_eq!(arr[0]["action_id"], "a-1");
    assert_eq!(arr[1]["action_id"], "a-2");
    assert_eq!(arr[2]["action_id"], "a-3");
    // Action records serialize with their full schema.
    assert_eq!(arr[0]["action_type"], "verify_price");
    assert_eq!(arr[1]["requires_approval"], true);
}

#[test]
fn render_actions_json_evidence_sufficient_flag_round_trips() {
    let out = render_actions_json("job-x", "v", false, &[]);
    let v: Value = serde_json::from_str(&out).expect("valid JSON");
    assert_eq!(v["evidence_sufficient"], false);
}
