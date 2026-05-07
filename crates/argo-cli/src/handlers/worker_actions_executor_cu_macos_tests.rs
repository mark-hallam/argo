//! Unit tests for the native-macOS computer-use dispatch arm.
//!
//! Loaded via `#[cfg(test)] #[path = ...] mod cu_macos_tests;`
//! from `worker_actions_executor_cu_macos.rs`.

#[cfg(not(target_os = "macos"))]
use super::ComputerUseMacOSExecutor;
use crate::handlers::worker_actions::ActionRecord;
#[cfg(target_os = "macos")]
use crate::handlers::worker_actions_executor::ActionOutcome;
use crate::handlers::worker_actions_executor::{resolve_executor, ExecutorKind};

fn rec_for(action_id: &str, action_type: &str) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: action_id.to_string(),
        job_id: "job-cu-macos".to_string(),
        worker_id: argo_identity::WorkerId::nil(),
        item_id: None,
        action_type: action_type.to_string(),
        risk_level: "low".to_string(),
        requires_approval: false,
        state: "executing".to_string(),
        payload: serde_json::json!({"target": "Notes"}),
        approved_by: None,
        approved_at: None,
        executed_at: None,
        platform: None,
        result: None,
        evidence: vec![],
        blocked_reason: None,
        created_at: "2026-05-08T00:00:00Z".to_string(),
    }
}

fn manifest_with_kind(kind: &str) -> argo_vertical::VerticalManifest {
    let toml = format!(
        r#"
schema_version = "1"

[vertical]
id = "cu-macos-fix"
name = "cu-macos-fix"
version = "0.0.1"

[actions.do_thing]
risk = "low"
requires_approval = false
description = "x"
executor = {{ kind = "{kind}" }}
"#
    );
    argo_vertical::parse(&toml).expect("manifest parses")
}

#[test]
fn manifest_underscore_form_resolves_to_macos_kind() {
    let manifest = manifest_with_kind("computer_use_macos");
    let exec = resolve_executor(&manifest, "do_thing").expect("resolves");
    assert_eq!(exec.kind(), ExecutorKind::ComputerUseMacOS);
}

#[test]
fn manifest_hyphen_alias_resolves_to_macos_kind() {
    let manifest = manifest_with_kind("computer-use-macos");
    let exec = resolve_executor(&manifest, "do_thing").expect("resolves");
    assert_eq!(exec.kind(), ExecutorKind::ComputerUseMacOS);
}

#[test]
fn unknown_kind_with_macos_in_name_does_not_resolve() {
    let manifest = manifest_with_kind("computer_use_macos_garbage");
    let result = resolve_executor(&manifest, "do_thing");
    let msg = match result {
        Err(e) => e.to_string(),
        Ok(_) => panic!("unrelated kind must not parse to ComputerUseMacOS"),
    };
    assert!(
        msg.contains("unknown executor kind"),
        "expected unknown-kind error, got: {msg}"
    );
}

#[test]
#[cfg(not(target_os = "macos"))]
fn dispatch_on_non_macos_returns_clean_provider_error() {
    let exec = ComputerUseMacOSExecutor::without_declaration();
    let r = rec_for("act-cu-mac-1", "do_thing");
    let cwd = tempfile::tempdir().expect("tempdir");
    let err = exec
        .dispatch(&r, cwd.path())
        .expect_err("non-macOS host must reject dispatch");
    let msg = err.to_string();
    assert!(
        msg.contains("requires macOS"),
        "expected clean macOS-required error, got: {msg}"
    );
}

#[test]
#[cfg(target_os = "macos")]
fn dispatch_on_macos_writes_bundle() {
    use std::fs;
    let manifest = manifest_with_kind("computer_use_macos");
    let exec = resolve_executor(&manifest, "do_thing").expect("resolves");
    let r = rec_for("act-cu-mac-2", "do_thing");
    let cwd = tempfile::tempdir().expect("tempdir");

    // The dispatch path on macOS reaches the macOS crate's
    // preflight + writes a bundle. preflight may report missing
    // permissions on a CI host; that's surfaced into the bundle
    // body, not raised as an error.
    let outcome = exec
        .dispatch(&r, cwd.path())
        .expect("macOS dispatch must succeed even when perms are missing");
    let bundle_path = match outcome {
        ActionOutcome::HumanHandoff { bundle_path } => bundle_path,
        ActionOutcome::Completed { .. } => {
            panic!("v1 dispatch lands as HumanHandoff pending follow-up loop")
        }
    };
    let body = fs::read_to_string(bundle_path.join("instructions.md"))
        .expect("instructions.md must exist");
    assert!(body.contains("computer-use-macos"));
    assert!(body.contains(&r.action_id));
}
