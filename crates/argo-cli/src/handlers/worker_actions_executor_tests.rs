//! Unit tests for `worker_actions_executor` (#731). Pure
//! function tests for the instruction + evidence renderers,
//! plus end-to-end-ish HumanExecutor coverage that writes a
//! tempdir and reads back what landed.
//!
//! Loaded via `#[cfg(test)] #[path = ...] mod tests;` from
//! `worker_actions_executor.rs`.

use super::*;
use crate::handlers::worker_actions::ActionRecord;

fn rec(action_id: &str, action_type: &str, requires_approval: bool) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: action_id.to_string(),
        job_id: "job-7".to_string(),
        item_id: Some("skates-110".to_string()),
        action_type: action_type.to_string(),
        risk_level: "medium".to_string(),
        requires_approval,
        state: "executing".to_string(),
        payload: serde_json::json!({"title": "Pro Skates", "price": 250}),
        approved_by: if requires_approval {
            Some("alice".to_string())
        } else {
            None
        },
        approved_at: if requires_approval {
            Some("2026-05-05T11:00:00Z".to_string())
        } else {
            None
        },
        executed_at: Some("2026-05-05T12:00:00Z".to_string()),
        platform: Some("ebay".to_string()),
        result: None,
        evidence: vec![
            "https://example.com/comp1".to_string(),
            "https://example.com/comp2".to_string(),
        ],
        blocked_reason: None,
        created_at: "2026-05-05T12:00:00Z".to_string(),
    }
}

#[test]
fn render_instructions_includes_action_id_payload_and_next_step_pointer() {
    let r = rec("act-100", "post_listing", true);
    let out = render_instructions(&r);
    assert!(out.contains("# Action: `post_listing`"));
    assert!(out.contains("on `ebay`"));
    assert!(out.contains("(item `skates-110`)"));
    assert!(out.contains("act-100"));
    assert!(out.contains("Approved** by `alice`"));
    assert!(out.contains("\"title\": \"Pro Skates\""));
    assert!(out.contains("argo worker actions complete"));
}

#[test]
fn render_instructions_for_autonomous_action_says_no_approval_required() {
    let r = rec("act-101", "verify_price", false);
    let out = render_instructions(&r);
    assert!(out.contains("Autonomous-safe action"));
    assert!(!out.contains("Approval-gated"));
    assert!(!out.contains("Approved**"));
}

#[test]
fn render_instructions_warns_when_approval_gated_action_lacks_approval_stamp() {
    let mut r = rec("act-102", "post_listing", true);
    r.approved_by = None;
    r.approved_at = None;
    let out = render_instructions(&r);
    assert!(out.contains("⚠️ Approval-gated action"));
    assert!(out.contains("argo worker actions status"));
}

#[test]
fn render_evidence_lists_every_cited_url() {
    let r = rec("act-103", "post_listing", true);
    let out = render_evidence(&r);
    assert!(out.contains("2 source(s) cited"));
    assert!(out.contains("https://example.com/comp1"));
    assert!(out.contains("https://example.com/comp2"));
}

#[test]
fn render_evidence_empty_state_placeholder_when_no_sources() {
    let mut r = rec("act-104", "verify_price", false);
    r.evidence.clear();
    let out = render_evidence(&r);
    assert!(out.contains("(no evidence attached"));
}

#[test]
fn human_executor_writes_per_action_bundle_files_to_disk() {
    let r = rec("act-105", "post_listing", true);
    let cwd = tempfile::tempdir().expect("tempdir");
    let bundle = HumanExecutor.execute(&r, cwd.path()).expect("execute");
    assert_eq!(
        bundle,
        cwd.path()
            .join(".skycot")
            .join("outputs")
            .join("job-7")
            .join("actions")
            .join("act-105")
    );
    let instr = std::fs::read_to_string(bundle.join("instructions.md")).unwrap();
    let evid = std::fs::read_to_string(bundle.join("evidence.md")).unwrap();
    let action_json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(bundle.join("action.json")).unwrap()).unwrap();
    assert!(instr.contains("act-105"));
    assert!(evid.contains("comp1"));
    assert_eq!(action_json["action_type"], "post_listing");
    assert_eq!(action_json["item_id"], "skates-110");
}

#[test]
fn resolve_executor_returns_human_for_default_declaration() {
    let manifest = argo_vertical::parse(
        r#"
schema_version = "1"

[vertical]
id = "fix"
name = "fix"
version = "0.0.1"

[actions.verify_price]
risk = "low"
requires_approval = false
description = "x"
executor = "human"
"#,
    )
    .expect("manifest parses");
    let resolved = resolve_executor(&manifest, "verify_price");
    assert!(resolved.is_ok(), "human executor must resolve");
}

#[test]
fn resolve_executor_rejects_unknown_executor() {
    let manifest = argo_vertical::parse(
        r#"
schema_version = "1"

[vertical]
id = "fix"
name = "fix"
version = "0.0.1"

[actions.post_listing]
risk = "medium"
requires_approval = true
description = "x"
executor = "browser"
"#,
    )
    .expect("manifest parses");
    let err = resolve_executor(&manifest, "post_listing")
        .err()
        .expect("browser should be unimplemented in v1");
    let msg = err.to_string();
    assert!(msg.contains("not yet implemented"), "got: {msg}");
    assert!(msg.contains("browser"), "got: {msg}");
}

#[test]
fn resolve_executor_rejects_undeclared_action_type() {
    let manifest = argo_vertical::parse(
        r#"
schema_version = "1"

[vertical]
id = "fix"
name = "fix"
version = "0.0.1"
"#,
    )
    .expect("manifest parses");
    let err = resolve_executor(&manifest, "verify_price")
        .err()
        .expect("undeclared types must error");
    assert!(err.to_string().contains("no [actions]"));
}
