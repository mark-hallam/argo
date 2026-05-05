//! Integration tests for `argo vertical eval <id>` (#656).

mod common;

use common::argo_bin;
use std::path::Path;
use std::process::Command;

fn write_vertical_with_eval(verticals_dir: &Path, id: &str, fixtures: &str) {
    let v = verticals_dir.join(id);
    std::fs::create_dir_all(v.join("eval")).unwrap();
    std::fs::write(
        v.join("vertical.toml"),
        format!(
            "schema_version = \"1\"\n\
             [vertical]\nid = \"{id}\"\nname = \"X\"\nversion = \"0.1.0\"\n\
             [evaluation]\nsuites = [\"eval/main.jsonl\"]\n"
        ),
    )
    .unwrap();
    std::fs::write(v.join("eval/main.jsonl"), fixtures).unwrap();
}

#[test]
fn cli_eval_passes_for_clean_suite() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_vertical_with_eval(
        verticals.path(),
        "demo",
        "{\"id\":\"a\",\"input\":\"x\",\"expected\":{}}\n\
         {\"id\":\"b\",\"input\":\"y\",\"expected\":{}}\n",
    );

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "eval", "demo", "--json"])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["id"], "demo");
    assert_eq!(v["ok"], true);
    let suites = v["suites"].as_array().unwrap();
    assert_eq!(suites.len(), 1);
    assert_eq!(suites[0]["fixture_count"], 2);
}

#[test]
fn cli_eval_fails_loud_for_broken_suite() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_vertical_with_eval(
        verticals.path(),
        "demo",
        "{\"id\":\"a\",\"input\":\"x\",\"expected\":{}}\n\
         NOT JSON\n\
         {\"id\":\"\",\"input\":\"y\",\"expected\":{}}\n",
    );

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "eval", "demo", "--json"])
        .output()
        .expect("spawn");
    assert!(!out.status.success(), "broken suite must fail");
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], false);
    let errors = v["suites"][0]["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 2, "expected 2 errors, got: {errors:?}");
}

#[test]
fn cli_eval_unknown_id_returns_non_zero() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "eval", "ghost"])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("ghost"), "got: {stderr}");
    assert!(stderr.contains("not found"), "got: {stderr}");
}

#[test]
fn cli_eval_pr_checker_real_asset_passes() {
    // Lock the shipped PR-Checker eval suite. Future edits to
    // verticals/pr-checker/eval/*.jsonl that introduce a
    // malformed line will fail this test.
    let repo_verticals = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join("verticals");
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", &repo_verticals)
        .args(["vertical", "eval", "pr-checker", "--json"])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "PR-Checker eval suite must pass: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["id"], "pr-checker");
    assert_eq!(v["ok"], true);
    let total: u64 = v["suites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["fixture_count"].as_u64().unwrap_or(0))
        .sum();
    assert!(
        total >= 5,
        "PR-Checker suite must have at least 5 fixtures, got {total}"
    );
}

/// Lock the three Worker v3 verticals' eval suites (#759).
/// Each vertical must declare at least one suite, every fixture
/// must be structurally valid, and the suites must carry a
/// minimum-coverage floor so a future "rip out fixtures" PR
/// fails CI loudly. Combined with the per-vertical scored-mode
/// invocation (which reports `no-actual` rows) this gives an
/// always-green structural CI gate even before a real provider
/// run populates the actuals.
#[test]
fn cli_eval_worker_v3_verticals_real_assets_pass() {
    let repo_verticals = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join("verticals");
    // (vertical id, minimum total fixtures across all declared suites)
    let cohort = [
        ("gtm-engineer", 15_u64),
        ("ai-receptionist", 5_u64),
        ("sales-follow-up", 5_u64),
    ];
    for (id, min_fixtures) in cohort {
        let home = tempfile::tempdir().unwrap();
        let out = Command::new(argo_bin())
            .env("ARGO_HOME", home.path())
            .env("ARGO_VERTICALS_DIR", &repo_verticals)
            .args(["vertical", "eval", id, "--json"])
            .output()
            .expect("spawn");
        assert!(
            out.status.success(),
            "vertical '{id}' eval suite must pass: stderr={}",
            String::from_utf8_lossy(&out.stderr)
        );
        let v: serde_json::Value =
            serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
        assert_eq!(v["id"], id);
        assert_eq!(v["ok"], true, "vertical '{id}' reported errors");
        let suites = v["suites"].as_array().unwrap();
        assert!(
            !suites.is_empty(),
            "vertical '{id}' must declare at least one eval suite"
        );
        let total: u64 = suites
            .iter()
            .map(|s| s["fixture_count"].as_u64().unwrap_or(0))
            .sum();
        assert!(
            total >= min_fixtures,
            "vertical '{id}' must have ≥{min_fixtures} fixtures, got {total}"
        );
    }
}
