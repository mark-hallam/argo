//! Scored-eval auto-detection tests for `argo vertical doctor`
//! (#695). Split out from `vertical_doctor.rs`; basic doctor
//! contracts (#673) live in the sibling file.

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

/// Writes a manifest + suite + optional actuals file matching the doctor's auto-detect convention.
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
    // #694 ships actuals for the core suite; #759 adds a
    // worker-v3-emission suite without bundled actuals. Doctor
    // reports core 10/10 pass and emission scored_count=0 with
    // overall status still passed.
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
    assert_eq!(suites.len(), 2, "expected core + worker_v3_emission suites");
    // Lookup by path is more robust than manifest-declared order.
    let core = suites
        .iter()
        .find(|s| s["path"] == "eval/gtm_engineer_core.jsonl")
        .expect("core suite present");
    assert_eq!(core["actuals_present"], true);
    assert_eq!(core["actuals_path"], "eval/known_good_actuals.jsonl");
    assert_eq!(core["scored_count"], 10);
    assert_eq!(core["passed"], 10);
    assert_eq!(core["failed"], 0);
    assert_eq!(core["pass_rate"].as_f64().unwrap(), 1.0);
    let emission = suites
        .iter()
        .find(|s| s["path"] == "eval/worker_v3_emission.jsonl")
        .expect("worker_v3_emission suite present");
    // Shared actuals file exists → actuals_present=true, but
    // no IDs match this suite → scored_count and pass/fail = 0.
    assert_eq!(emission["actuals_present"], true);
    assert_eq!(emission["passed"], 0);
    assert_eq!(emission["failed"], 0);
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
    assert!(body.contains("[ok] scored eval"), "body:\n{body}");
    assert!(body.contains("10/10 pass"), "body:\n{body}");
}

#[test]
fn cli_doctor_skips_scored_eval_cleanly_when_no_actuals_file_exists() {
    // Suite present, no actuals → scored_eval status="skipped",
    // overall=passed.
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
    // Path reported even when absent so operators see the
    // expected filename.
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
    // Suite expects "alpha"; actuals supplies "beta" → scored
    // eval fails, overall flips to FAILED, exit non-zero.
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
    // JSON still emitted before non-zero exit so callers can
    // inspect WHICH check failed.
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
    // Invalid JSON on line 2 → scored_eval="failed" with
    // per-suite error naming the line number.
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
    // Actuals cover 1 of 2 fixtures → scored_eval still passes
    // (1 scored, 1 no_actual, 0 failed). no_actual is coverage,
    // not a regression.
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
    // Regression lock: pr-checker has no actuals file. Doctor
    // stays overall=true with scored_eval status="skipped" —
    // the "unchanged for verticals without actuals" case.
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
