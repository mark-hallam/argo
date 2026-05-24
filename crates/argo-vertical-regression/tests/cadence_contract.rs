//! Cadence/surface contracts for the static per-vertical regression harness.

use argo_vertical_regression::{check, discover, partition, workspace_root, Violation};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

const MIN_MANIFEST: &str = r#"
schema_version = "1"

[vertical]
id = "synthetic"
name = "Synthetic"
version = "0.1.0"

[role]
system_prompt = "prompts/system.md"

[escalation]
human_approval_required = ["send_thing"]

[actions]
[actions.draft_thing]
risk = "low"
requires_approval = false

[actions.send_thing]
risk = "high"
requires_approval = true
"#;

#[test]
fn lib_rs_public_inventory_stays_explicit() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lib_rs = manifest_dir.join("src/lib.rs");
    let src = fs::read_to_string(&lib_rs).expect("read src/lib.rs");
    let items: Vec<String> = src.lines().filter_map(public_item_label).collect();

    assert_eq!(
        items,
        [
            "struct VerticalCase",
            "struct Fixture",
            "enum Violation",
            "fn discover",
            "fn check",
            "fn partition",
            "fn workspace_root",
        ],
        "argo-vertical-regression public API changed; update the audit report and this pin if the change is intentional"
    );
}

#[test]
fn manual_cadence_runbook_names_the_executable_test_target() {
    let runbook = workspace_root().join("docs/skycot/runbook/per-vertical-regression.md");
    let body = fs::read_to_string(&runbook).expect("read per-vertical regression runbook");

    assert!(
        body.contains("cargo test -p argo-vertical-regression --test regression_all"),
        "manual cadence runbook must name the exact regression_all command"
    );
    assert!(
        body.contains("No LLM calls, no E2B"),
        "runbook must preserve the static/no-network contract"
    );
}

#[test]
fn new_action_vocab_drift_is_reported_as_a_new_violation() {
    let dir = write_case(&[r#"{"id":"f1","expected":{"action_type":"invented_action"}}"#]);
    let cases = discover(dir.path());

    assert_eq!(cases.len(), 1, "role-bearing synthetic vertical discovered");
    let violations = check(&cases[0]);
    let (new_violations, _retired_baseline) = partition(&violations);

    assert_eq!(new_violations.len(), 1, "unbaselined drift must fail");
    assert!(matches!(
        &new_violations[0],
        Violation::UndeclaredActionType { action_type, .. }
            if action_type == "invented_action"
    ));
}

#[test]
fn approval_policy_drift_is_reported_when_fixture_and_manifest_disagree() {
    let dir = write_case(&[
        r#"{"id":"f1","expected":{"action_type":"send_thing","requires_approval":false}}"#,
    ]);
    let cases = discover(dir.path());

    let violations = check(&cases[0]);

    assert!(violations.iter().any(|v| matches!(
        v,
        Violation::ApprovalMismatch {
            action_type,
            expected_approval: false,
            declared_required: true,
            ..
        } if action_type == "send_thing"
    )));
}

fn write_case(fixtures: &[&str]) -> tempfile::TempDir {
    let dir = tempdir().expect("tempdir");
    let vertical_root = dir.path().join("synthetic");
    fs::create_dir_all(vertical_root.join("eval")).expect("create eval dir");
    fs::write(vertical_root.join("vertical.toml"), MIN_MANIFEST).expect("write manifest");
    fs::write(vertical_root.join("eval/seed.jsonl"), fixtures.join("\n")).expect("write fixtures");
    dir
}

fn public_item_label(line: &str) -> Option<String> {
    let rest = line.strip_prefix("pub ")?;
    let (kind, name_source) = rest
        .strip_prefix("struct ")
        .map(|s| ("struct", s))
        .or_else(|| rest.strip_prefix("enum ").map(|s| ("enum", s)))
        .or_else(|| rest.strip_prefix("fn ").map(|s| ("fn", s)))?;
    let name = name_source
        .split(|c: char| c == '<' || c == '(' || c == '{' || c.is_whitespace())
        .next()
        .expect("public item name");
    Some(format!("{kind} {name}"))
}
