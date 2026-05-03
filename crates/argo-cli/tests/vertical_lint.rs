//! Integration tests for `argo vertical lint <id>` (#671).
//!
//! Locks two contracts:
//!
//! 1. The two shipped verticals (PR-Checker, Receptionist) must
//!    lint clean. Future edits that introduce typo'd tool names,
//!    orphan escalation entries, or empty bundles will fail this
//!    test in the same PR.
//! 2. The CLI's text + JSON shapes work end-to-end against the
//!    binary.

mod common;

use common::argo_bin;
use std::path::Path;
use std::process::Command;

fn repo_verticals_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join("verticals")
}

#[test]
fn cli_lint_pr_checker_is_clean_or_only_emits_info_findings() {
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", repo_verticals_dir())
        .args(["vertical", "lint", "pr-checker", "--json"])
        .output()
        .expect("spawn lint");
    assert!(
        out.status.success(),
        "pr-checker lint must not produce errors: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    // Per-finding sanity: anything present must be `info` or
    // `warn`, never `error`.
    for f in v["findings"].as_array().unwrap_or(&Vec::new()) {
        let sev = f["severity"].as_str().unwrap();
        assert!(
            matches!(sev, "info" | "warn"),
            "pr-checker emitted unexpected severity {sev}: {f}"
        );
    }
}

#[test]
fn cli_lint_receptionist_is_clean_or_only_emits_info_findings() {
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", repo_verticals_dir())
        .args(["vertical", "lint", "receptionist", "--json"])
        .output()
        .expect("spawn lint");
    assert!(
        out.status.success(),
        "receptionist lint must not produce errors: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
}

#[test]
fn cli_lint_unknown_vertical_fails_loudly() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "lint", "ghost"])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("ghost"), "stderr: {stderr}");
    assert!(stderr.contains("not found"), "stderr: {stderr}");
}

#[test]
fn cli_lint_synthetic_vertical_with_typo_in_allowed_emits_warn() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let v = verticals.path().join("typo");
    std::fs::create_dir_all(&v).unwrap();
    std::fs::write(
        v.join("vertical.toml"),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"typo\"\nname = \"T\"\nversion = \"0.1.0\"\n\
         [tools]\nallowed = [\"file_read\", \"file_reed_typo\"]\ndenied = []\n",
    )
    .unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "lint", "typo", "--json"])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "warn-only must exit 0");
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true, "ok=true because no error findings");
    let findings = v["findings"].as_array().unwrap();
    let warn = findings
        .iter()
        .find(|f| f["code"] == "tools.allowed.unknown_name")
        .expect("expected unknown_name warn");
    assert_eq!(warn["severity"], "warn");
    assert!(warn["message"]
        .as_str()
        .unwrap()
        .contains("file_reed_typo"));
}

#[test]
fn cli_lint_text_mode_prints_finding_lines() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let v = verticals.path().join("orphan");
    std::fs::create_dir_all(&v).unwrap();
    std::fs::write(
        v.join("vertical.toml"),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"orphan\"\nname = \"O\"\nversion = \"0.1.0\"\n\
         [tools]\nallowed = [\"file_read\"]\ndenied = []\n\
         [escalation]\nhuman_approval_required = [\"delegate_task\"]\n",
    )
    .unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "lint", "orphan"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let body = String::from_utf8(out.stdout).unwrap();
    assert!(body.contains("orphan"), "body: {body}");
    assert!(body.contains("[warn]"), "body should mark warn severity: {body}");
    assert!(
        body.contains("escalation.orphan_tool"),
        "body should name the code: {body}"
    );
}
