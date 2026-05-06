//! Integration tests for `argo eval run/list` (H-eval-runner-cli).

mod common;

use common::argo_bin;
use std::path::Path;
use std::process::Command;

fn repo_verticals() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join("verticals")
}

#[test]
fn eval_list_json_reports_marketplace_seller_fixtures() {
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", repo_verticals())
        .args(["eval", "list", "--vertical", "marketplace-seller", "--json"])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json eval list output");
    assert_eq!(v["ok"], true);
    assert_eq!(v["vertical"], "marketplace-seller");
    assert!(v["fixture_count"].as_u64().unwrap() >= 4);
    let fixtures = v["fixtures"].as_array().unwrap();
    assert!(fixtures
        .iter()
        .any(|f| f["id"] == "ms_audit_inventory_missing_photos"));
}

#[test]
fn eval_run_json_executes_marketplace_seller_seed() {
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", repo_verticals())
        .args([
            "eval",
            "run",
            "--vertical",
            "marketplace-seller",
            "--limit",
            "1",
            "--json",
        ])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json eval run output");
    assert_eq!(v["ok"], true);
    assert_eq!(v["summary"]["total"], 1);
    assert_eq!(v["summary"]["passed"], 1);
    assert_eq!(v["fixtures"][0]["id"], "ms_audit_inventory_missing_photos");
}

#[test]
fn eval_run_score_json_includes_judge_output() {
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", repo_verticals())
        .args([
            "eval",
            "run",
            "--vertical",
            "marketplace-seller",
            "--score",
            "--limit",
            "1",
            "--json",
        ])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("json scored eval output");
    let row = &v["fixtures"][0];
    assert_eq!(row["passed"], true);
    assert_eq!(row["score"]["status"], "scored");
    assert_eq!(row["judge"]["source"], "heuristic");
}

#[test]
fn eval_run_requires_vertical_flag() {
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", repo_verticals())
        .args(["eval", "run", "--json"])
        .output()
        .expect("spawn");
    assert!(!out.status.success(), "missing --vertical must fail");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("--vertical"), "got: {stderr}");
}
