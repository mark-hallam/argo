//! Unit tests for the native-Windows computer-use dispatch arm.
//!
//! Loaded via `#[cfg(test)] #[path = ...] mod cu_windows_tests;`
//! from `worker_actions_executor_cu_windows.rs`.

#[cfg(not(target_os = "windows"))]
use super::ComputerUseWindowsExecutor;
use crate::handlers::worker_actions::ActionRecord;
#[cfg(target_os = "windows")]
use crate::handlers::worker_actions_executor::ActionOutcome;
use crate::handlers::worker_actions_executor::{resolve_executor, ExecutorKind};

fn rec_for(action_id: &str, action_type: &str) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: action_id.to_string(),
        job_id: "job-cu-windows".to_string(),
        worker_id: argo_identity::WorkerId::nil(),
        item_id: None,
        action_type: action_type.to_string(),
        risk_level: "low".to_string(),
        requires_approval: false,
        state: "executing".to_string(),
        payload: serde_json::json!({"target": "notepad.exe"}),
        approved_by: None,
        approved_at: None,
        executed_at: None,
        platform: None,
        result: None,
        evidence: vec![],
        blocked_reason: None,
        created_at: "2026-05-12T00:00:00Z".to_string(),
    }
}

fn manifest_with_kind(kind: &str) -> argo_vertical::VerticalManifest {
    let toml = format!(
        r#"
schema_version = "1"

[vertical]
id = "cu-windows-fix"
name = "cu-windows-fix"
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
fn manifest_underscore_form_resolves_to_windows_kind() {
    let manifest = manifest_with_kind("computer_use_windows");
    let exec = resolve_executor(&manifest, "do_thing").expect("resolves");
    assert_eq!(exec.kind(), ExecutorKind::ComputerUseWindows);
}

#[test]
fn manifest_hyphen_alias_resolves_to_windows_kind() {
    let manifest = manifest_with_kind("computer-use-windows");
    let exec = resolve_executor(&manifest, "do_thing").expect("resolves");
    assert_eq!(exec.kind(), ExecutorKind::ComputerUseWindows);
}

#[test]
fn windows_kind_display_string_round_trips() {
    let kind = ExecutorKind::ComputerUseWindows;
    assert_eq!(kind.as_str(), "computer_use_windows");
    let manifest = manifest_with_kind(kind.as_str());
    let exec = resolve_executor(&manifest, "do_thing").expect("resolves");
    assert_eq!(exec.kind(), ExecutorKind::ComputerUseWindows);
}

#[test]
fn unknown_kind_with_windows_in_name_does_not_resolve() {
    // The strict manifest validator (argo-vertical) rejects
    // unknown executor kinds at parse time before the dispatch
    // shim ever sees them. Assert the rejection mentions the
    // bogus kind and the "unknown executor kind" hint string so
    // an operator typo lands a readable error.
    let toml = r#"
schema_version = "1"

[vertical]
id = "cu-windows-fix"
name = "cu-windows-fix"
version = "0.0.1"

[actions.do_thing]
risk = "low"
requires_approval = false
description = "x"
executor = { kind = "computer_use_windows_garbage" }
"#;
    let err = argo_vertical::parse(toml)
        .expect_err("unknown kind must not pass strict validation");
    let msg = err.to_string();
    assert!(
        msg.contains("unknown executor kind"),
        "expected unknown-kind error, got: {msg}"
    );
    assert!(
        msg.contains("computer_use_windows_garbage"),
        "expected bogus kind in error, got: {msg}"
    );
}

#[test]
#[cfg(not(target_os = "windows"))]
fn dispatch_on_non_windows_returns_clean_provider_error() {
    use crate::handlers::worker_actions_executor::Executor;
    let exec = ComputerUseWindowsExecutor::without_declaration();
    let r = rec_for("act-cu-win-1", "do_thing");
    let cwd = tempfile::tempdir().expect("tempdir");
    let err = exec
        .dispatch(&r, cwd.path())
        .expect_err("non-Windows host must reject dispatch");
    let msg = err.to_string();
    assert!(
        msg.contains("requires Windows"),
        "expected clean Windows-required error, got: {msg}"
    );
    assert!(
        msg.contains("computer_use sandbox"),
        "expected hint pointing at sandbox executor, got: {msg}"
    );
}

#[test]
#[cfg(target_os = "windows")]
fn dispatch_on_windows_writes_bundle() {
    use crate::handlers::worker_actions_executor::Executor;
    use std::fs;
    let manifest = manifest_with_kind("computer_use_windows");
    let exec = resolve_executor(&manifest, "do_thing").expect("resolves");
    let r = rec_for("act-cu-win-2", "do_thing");
    let cwd = tempfile::tempdir().expect("tempdir");

    // The dispatch path on Windows runs preflight + writes a
    // bundle. Preflight may report missing UAC / DPI / a11y bits
    // on a CI host; that's surfaced into the bundle body, not
    // raised as an error.
    let outcome = exec
        .dispatch(&r, cwd.path())
        .expect("Windows dispatch must succeed even when preflight is failing");
    let bundle_path = match outcome {
        ActionOutcome::HumanHandoff { bundle_path } => bundle_path,
        ActionOutcome::Completed { .. } => {
            panic!("v1 dispatch lands as HumanHandoff pending follow-up loop")
        }
    };
    let body = fs::read_to_string(bundle_path.join("instructions.md"))
        .expect("instructions.md must exist");
    assert!(body.contains("computer-use-windows"));
    assert!(body.contains(&r.action_id));
}
