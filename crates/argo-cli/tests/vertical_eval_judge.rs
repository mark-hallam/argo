//! End-to-end tests for the optional `--judge` layer (#698).
//! Drives the binary so the CLI surface is exercised at the
//! same layer operators use.
//!
//! Coverage:
//! - `--judge` off (default): JSON shape unchanged from #694
//! - `--judge heuristic`: each Scored fixture gains a `judge`
//!   block with passed + reason + source = "heuristic"
//! - `--judge <provider>/<model>`: live placeholder verdict
//!   surfaces with source = "not_wired:<spec>"
//! - `--judge invalid`: rejected with a clear error before any
//!   fixture is touched
//! - `--judge requires --score` (errors when used standalone)
//! - heuristic fail propagates: a fixture whose rule engine
//!   fails AND the heuristic judge agrees flips the suite
//!   status, exits non-zero
//! - heuristic pass on shipped GTM Engineer actuals: every one
//!   of the 10 fixtures gets `judge.passed = true`

mod common;

use common::argo_bin;
use std::path::Path;
use std::process::Command;

const MIN_VERTICAL_TOML: &str = "schema_version = \"1\"\n\n\
[vertical]\n\
id = \"judge-fixture\"\n\
name = \"Judge Fixture\"\n\
version = \"0.1.0\"\n\n\
[evaluation]\n\
suites = [\"eval/cases.jsonl\"]\n";

fn write_synthetic_vertical(verticals: &Path, body: &str) {
    let v = verticals.join("judge-fixture");
    std::fs::create_dir_all(v.join("eval")).unwrap();
    std::fs::write(v.join("vertical.toml"), MIN_VERTICAL_TOML).unwrap();
    std::fs::write(v.join("eval/cases.jsonl"), body).unwrap();
}

fn run_eval(args: &[&str], verticals: &Path) -> (bool, String, String) {
    let out = Command::new(argo_bin())
        .env("ARGO_VERTICALS_DIR", verticals)
        .args(args)
        .output()
        .expect("spawn");
    (
        out.status.success(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

#[test]
fn judge_off_by_default_omits_judge_field_from_json() {
    let dir = tempfile::tempdir().unwrap();
    write_synthetic_vertical(
        dir.path(),
        "{\"id\":\"a\",\"input\":\"\",\"expected\":{\"must_include\":[\"x\"]},\"actual\":\"x present\"}\n",
    );
    let (success, out, _) = run_eval(
        &["vertical", "eval", "judge-fixture", "--score", "--json"],
        dir.path(),
    );
    assert!(success, "out={out}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let fx = &v["suites"][0]["fixtures"][0];
    assert!(
        fx.get("judge").is_none(),
        "default mode must NOT emit a judge field: {fx}"
    );
}

#[test]
fn judge_heuristic_attaches_verdict_when_rules_pass() {
    let dir = tempfile::tempdir().unwrap();
    write_synthetic_vertical(
        dir.path(),
        "{\"id\":\"a\",\"input\":\"\",\"expected\":{\"must_include\":[\"x\"]},\"actual\":\"x present\"}\n",
    );
    let (success, out, _) = run_eval(
        &[
            "vertical", "eval", "judge-fixture", "--score", "--json",
            "--judge", "heuristic",
        ],
        dir.path(),
    );
    assert!(success, "out={out}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let fx = &v["suites"][0]["fixtures"][0];
    let judge = &fx["judge"];
    assert_eq!(judge["passed"], true);
    assert_eq!(judge["source"], "heuristic");
    assert!(
        judge["reason"]
            .as_str()
            .unwrap()
            .contains("rule check"),
        "reason should reference rule checks: {judge}"
    );
}

#[test]
fn judge_heuristic_propagates_rule_failure_into_suite_failure() {
    let dir = tempfile::tempdir().unwrap();
    // Fixture fails the rule engine — judge agrees.
    write_synthetic_vertical(
        dir.path(),
        "{\"id\":\"a\",\"input\":\"\",\"expected\":{\"must_include\":[\"absent\"]},\"actual\":\"this lacks the keyword\"}\n",
    );
    let (success, out, _) = run_eval(
        &[
            "vertical", "eval", "judge-fixture", "--score", "--json",
            "--judge", "heuristic",
        ],
        dir.path(),
    );
    assert!(!success, "rule + judge fail must exit non-zero");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["ok"], false);
    let fx = &v["suites"][0]["fixtures"][0];
    let judge = &fx["judge"];
    assert_eq!(judge["passed"], false);
    assert!(
        judge["reason"]
            .as_str()
            .unwrap()
            .contains("absent"),
        "judge reason should surface the failing-rule detail: {judge}"
    );
}

#[test]
fn judge_live_placeholder_attaches_not_wired_verdict() {
    let dir = tempfile::tempdir().unwrap();
    write_synthetic_vertical(
        dir.path(),
        "{\"id\":\"a\",\"input\":\"\",\"expected\":{\"must_include\":[\"x\"]},\"actual\":\"x present\"}\n",
    );
    let (success, out, _) = run_eval(
        &[
            "vertical", "eval", "judge-fixture", "--score", "--json",
            "--judge", "anthropic/claude-sonnet-4-6",
        ],
        dir.path(),
    );
    assert!(success, "live placeholder mirrors rule pass: out={out}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let fx = &v["suites"][0]["fixtures"][0];
    let judge = &fx["judge"];
    assert_eq!(judge["passed"], true);
    assert_eq!(judge["source"], "not_wired:anthropic/claude-sonnet-4-6");
    let reason = judge["reason"].as_str().unwrap();
    assert!(reason.contains("not yet wired"), "reason={reason}");
    assert!(reason.contains("anthropic/claude-sonnet-4-6"), "reason={reason}");
}

#[test]
fn judge_invalid_spec_is_rejected_before_any_fixture_runs() {
    let dir = tempfile::tempdir().unwrap();
    write_synthetic_vertical(
        dir.path(),
        "{\"id\":\"a\",\"input\":\"\",\"expected\":{}}\n",
    );
    let (success, _out, stderr) = run_eval(
        &[
            "vertical", "eval", "judge-fixture", "--score", "--judge", "invalid-no-slash",
        ],
        dir.path(),
    );
    assert!(!success, "malformed spec must fail");
    assert!(
        stderr.contains("provider>/<model"),
        "stderr should explain the format: {stderr}"
    );
}

#[test]
fn judge_without_score_flag_errors_clearly() {
    let dir = tempfile::tempdir().unwrap();
    write_synthetic_vertical(
        dir.path(),
        "{\"id\":\"a\",\"input\":\"\",\"expected\":{}}\n",
    );
    let (success, _out, stderr) = run_eval(
        &["vertical", "eval", "judge-fixture", "--judge", "heuristic"],
        dir.path(),
    );
    assert!(!success);
    assert!(
        stderr.contains("--judge requires --score"),
        "stderr: {stderr}"
    );
}

#[test]
fn gtm_engineer_with_known_good_actuals_passes_heuristic_judge_ten_of_ten() {
    // Regression lock: every shipped GTM actual passes the rule
    // engine AND the heuristic judge agrees. If a future rule
    // change causes the judge to flip on any of the 10, this
    // test fails fast.
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf();
    let actuals = workspace_root.join("verticals/gtm-engineer/eval/known_good_actuals.jsonl");
    assert!(actuals.is_file());
    let (success, out, _) = run_eval(
        &[
            "vertical",
            "eval",
            "gtm-engineer",
            "--score",
            "--json",
            "--actuals",
            actuals.to_str().unwrap(),
            "--judge",
            "heuristic",
        ],
        &workspace_root.join("verticals"),
    );
    assert!(success, "10/10 must pass with heuristic judge");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let fixtures = v["suites"][0]["fixtures"].as_array().unwrap();
    assert_eq!(fixtures.len(), 10);
    for fx in fixtures {
        assert_eq!(
            fx["judge"]["passed"], true,
            "fixture `{}` heuristic judge disagreed: {fx}",
            fx["id"]
        );
        assert_eq!(fx["judge"]["source"], "heuristic");
    }
}
