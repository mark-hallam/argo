//! Unit tests for `worker_actions_executor` (#731). Pure
//! function tests for the instruction + evidence renderers,
//! plus end-to-end-ish HumanExecutor coverage that writes a
//! tempdir and reads back what landed.
//!
//! Stream A's executor-trait + parsing + stubs coverage lives
//! in the sibling file `worker_actions_executor_stream_a_tests.rs`
//! to keep per-file LOC under the 500-line budget.
//!
//! Loaded via `#[cfg(test)] #[path = ...] mod tests;` from
//! `worker_actions_executor.rs`.

use super::*;
use crate::handlers::worker_actions::ActionRecord;

pub(super) fn rec(action_id: &str, action_type: &str, requires_approval: bool) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: action_id.to_string(),
        job_id: "job-7".to_string(),
        worker_id: argo_identity::WorkerId::nil(),
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

/// Helper: extract bundle path from the HumanHandoff outcome
/// (panics if anything else; HumanExecutor must always return
/// HumanHandoff).
pub(super) fn human_bundle(outcome: ActionOutcome) -> std::path::PathBuf {
    match outcome {
        ActionOutcome::HumanHandoff { bundle_path } => bundle_path,
        ActionOutcome::Completed { .. } => {
            panic!("HumanExecutor must return HumanHandoff, not Completed")
        }
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
    let outcome = HumanExecutor::without_manifest()
        .dispatch(&r, cwd.path())
        .expect("dispatch");
    let bundle = human_bundle(outcome);
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
    let resolved = resolve_executor(&manifest, "verify_price").expect("human resolves");
    assert_eq!(resolved.kind(), ExecutorKind::Human);
}

#[test]
fn resolve_executor_rejects_unknown_executor_kind() {
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
        .map(|_| ())
        .expect_err("'browser' is not a valid executor kind");
    let msg = err.to_string();
    assert!(msg.contains("unknown executor kind"), "got: {msg}");
    assert!(msg.contains("browser"), "got: {msg}");
}

// ───────────────── template renderer (#732) ────────────

#[test]
fn render_with_template_substitutes_action_fields_and_payload_dotted_paths() {
    let mut r = rec("act-200", "post_listing", true);
    r.payload = serde_json::json!({"title": "Pro Skates", "price": 250});
    let body = "title={{ payload.title }} | price=${{ payload.price }} | id={{ action_id }} | platform={{ platform }}";
    let out = render_with_template(&r, body);
    assert_eq!(
        out,
        "title=Pro Skates | price=$250 | id=act-200 | platform=ebay"
    );
}

#[test]
fn render_with_template_renders_unset_for_missing_keys() {
    let mut r = rec("act-201", "verify_price", false);
    r.platform = None;
    r.item_id = None;
    let body = "platform={{ platform }} item={{ item_id }} unknown={{ does_not_exist }}";
    let out = render_with_template(&r, body);
    assert_eq!(out, "platform=(unset) item=(unset) unknown=(unset)");
}

#[test]
fn render_with_template_handles_unmatched_open_braces_safely() {
    let r = rec("act-202", "verify_price", false);
    let body = "no closer here {{ broken — verbatim";
    let out = render_with_template(&r, body);
    assert!(out.contains("no closer here"));
    assert!(out.contains("{{"));
}

#[test]
fn pick_template_prefers_per_platform_override_over_default() {
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
executor = "human"
instruction_template = "default body"

[actions.post_listing.instruction_templates]
ebay = "ebay-specific body"
"#,
    )
    .expect("manifest parses");
    let decl = manifest
        .actions
        .as_ref()
        .and_then(|a| a.actions.get("post_listing"))
        .expect("post_listing");
    assert_eq!(
        pick_template(decl, Some("ebay")),
        Some("ebay-specific body")
    );
    assert_eq!(pick_template(decl, Some("gumtree")), Some("default body"));
    assert_eq!(pick_template(decl, None), Some("default body"));
}

#[test]
fn pick_template_returns_none_when_neither_default_nor_per_platform_set() {
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
    let decl = manifest
        .actions
        .as_ref()
        .and_then(|a| a.actions.get("verify_price"))
        .expect("verify_price");
    assert_eq!(pick_template(decl, Some("ebay")), None);
}

#[test]
fn human_executor_renders_template_when_manifest_provides_one() {
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
executor = "human"
instruction_template = "Title: {{ payload.title }}; Action: {{ action_id }}"
"#,
    )
    .expect("manifest parses");
    let decl = manifest
        .actions
        .as_ref()
        .and_then(|a| a.actions.get("post_listing"))
        .expect("post_listing");
    let executor = HumanExecutor::new(decl.clone());
    let mut r = rec("act-300", "post_listing", true);
    r.payload = serde_json::json!({"title": "Hello"});
    let cwd = tempfile::tempdir().expect("tempdir");
    let outcome = executor.dispatch(&r, cwd.path()).expect("dispatch");
    let bundle = human_bundle(outcome);
    let instr = std::fs::read_to_string(bundle.join("instructions.md")).unwrap();
    assert_eq!(instr, "Title: Hello; Action: act-300");
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
        .map(|_| ())
        .expect_err("undeclared types must error");
    assert!(err.to_string().contains("no [actions]"));
}
