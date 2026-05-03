//! Integration tests for `argo vertical doctor <id>` (#673).
//!
//! Locks two contracts:
//!
//! 1. The two shipped verticals pass the doctor.
//! 2. A vertical with a synthetic problem (e.g., missing
//!    referenced file) fails loud.

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
    assert!(!out.status.success(), "doctor must surface validate failures");
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

// ---- scored-eval auto-detection (#695) -------------------------

/// Synthetic vertical builder: writes a minimal manifest + a
/// suite + (optionally) an actuals file alongside the suite.
/// Mirrors the `eval/` convention the doctor's scored-eval check
/// auto-detects.
fn write_vertical_with_optional_actuals(
    verticals: &Path,
    id: &str,
    suite_body: &str,
    actuals_body: Option<&str>,
) {
    let v = verticals.join(id);
    std::fs::create_dir_all(v.join("eval")).unwrap();
    std::fs::write(
        v.join("vertical.toml"),
        format!(
            "schema_version = \"1\"\n\
             [vertical]\nid = \"{id}\"\nname = \"S\"\nversion = \"0.1.0\"\n\n\
             [evaluation]\nsuites = [\"eval/cases.jsonl\"]\n",
        ),
    )
    .unwrap();
    std::fs::write(v.join("eval/cases.jsonl"), suite_body).unwrap();
    if let Some(body) = actuals_body {
        std::fs::write(v.join("eval/known_good_actuals.jsonl"), body).unwrap();
    }
}

#[test]
fn cli_doctor_gtm_engineer_reports_scored_eval_passed_ten_of_ten() {
    // GTM Engineer ships verticals/gtm-engineer/eval/known_good_
    // actuals.jsonl from #694. Doctor must auto-detect it and
    // report the scored line as 10/10 pass without any flag.
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", repo_verticals_dir())
        .args(["vertical", "doctor", "gtm-engineer", "--json"])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "gtm-engineer doctor must pass: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    let scored = &v["scored_eval"];
    assert_eq!(scored["status"], "passed", "scored_eval: {scored}");
    assert_eq!(scored["ok"], true);
    let suites = scored["suites"].as_array().unwrap();
    assert_eq!(suites.len(), 1);
    let s = &suites[0];
    assert_eq!(s["actuals_present"], true);
    assert_eq!(s["actuals_path"], "eval/known_good_actuals.jsonl");
    assert_eq!(s["scored_count"], 10);
    assert_eq!(s["passed"], 10);
    assert_eq!(s["failed"], 0);
    assert_eq!(s["pass_rate"].as_f64().unwrap(), 1.0);
}

#[test]
fn cli_doctor_gtm_engineer_text_mode_prints_scored_eval_line() {
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", repo_verticals_dir())
        .args(["vertical", "doctor", "gtm-engineer"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let body = String::from_utf8(out.stdout).unwrap();
    assert!(
        body.contains("[ok] scored eval"),
        "missing scored-eval line; body:\n{body}"
    );
    assert!(
        body.contains("10/10 pass"),
        "scored line should report 10/10; body:\n{body}"
    );
}

#[test]
fn cli_doctor_skips_scored_eval_cleanly_when_no_actuals_file_exists() {
    // Build a synthetic vertical with a suite but NO actuals
    // file alongside. Doctor should report scored_eval status =
    // "skipped" and overall = passed.
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_vertical_with_optional_actuals(
        verticals.path(),
        "no-actuals",
        "{\"id\":\"a\",\"input\":\"\",\"expected\":{}}\n",
        None,
    );
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "doctor", "no-actuals", "--json"])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "missing actuals file must NOT fail doctor: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    let scored = &v["scored_eval"];
    assert_eq!(scored["status"], "skipped");
    let s = &scored["suites"][0];
    assert_eq!(s["actuals_present"], false);
    // Path is reported even when absent, so operators know what
    // filename to create.
    assert_eq!(s["actuals_path"], "eval/known_good_actuals.jsonl");
}

#[test]
fn cli_doctor_skip_text_mode_explains_convention() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_vertical_with_optional_actuals(
        verticals.path(),
        "no-actuals-text",
        "{\"id\":\"a\",\"input\":\"\",\"expected\":{}}\n",
        None,
    );
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "doctor", "no-actuals-text"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let body = String::from_utf8(out.stdout).unwrap();
    assert!(body.contains("[skip] scored eval"), "body: {body}");
    assert!(
        body.contains("known_good_actuals.jsonl"),
        "skip message should name the convention file; body: {body}"
    );
}

#[test]
fn cli_doctor_fails_when_known_good_actuals_contains_failing_actual() {
    // Synthetic vertical: the suite expects must_include "alpha";
    // the actuals file deliberately supplies the wrong response
    // ("beta") so scored eval fails. Doctor's overall must flip
    // to FAILED and exit non-zero.
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_vertical_with_optional_actuals(
        verticals.path(),
        "bad-actuals",
        "{\"id\":\"a\",\"input\":\"\",\"expected\":{\"must_include\":[\"alpha\"]}}\n",
        Some("{\"id\":\"a\",\"actual\":\"only beta here\"}\n"),
    );
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "doctor", "bad-actuals", "--json"])
        .output()
        .expect("spawn");
    assert!(
        !out.status.success(),
        "failing scored fixture must fail doctor"
    );
    // Doctor still emits JSON before exiting non-zero so callers
    // can inspect WHICH check failed.
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], false);
    let scored = &v["scored_eval"];
    assert_eq!(scored["status"], "failed");
    assert_eq!(scored["ok"], false);
    let s = &scored["suites"][0];
    assert_eq!(s["scored_count"], 1);
    assert_eq!(s["passed"], 0);
    assert_eq!(s["failed"], 1);
    assert!(
        s["error"].as_str().unwrap().contains("fixture"),
        "per-suite error should describe failure: {s}"
    );
}

#[test]
fn cli_doctor_fails_clearly_when_actuals_file_is_malformed() {
    // The actuals file has invalid JSON on line 2 — doctor's
    // scored_eval status flips to "failed" with a per-suite
    // error that names the line number so the operator can
    // jump to the fix.
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_vertical_with_optional_actuals(
        verticals.path(),
        "malformed-actuals",
        "{\"id\":\"a\",\"input\":\"\",\"expected\":{}}\n",
        Some("{\"id\":\"a\",\"actual\":\"ok\"}\nNOT JSON\n"),
    );
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "doctor", "malformed-actuals", "--json"])
        .output()
        .expect("spawn");
    assert!(!out.status.success(), "malformed actuals must fail doctor");
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], false);
    let s = &v["scored_eval"]["suites"][0];
    let err = s["error"].as_str().unwrap();
    assert!(err.contains("parse error"), "err: {err}");
    assert!(err.contains("line 2"), "err must name line: {err}");
}

#[test]
fn cli_doctor_no_actual_rows_remain_visible_in_per_suite_stats() {
    // Actuals file covers only one of two fixtures. scored_eval
    // should still pass (1 scored, 1 no_actual, 0 failed) — the
    // no-actual count is a coverage signal, not a regression.
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_vertical_with_optional_actuals(
        verticals.path(),
        "partial-actuals",
        "{\"id\":\"a\",\"input\":\"\",\"expected\":{\"must_include\":[\"alpha\"]}}\n\
         {\"id\":\"b\",\"input\":\"\",\"expected\":{\"must_include\":[\"beta\"]}}\n",
        Some("{\"id\":\"a\",\"actual\":\"alpha is here\"}\n"),
    );
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "doctor", "partial-actuals", "--json"])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "partial coverage must NOT fail doctor: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    let s = &v["scored_eval"]["suites"][0];
    assert_eq!(s["scored_count"], 1);
    assert_eq!(s["no_actual"], 1);
    assert_eq!(s["passed"], 1);
    assert_eq!(s["failed"], 0);
}

#[test]
fn cli_doctor_pr_checker_unchanged_skips_scored_eval() {
    // Regression lock: pr-checker has no known_good_actuals file
    // (#695 didn't ship one). Doctor must continue to pass with
    // overall=true and scored_eval status="skipped". This is the
    // "existing doctor behavior unchanged for verticals without
    // actuals" case the spec called out.
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", repo_verticals_dir())
        .args(["vertical", "doctor", "pr-checker", "--json"])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "pr-checker must continue to pass: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["validate"]["ok"], true);
    assert_eq!(v["eval"]["ok"], true);
    assert_eq!(v["lint"]["ok"], true);
    let scored = &v["scored_eval"];
    assert_eq!(
        scored["status"], "skipped",
        "pr-checker should skip scored_eval until an actuals file ships: {scored}"
    );
}
