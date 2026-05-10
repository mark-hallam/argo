//! Integration tests for `argo vertical doctor <id>` (#673).
//!
//! Locks two contracts:
//!
//! 1. The two shipped verticals pass the doctor.
//! 2. A vertical with a synthetic problem (e.g., missing
//!    referenced file) fails loud.
//!
//! Scored-eval auto-detection cases (#695) live in the sibling
//! `vertical_doctor_scored_eval.rs` to keep this file under the
//! 500-LOC ceiling.

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
fn cli_doctor_pr_checker_passes() {
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", repo_verticals_dir())
        .args(["vertical", "doctor", "pr-checker", "--json"])
        .output()
        .expect("spawn doctor");
    assert!(
        out.status.success(),
        "pr-checker doctor must pass: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["validate"]["ok"], true);
    assert_eq!(v["eval"]["ok"], true);
    assert_eq!(v["lint"]["ok"], true);
    assert_eq!(v["lint"]["error_count"].as_u64().unwrap(), 0);
}

#[test]
fn cli_doctor_receptionist_passes() {
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", repo_verticals_dir())
        .args(["vertical", "doctor", "receptionist", "--json"])
        .output()
        .expect("spawn doctor");
    assert!(out.status.success());
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
}

#[test]
fn cli_doctor_unknown_vertical_fails_loudly() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "doctor", "ghost"])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("ghost"), "stderr: {stderr}");
    assert!(stderr.contains("not found"), "stderr: {stderr}");
}

#[test]
fn cli_doctor_vertical_with_missing_role_file_fails_with_validate_error() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let v = verticals.path().join("broken");
    std::fs::create_dir_all(&v).unwrap();
    // [role] points at a file that doesn't exist → validate fails.
    std::fs::write(
        v.join("vertical.toml"),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"broken\"\nname = \"B\"\nversion = \"0.1.0\"\n\
         [role]\nsystem_prompt = \"prompts/missing.md\"\n",
    )
    .unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "doctor", "broken", "--json"])
        .output()
        .expect("spawn");
    // Vertical loader rejects the manifest at load_dir time
    // (since validate runs there too), so the doctor command
    // bails with a load-side error before producing JSON.
    // Either way it must exit non-zero.
    assert!(
        !out.status.success(),
        "doctor must surface validate failures"
    );
}

#[test]
fn cli_doctor_text_mode_prints_overall_passed_or_failed() {
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", repo_verticals_dir())
        .args(["vertical", "doctor", "pr-checker"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let body = String::from_utf8(out.stdout).unwrap();
    assert!(body.contains("vertical 'pr-checker'"), "body: {body}");
    assert!(body.contains("[ok] validate"), "body: {body}");
    assert!(body.contains("[ok] eval"), "body: {body}");
    assert!(body.contains("[ok] lint"), "body: {body}");
    assert!(body.contains("overall: PASSED"), "body: {body}");
}
