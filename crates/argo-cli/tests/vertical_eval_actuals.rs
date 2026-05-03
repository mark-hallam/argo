//! End-to-end tests for `argo vertical eval --score --actuals`
//! (#694). Sibling of `vertical_eval_scoring.rs` so each file
//! stays under the 500-LOC budget.
//!
//! Coverage:
//! - external actuals join by id
//! - `actual` vs `response` field aliasing
//! - duplicate id, malformed JSON, missing-id, missing-actual all
//!   fail loudly with line numbers
//! - unknown actuals ids surface as warning + don't fail the run
//! - `--actuals` without `--score` errors clearly
//! - the shipped GTM Engineer `known_good_actuals.jsonl` scores
//!   10/10 against the canonical eval suite (regression lock)

mod common;

use common::argo_bin;
use std::path::Path;
use std::process::Command;

const MIN_VERTICAL_TOML: &str = "schema_version = \"1\"\n\n\
[vertical]\n\
id = \"actuals-fixture\"\n\
name = \"Actuals Fixture\"\n\
version = \"0.1.0\"\n\n\
[evaluation]\n\
suites = [\"eval/cases.jsonl\"]\n";

fn write_synthetic_vertical(verticals: &Path, body: &str) {
    let v = verticals.join("actuals-fixture");
    std::fs::create_dir_all(v.join("eval")).unwrap();
    std::fs::write(v.join("vertical.toml"), MIN_VERTICAL_TOML).unwrap();
    std::fs::write(v.join("eval/cases.jsonl"), body).unwrap();
}

fn run_score_with_actuals(
    verticals: &Path,
    actuals_path: &Path,
) -> (bool, String, String) {
    let out = Command::new(argo_bin())
        .env("ARGO_VERTICALS_DIR", verticals)
        .args([
            "vertical",
            "eval",
            "actuals-fixture",
            "--score",
            "--json",
            "--actuals",
            actuals_path.to_str().unwrap(),
        ])
        .output()
        .expect("spawn");
    (
        out.status.success(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

#[test]
fn actuals_flag_joins_external_responses_by_id() {
    let dir = tempfile::tempdir().unwrap();
    // Suite has TWO fixtures, neither with their own `actual`.
    // Actuals file provides one; the other should land as no_actual.
    let suite = "\
{\"id\":\"a\",\"input\":\"\",\"expected\":{\"must_include\":[\"hello\"]}}
{\"id\":\"b\",\"input\":\"\",\"expected\":{\"must_include\":[\"world\"]}}
";
    write_synthetic_vertical(dir.path(), suite);
    let actuals = dir.path().join("actuals.jsonl");
    std::fs::write(&actuals, "{\"id\":\"a\",\"actual\":\"hello there\"}\n").unwrap();
    let (success, out, _stderr) = run_score_with_actuals(dir.path(), &actuals);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let suite = &v["suites"][0];
    assert!(
        success,
        "missing actuals for fixture `b` is no_actual, not failure; stdout={v}"
    );
    assert_eq!(suite["fixture_count"], 2);
    assert_eq!(suite["scored_count"], 1);
    assert_eq!(suite["no_actual"], 1);
    assert_eq!(suite["passed"], 1);
}

#[test]
fn actuals_response_field_is_alias_for_actual() {
    let dir = tempfile::tempdir().unwrap();
    let suite = "{\"id\":\"a\",\"input\":\"\",\"expected\":{\"must_include\":[\"alpha\"]}}\n";
    write_synthetic_vertical(dir.path(), suite);
    let actuals = dir.path().join("actuals.jsonl");
    // Use `response` instead of `actual` to mirror externally-
    // captured LLM transcripts that often use that key name.
    std::fs::write(&actuals, "{\"id\":\"a\",\"response\":\"alpha is present\"}\n").unwrap();
    let (success, out, _) = run_score_with_actuals(dir.path(), &actuals);
    assert!(success, "out={out}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["suites"][0]["passed"], 1);
}

#[test]
fn duplicate_actual_id_fails_loudly() {
    let dir = tempfile::tempdir().unwrap();
    let suite = "{\"id\":\"a\",\"input\":\"\",\"expected\":{}}\n";
    write_synthetic_vertical(dir.path(), suite);
    let actuals = dir.path().join("dup.jsonl");
    std::fs::write(
        &actuals,
        "{\"id\":\"a\",\"actual\":\"first\"}\n{\"id\":\"a\",\"actual\":\"dup\"}\n",
    )
    .unwrap();
    let (success, _out, stderr) = run_score_with_actuals(dir.path(), &actuals);
    assert!(!success, "duplicate id must fail the run");
    assert!(
        stderr.contains("duplicate") && stderr.contains("`a`"),
        "stderr should name the duplicate id and line: {stderr}"
    );
}

#[test]
fn malformed_actuals_row_fails_loudly() {
    let dir = tempfile::tempdir().unwrap();
    let suite = "{\"id\":\"a\",\"input\":\"\",\"expected\":{}}\n";
    write_synthetic_vertical(dir.path(), suite);
    let actuals = dir.path().join("bad.jsonl");
    std::fs::write(&actuals, "{\"id\":\"a\",\"actual\":\"ok\"}\nNOT JSON\n").unwrap();
    let (success, _out, stderr) = run_score_with_actuals(dir.path(), &actuals);
    assert!(!success, "invalid JSON in actuals must fail");
    assert!(
        stderr.contains("invalid JSON") && stderr.contains("line 2"),
        "stderr should name the bad line: {stderr}"
    );
}

#[test]
fn actuals_missing_id_field_fails_loudly() {
    let dir = tempfile::tempdir().unwrap();
    let suite = "{\"id\":\"a\",\"input\":\"\",\"expected\":{}}\n";
    write_synthetic_vertical(dir.path(), suite);
    let actuals = dir.path().join("no_id.jsonl");
    std::fs::write(&actuals, "{\"actual\":\"orphan\"}\n").unwrap();
    let (success, _out, stderr) = run_score_with_actuals(dir.path(), &actuals);
    assert!(!success);
    assert!(stderr.contains("missing `id`"), "stderr: {stderr}");
}

#[test]
fn actuals_missing_actual_and_response_fails_loudly() {
    let dir = tempfile::tempdir().unwrap();
    let suite = "{\"id\":\"a\",\"input\":\"\",\"expected\":{}}\n";
    write_synthetic_vertical(dir.path(), suite);
    let actuals = dir.path().join("empty_row.jsonl");
    std::fs::write(&actuals, "{\"id\":\"a\"}\n").unwrap();
    let (success, _out, stderr) = run_score_with_actuals(dir.path(), &actuals);
    assert!(!success);
    assert!(
        stderr.contains("actual") && stderr.contains("response"),
        "stderr should name both keys: {stderr}"
    );
}

#[test]
fn unknown_actuals_ids_warn_but_do_not_fail() {
    let dir = tempfile::tempdir().unwrap();
    let suite = "{\"id\":\"a\",\"input\":\"\",\"expected\":{\"must_include\":[\"x\"]}}\n";
    write_synthetic_vertical(dir.path(), suite);
    let actuals = dir.path().join("with_extras.jsonl");
    std::fs::write(
        &actuals,
        "{\"id\":\"a\",\"actual\":\"x present\"}\n{\"id\":\"phantom\",\"actual\":\"unused\"}\n",
    )
    .unwrap();
    let (success, out, _) = run_score_with_actuals(dir.path(), &actuals);
    assert!(success, "unknown ids are a warning, not a failure: {out}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let suite = &v["suites"][0];
    assert_eq!(suite["passed"], 1);
    assert_eq!(
        suite["unknown_actual_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["phantom"]
    );
}

#[test]
fn actuals_flag_without_score_flag_errors_clearly() {
    let dir = tempfile::tempdir().unwrap();
    let suite = "{\"id\":\"a\",\"input\":\"\",\"expected\":{}}\n";
    write_synthetic_vertical(dir.path(), suite);
    let actuals = dir.path().join("a.jsonl");
    std::fs::write(&actuals, "{\"id\":\"a\",\"actual\":\"x\"}\n").unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_VERTICALS_DIR", dir.path())
        .args([
            "vertical",
            "eval",
            "actuals-fixture",
            "--actuals",
            actuals.to_str().unwrap(),
        ])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("--actuals requires --score"),
        "stderr should explain the constraint: {stderr}"
    );
}

#[test]
fn gtm_engineer_scored_with_known_good_actuals_passes_ten_of_ten() {
    // Critical regression lock: PR #694 ships
    // verticals/gtm-engineer/eval/known_good_actuals.jsonl with
    // 10 hand-curated reference responses, every one of which
    // is engineered to pass the rule engine. Any future edit to
    // either file (rules, fixtures, or actuals) that breaks the
    // 10/10 pass rate is an intentional change worth surfacing
    // in PR review — this test makes that change loud.
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf();
    let repo_verticals = workspace_root.join("verticals");
    let actuals = workspace_root.join("verticals/gtm-engineer/eval/known_good_actuals.jsonl");
    assert!(actuals.is_file(), "shipped actuals file must exist: {actuals:?}");
    let out = Command::new(argo_bin())
        .env("ARGO_VERTICALS_DIR", &repo_verticals)
        .args([
            "vertical",
            "eval",
            "gtm-engineer",
            "--score",
            "--json",
            "--actuals",
            actuals.to_str().unwrap(),
        ])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "gtm-engineer scored eval with known-good actuals must pass; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    let suite = &v["suites"][0];
    assert_eq!(suite["fixture_count"], 10, "expected 10 fixtures: {suite}");
    assert_eq!(suite["scored_count"], 10, "all 10 should have actuals: {suite}");
    assert_eq!(suite["passed"], 10, "all 10 must pass: {suite}");
    assert_eq!(suite["failed"], 0);
    assert_eq!(suite["no_actual"], 0);
    assert_eq!(suite["malformed"], 0);
    assert_eq!(suite["pass_rate"].as_f64().unwrap(), 1.0);
    // Specific safety-critical fixtures the user spec called out.
    let fixtures = suite["fixtures"].as_array().unwrap();
    for required in [
        "gtm_safety_no_send",
        "gtm_abstain_weak_context",
        "gtm_no_skill_from_weak_evidence",
        "gtm_company_personalization",
        "gtm_low_reply_analysis",
    ] {
        let row = fixtures
            .iter()
            .find(|f| f["id"] == required)
            .unwrap_or_else(|| panic!("missing required fixture id `{required}`"));
        assert_eq!(
            row["passed"], true,
            "safety-critical fixture `{required}` must pass: {row}"
        );
    }
}
