use super::*;

fn action(state: &str, requires_approval: bool) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: "act-1".to_string(),
        job_id: "job-1".to_string(),
        worker_id: argo_identity::WorkerId::nil(),
        item_id: None,
        action_type: "draft_reply".to_string(),
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
        created_at: "2026-05-08T00:00:00Z".to_string(),
    }
}

#[test]
fn pending_autonomous_action_can_be_previewed() {
    let rec = action("pending", false);
    validate_would_execute(&rec).expect("autonomous pending action should preview");
}

#[test]
fn pending_approval_gated_action_requires_approval() {
    let rec = action("pending", true);
    let err = validate_would_execute(&rec).expect_err("approval-gated pending action");
    assert!(
        err.to_string().contains("approval required"),
        "unexpected error: {err}"
    );
}

#[test]
fn terminal_action_cannot_be_previewed() {
    let rec = action("completed", false);
    let err = validate_would_execute(&rec).expect_err("completed action");
    assert!(
        err.to_string().contains("illegal transition"),
        "unexpected error: {err}"
    );
}
