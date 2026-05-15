#[cfg(not(target_os = "macos"))]
use super::ComputerUseMacOSExecutor;
use crate::handlers::worker_actions::ActionRecord;
use crate::handlers::worker_actions_executor::{resolve_executor, ActionOutcome, ExecutorKind};

fn rec_for(action_id: &str, action_type: &str) -> ActionRecord {
    serde_json::from_value(serde_json::json!({
        "schema_key": ActionRecord::SCHEMA_KEY,
        "action_id": action_id,
        "job_id": "job-cu-macos",
        "action_type": action_type,
        "state": "executing",
        "payload": {"target": "Notes"},
        "created_at": "2026-05-08T00:00:00Z"
    }))
    .expect("valid action record")
}

fn manifest_src(kind: &str) -> String {
    format!(
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
executor = {{ kind = "{kind}", profile = "com.apple.Notes" }}
"#
    )
}

fn manifest_with_kind(kind: &str) -> argo_vertical::VerticalManifest {
    argo_vertical::parse(&manifest_src(kind)).expect("manifest parses")
}

#[test]
fn manifest_aliases_resolve_to_macos_kind() {
    for kind in ["computer_use_macos", "computer-use-macos"] {
        let manifest = manifest_with_kind(kind);
        let exec = resolve_executor(&manifest, "do_thing").expect("resolves");
        assert_eq!(exec.kind(), ExecutorKind::ComputerUseMacOS);
    }
}

#[test]
fn unknown_kind_with_macos_in_name_does_not_resolve() {
    let result = argo_vertical::parse(&manifest_src("computer_use_macos_garbage"));
    let msg = match result {
        Err(e) => e.to_string(),
        Ok(_) => panic!("unrelated kind must not pass strict manifest validation"),
    };
    assert!(
        msg.contains("unknown executor kind") && msg.contains("computer_use_macos_garbage"),
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
fn maps_completed_and_handoff_outcomes() {
    let r = rec_for("act-cu-mac-complete", "do_thing");
    let cwd = tempfile::tempdir().expect("tempdir");
    let outcome = argo_executor_computer_use_macos::MacosCuOutcome::Completed {
        summary: "typed note".into(),
        steps: Vec::new(),
        wallclock_seconds: 2,
    };

    let mapped = super::map_macos_outcome(&r, cwd.path(), None, outcome).expect("maps");
    let ActionOutcome::Completed {
        bundle_path,
        result_json,
    } = mapped
    else {
        panic!("expected Completed");
    };
    assert!(bundle_path.ends_with(".skycot/outputs/job-cu-macos/actions/act-cu-mac-complete"));
    assert_eq!(result_json["outcome"], "completed");
    assert_eq!(result_json["summary"], "typed note");
    assert_eq!(result_json["wallclock_seconds"], 2);

    let r = rec_for("act-cu-mac-handoff", "do_thing");
    let outcome = argo_executor_computer_use_macos::MacosCuOutcome::HumanHandoff {
        reason: "Screen Recording missing".into(),
        steps: Vec::new(),
    };

    let mapped = super::map_macos_outcome(&r, cwd.path(), None, outcome).expect("maps");
    let ActionOutcome::HumanHandoff { bundle_path } = mapped else {
        panic!("expected HumanHandoff");
    };
    let body = std::fs::read_to_string(bundle_path.join("instructions.md"))
        .expect("instructions.md must exist");
    assert!(body.contains("Screen Recording missing"));
    assert!(body.contains(&r.action_id));
}

#[test]
#[cfg(target_os = "macos")]
fn dispatch_on_macos_preserves_handoff_when_preflight_missing() {
    use argo_executor_computer_use_macos::{MacosCuConfig, MacosCuExecutor};
    use std::fs;

    let exec_for_probe = MacosCuExecutor::new(MacosCuConfig::for_notes_app()).expect("config");
    if super::run_preflight_summary(&exec_for_probe)
        .expect("preflight")
        .is_none()
    {
        return;
    }

    let manifest = manifest_with_kind("computer_use_macos");
    let exec = resolve_executor(&manifest, "do_thing").expect("resolves");
    let r = rec_for("act-cu-mac-2", "do_thing");
    let cwd = tempfile::tempdir().expect("tempdir");

    // Permission gaps are legitimate handoff triggers; a fully
    // permitted host exits early to avoid real Anthropic traffic.
    let outcome = exec
        .dispatch(&r, cwd.path())
        .expect("macOS preflight handoff must succeed");
    let bundle_path = match outcome {
        ActionOutcome::HumanHandoff { bundle_path } => bundle_path,
        ActionOutcome::Completed { .. } => {
            panic!("preflight-missing host should hand off")
        }
    };
    let body = fs::read_to_string(bundle_path.join("instructions.md"))
        .expect("instructions.md must exist");
    assert!(body.contains("computer-use-macos"));
    assert!(body.contains(&r.action_id));
}
