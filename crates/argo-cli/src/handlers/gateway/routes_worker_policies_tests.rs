//! Unit tests for `routes_worker_policies` — pure helpers only.
//!
//! The full HTTP path (registry lookup, identity-store, audit sink)
//! is exercised by the gateway smoke tests in
//! `crates/argo-cli/tests/`; these tests pin the body-parsing,
//! TOML-roundtrip, and policy-vocabulary contracts so a typo in
//! the wire schema surfaces immediately.

use super::*;
use std::collections::BTreeMap;
use tempfile::tempdir;

fn id() -> WorkerId {
    "00000000-0000-0000-0000-000000000001"
        .parse::<WorkerId>()
        .expect("valid test worker_id")
}

#[test]
fn parse_patch_body_accepts_valid_input() {
    let body = r#"{"policy":"auto-approve","operator_note":"trust ladder hit"}"#;
    let parsed = parse_patch_body(body).expect("valid body parses");
    assert_eq!(parsed.policy, "auto-approve");
    assert_eq!(parsed.operator_note, "trust ladder hit");
}

#[test]
fn parse_patch_body_rejects_unknown_policy() {
    let body = r#"{"policy":"please","operator_note":"hi"}"#;
    let err = parse_patch_body(body).expect_err("unknown policy rejects");
    assert!(err.contains("unknown policy"), "got: {err}");
    assert!(
        err.contains("auto-approve") && err.contains("risk-gated"),
        "error should enumerate the canonical vocabulary: {err}"
    );
}

#[test]
fn parse_patch_body_requires_non_empty_operator_note() {
    let missing = r#"{"policy":"always-ask"}"#;
    let e1 = parse_patch_body(missing).expect_err("missing note rejects");
    assert!(e1.contains("operator_note"), "got: {e1}");

    let blank = r#"{"policy":"always-ask","operator_note":"   "}"#;
    let e2 = parse_patch_body(blank).expect_err("whitespace-only note rejects");
    assert!(e2.contains("non-empty"), "got: {e2}");
}

#[test]
fn read_policies_returns_empty_when_file_absent() {
    let dir = tempdir().expect("tempdir");
    let overrides = read_policies(dir.path(), id()).expect("missing file is not an error");
    assert!(overrides.is_empty());
}

#[test]
fn write_then_read_round_trips_the_policy_table() {
    let dir = tempdir().expect("tempdir");
    let worker = id();
    let mut overrides = BTreeMap::new();
    overrides.insert("draft_listing".to_string(), "auto-approve".to_string());
    overrides.insert("post_listing".to_string(), "always-ask".to_string());
    write_policies(dir.path(), worker, &overrides).expect("write");

    let path = worker_policy_path(dir.path(), worker);
    assert!(path.exists(), "file written at {}", path.display());

    let round = read_policies(dir.path(), worker).expect("read");
    assert_eq!(
        round.get("draft_listing"),
        Some(&"auto-approve".to_string())
    );
    assert_eq!(round.get("post_listing"), Some(&"always-ask".to_string()));
    assert_eq!(round.len(), 2);
}

#[test]
fn read_policies_filters_out_unknown_policy_values() {
    // Operator-edited TOML on disk may carry stale or hand-written
    // values. The reader keeps the valid arms and drops the rest so
    // the UI never surfaces a value it can't render.
    let dir = tempdir().expect("tempdir");
    let worker = id();
    let path = worker_policy_path(dir.path(), worker);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        "[policies]\ngood = \"auto-approve\"\nbad = \"yolo\"\n",
    )
    .unwrap();
    let overrides = read_policies(dir.path(), worker).expect("read");
    assert_eq!(overrides.get("good"), Some(&"auto-approve".to_string()));
    assert!(!overrides.contains_key("bad"), "unknown value dropped");
}
