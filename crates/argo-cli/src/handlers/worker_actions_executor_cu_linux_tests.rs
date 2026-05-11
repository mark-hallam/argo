//! Unit tests for the native-Linux computer-use dispatch arm.
//!
//! Loaded via `#[cfg(test)] #[path = ...] mod cu_linux_tests;`
//! from `worker_actions_executor_cu_linux.rs`.

#[cfg(not(target_os = "linux"))]
use super::ComputerUseLinuxExecutor;
use crate::handlers::worker_actions::ActionRecord;
#[cfg(target_os = "linux")]
use crate::handlers::worker_actions_executor::ActionOutcome;
use crate::handlers::worker_actions_executor::{resolve_executor, ExecutorKind};

fn rec_for(action_id: &str, action_type: &str) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: action_id.to_string(),
        job_id: "job-cu-linux".to_string(),
        worker_id: argo_identity::WorkerId::nil(),
        item_id: None,
        action_type: action_type.to_string(),
        risk_level: "low".to_string(),
        requires_approval: false,
        state: "executing".to_string(),
        payload: serde_json::json!({"target": "Firefox"}),
        approved_by: None,
        approved_at: None,
        executed_at: None,
        platform: None,
        result: None,
        evidence: vec![],
        blocked_reason: None,
        created_at: "2026-05-11T00:00:00Z".to_string(),
    }
}

fn manifest_with_kind(kind: &str) -> argo_vertical::VerticalManifest {
    let toml = format!(
        r#"
schema_version = "1"

[vertical]
id = "cu-linux-fix"
name = "cu-linux-fix"
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
fn manifest_underscore_form_resolves_to_linux_kind() {
    let manifest = manifest_with_kind("computer_use_linux");
    let exec = resolve_executor(&manifest, "do_thing").expect("resolves");
    assert_eq!(exec.kind(), ExecutorKind::ComputerUseLinux);
}

#[test]
fn manifest_hyphen_alias_resolves_to_linux_kind() {
    let manifest = manifest_with_kind("computer-use-linux");
    let exec = resolve_executor(&manifest, "do_thing").expect("resolves");
    assert_eq!(exec.kind(), ExecutorKind::ComputerUseLinux);
}

#[test]
fn linux_kind_display_string_round_trips() {
    let kind = ExecutorKind::ComputerUseLinux;
    assert_eq!(kind.as_str(), "computer_use_linux");
    let manifest = manifest_with_kind(kind.as_str());
    let exec = resolve_executor(&manifest, "do_thing").expect("resolves");
    assert_eq!(exec.kind(), ExecutorKind::ComputerUseLinux);
}

#[test]
fn unknown_kind_with_linux_in_name_does_not_resolve() {
    // The strict manifest validator (argo-vertical) rejects
    // unknown executor kinds at parse time before the dispatch
    // shim ever sees them. Assert the rejection mentions the
    // bogus kind and the "unknown executor kind" hint string so
    // an operator typo lands a readable error.
    let toml = r#"
schema_version = "1"

[vertical]
id = "cu-linux-fix"
name = "cu-linux-fix"
version = "0.0.1"

[actions.do_thing]
risk = "low"
requires_approval = false
description = "x"
executor = { kind = "computer_use_linux_garbage" }
"#;
    let err = argo_vertical::parse(toml)
        .expect_err("unknown kind must not pass strict validation");
    let msg = err.to_string();
    assert!(
        msg.contains("unknown executor kind"),
        "expected unknown-kind error, got: {msg}"
    );
    assert!(
        msg.contains("computer_use_linux_garbage"),
        "expected bogus kind in error, got: {msg}"
    );
}

#[test]
#[cfg(not(target_os = "linux"))]
fn dispatch_on_non_linux_returns_clean_provider_error() {
    use crate::handlers::worker_actions_executor::Executor;
    let exec = ComputerUseLinuxExecutor::without_declaration();
    let r = rec_for("act-cu-lin-1", "do_thing");
    let cwd = tempfile::tempdir().expect("tempdir");
    let err = exec
        .dispatch(&r, cwd.path())
        .expect_err("non-Linux host must reject dispatch");
    let msg = err.to_string();
    assert!(
        msg.contains("requires Linux"),
        "expected clean Linux-required error, got: {msg}"
    );
    assert!(
        msg.contains("computer_use sandbox"),
        "expected hint pointing at sandbox executor, got: {msg}"
    );
}

#[test]
#[cfg(target_os = "linux")]
fn dispatch_on_linux_writes_bundle() {
    use std::fs;
    let manifest = manifest_with_kind("computer_use_linux");
    let exec = resolve_executor(&manifest, "do_thing").expect("resolves");
    let r = rec_for("act-cu-lin-2", "do_thing");
    let cwd = tempfile::tempdir().expect("tempdir");

    // The dispatch path on Linux runs probe + writes a bundle.
    // probe may report missing xdotool/scrot/DISPLAY on a CI host;
    // that's surfaced into the bundle body, not raised as an
    // error.
    let outcome = exec
        .dispatch(&r, cwd.path())
        .expect("Linux dispatch must succeed even when probe is unsupported");
    let bundle_path = match outcome {
        ActionOutcome::HumanHandoff { bundle_path } => bundle_path,
        ActionOutcome::Completed { .. } => {
            panic!("v1 dispatch lands as HumanHandoff pending follow-up loop")
        }
    };
    let body = fs::read_to_string(bundle_path.join("instructions.md"))
        .expect("instructions.md must exist");
    assert!(body.contains("computer-use-linux"));
    assert!(body.contains(&r.action_id));
}
