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

// REPLACED in #701: this slot used to lock the `not_wired`
// placeholder behaviour from #698. PR #701 wires the live
// transport, so `--judge anthropic/...` now invokes the real
// backend (or fails loudly without an API key — covered by
// `live_judge_without_api_key_errors_clearly_with_actionable_message`
// below). The pure-helper placeholder is locked at the
// argo-vertical sibling test
// `live_placeholder_mirrors_rule_outcome_and_explains_gap`.

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

// ---- live judge wiring (#701) -----------------------------------

#[test]
fn judge_systemic_failure_field_is_false_when_judge_is_off() {
    // Regression lock for the JSON shape (#701c added
    // `judge_systemic_failure` at the top level). Off-by-
    // default mode: no judge ran, no systemic failure, field
    // present and false.
    let dir = tempfile::tempdir().unwrap();
    write_synthetic_vertical(
        dir.path(),
        "{\"id\":\"a\",\"input\":\"\",\"expected\":{\"must_include\":[\"x\"]},\"actual\":\"x present\"}\n",
    );
    let (success, out, _) = run_eval(
        &["vertical", "eval", "judge-fixture", "--score", "--json"],
        dir.path(),
    );
    assert!(success);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(
        v["judge_systemic_failure"], false,
        "off-by-default mode must report systemic_failure=false: {v}"
    );
}

#[test]
fn judge_systemic_failure_field_is_false_when_heuristic_judge_passes() {
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
    assert!(success);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["judge_systemic_failure"], false);
}

#[test]
fn judge_systemic_failure_field_distinct_from_per_fixture_rule_failure() {
    // Critical contract: the LLM/heuristic judge ran fine
    // but the agent's output failed the rules. The systemic
    // flag stays false because the judge itself succeeded —
    // the fixture failure is a GTM-quality regression, not a
    // configuration problem. Operators can tell them apart.
    let dir = tempfile::tempdir().unwrap();
    write_synthetic_vertical(
        dir.path(),
        "{\"id\":\"a\",\"input\":\"\",\"expected\":{\"must_include\":[\"missing-keyword-xyz\"]},\"actual\":\"this body is unrelated\"}\n",
    );
    let (_success, out, _) = run_eval(
        &[
            "vertical", "eval", "judge-fixture", "--score", "--json",
            "--judge", "heuristic",
        ],
        dir.path(),
    );
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["ok"], false, "fixture failed");
    assert_eq!(
        v["judge_systemic_failure"], false,
        "rule failure ≠ systemic judge failure: {v}"
    );
}

#[test]
fn live_judge_without_api_key_errors_clearly_with_actionable_message() {
    // Critical contract: when the operator passes
    // `--judge <provider>/<model>` without setting the
    // matching API key env, the run must fail BEFORE any
    // grading happens. The error must name the missing env
    // var (or, if the manifest itself is unreachable, name
    // the manifest path) and the offline fallback (`--judge
    // heuristic`). Locks the "no API key in CI" guarantee.
    let dir = tempfile::tempdir().unwrap();
    write_synthetic_vertical(
        dir.path(),
        "{\"id\":\"a\",\"input\":\"x\",\"expected\":{\"must_include\":[\"x\"]},\"actual\":\"x present\"}\n",
    );
    let out = Command::new(argo_bin())
        .env("ARGO_VERTICALS_DIR", dir.path())
        .env_remove("ANTHROPIC_API_KEY")
        .args([
            "vertical",
            "eval",
            "judge-fixture",
            "--score",
            "--judge",
            "anthropic/claude-sonnet-4-6",
        ])
        .output()
        .expect("spawn");
    assert!(!out.status.success(), "must fail without key");
    let stderr = String::from_utf8(out.stderr).unwrap();
    // Either error path is acceptable: missing key OR
    // missing manifest. Both emit a recognisable "live
    // judge:" prefix from build_anthropic_backend so
    // operators know to fix configuration before re-running.
    assert!(
        stderr.contains("live judge:"),
        "stderr should be prefixed by live-judge configuration error: {stderr}"
    );
}

#[test]
fn judge_verdict_json_includes_score_field() {
    // #701 added `score` to JudgeVerdict. Lock the JSON shape —
    // operators downstream parse this.
    let dir = tempfile::tempdir().unwrap();
    write_synthetic_vertical(
        dir.path(),
        "{\"id\":\"a\",\"input\":\"\",\"expected\":{\"must_include\":[\"specific\"]},\"actual\":\"contains specific words here\"}\n",
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
    let judge = &v["suites"][0]["fixtures"][0]["judge"];
    assert_eq!(judge["passed"], true);
    assert_eq!(judge["score"].as_f64().unwrap(), 1.0);
    // failed_criteria is skip-if-empty, so it's absent on pass.
    assert!(
        judge.get("failed_criteria").is_none(),
        "passing verdict should NOT carry failed_criteria: {judge}"
    );
}

#[test]
fn judge_verdict_json_failed_criteria_populated_on_heuristic_fail() {
    let dir = tempfile::tempdir().unwrap();
    write_synthetic_vertical(
        dir.path(),
        "{\"id\":\"a\",\"input\":\"\",\"expected\":{\"must_include\":[\"absent\",\"missing\"]},\"actual\":\"this contains neither keyword\"}\n",
    );
    let (success, out, _) = run_eval(
        &[
            "vertical", "eval", "judge-fixture", "--score", "--json",
            "--judge", "heuristic",
        ],
        dir.path(),
    );
    assert!(!success);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let judge = &v["suites"][0]["fixtures"][0]["judge"];
    assert_eq!(judge["passed"], false);
    assert_eq!(judge["score"].as_f64().unwrap(), 0.0);
    let criteria = judge["failed_criteria"].as_array().unwrap();
    assert_eq!(criteria.len(), 2, "two must_include rules failed: {judge}");
    assert!(criteria.iter().any(|c| c.as_str().unwrap().contains("absent")));
    assert!(criteria.iter().any(|c| c.as_str().unwrap().contains("missing")));
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
