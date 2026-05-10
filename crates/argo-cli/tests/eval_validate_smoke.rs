//! Integration smoke for `argo eval validate <vertical>` —
//! `mvp-vertical-eval-harness`. Asserts the brief's envelope
//! shape: `{ok, vertical, total, passed, failed, pass_rate, seeds}`
//! and that the deterministic-mock judge produces the same
//! pass/fail counts twice in a row.

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

fn run_validate(vertical: &str) -> std::process::Output {
    let home = tempfile::tempdir().unwrap();
    Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", repo_verticals())
        .args(["eval", "validate", vertical, "--json"])
        .output()
        .expect("spawn argo eval validate")
}

#[test]
fn eval_validate_finance_ar_emits_brief_envelope() {
    let out = run_validate("finance-ar");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("stdout not JSON ({e}); stdout=`{stdout}`; stderr=`{stderr}`");
    });

    assert_eq!(v["vertical"], "finance-ar");
    assert!(v["total"].as_u64().is_some(), "missing total: {v}");
    assert!(v["passed"].as_u64().is_some(), "missing passed: {v}");
    assert!(v["failed"].as_u64().is_some(), "missing failed: {v}");
    assert!(v["pass_rate"].as_f64().is_some(), "missing pass_rate: {v}");
    let seeds = v["seeds"].as_array().expect("seeds array");
    assert!(!seeds.is_empty(), "expected at least one seed in finance-ar");
    for seed in seeds {
        assert!(seed["id"].is_string(), "seed missing id: {seed}");
        assert!(seed["passed"].is_boolean(), "seed missing passed: {seed}");
        assert!(seed["status"].is_string(), "seed missing status: {seed}");
    }

    // Brief: surface a Prometheus-style pass-rate metric. Stderr
    // is the channel — stdout stays pure JSON for jq pipelines.
    assert!(
        stderr.contains("vertical_eval_pass_rate{vertical=\"finance-ar\"}"),
        "missing pass_rate metric in stderr: `{stderr}`"
    );
}

#[test]
fn eval_validate_is_deterministic_across_runs() {
    let a = run_validate("finance-ar");
    let b = run_validate("finance-ar");
    let va: serde_json::Value = serde_json::from_slice(&a.stdout).expect("json a");
    let vb: serde_json::Value = serde_json::from_slice(&b.stdout).expect("json b");
    assert_eq!(va["total"], vb["total"]);
    assert_eq!(va["passed"], vb["passed"]);
    assert_eq!(va["failed"], vb["failed"]);
    assert_eq!(va["pass_rate"], vb["pass_rate"]);
    let aids: Vec<&str> = va["seeds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap_or_default())
        .collect();
    let bids: Vec<&str> = vb["seeds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(aids, bids);
}

#[test]
fn eval_validate_unknown_vertical_errors() {
    let out = run_validate("not-a-real-vertical-id-xyz");
    assert!(!out.status.success(), "missing vertical must error");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("not found"),
        "stderr should mention not found: `{stderr}`"
    );
}
