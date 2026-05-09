//! End-to-end lock for `argo worker review` (Stream
//! `claude-worker-review-cycle-cli`). Drives the real `argo` binary
//! against a temp store with no seeded rows — the read-side
//! aggregator must return all-zero counts cleanly and emit valid
//! markdown / JSON for both presets.

mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

#[tokio::test]
async fn empty_worker_returns_all_zero_counts_in_json() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let worker_id = "11111111-2222-7333-8444-555555555555";

    let out = run(
        &home,
        work.path(),
        &[
            "worker", "review", worker_id,
            "--period", "quarterly",
            "--format", "json",
        ],
    );
    assert_success(&out);
    let body: Value = serde_json::from_slice(&out.stdout)
        .expect("JSON output parses");
    assert_eq!(body["worker_id"], worker_id);
    assert_eq!(body["period"], "quarterly");
    assert_eq!(body["summary"]["session_count"], 0);
    assert!(body["summary"]["last_session_rfc3339"].is_null());
    assert_eq!(body["kpis"].as_array().unwrap().len(), 0);
    assert_eq!(body["success"]["total"], 0);
    assert!(body["success"]["approval_rate"].is_null());
    assert_eq!(body["feedback"].as_array().unwrap().len(), 0);
    assert_eq!(body["handoffs_sent"].as_array().unwrap().len(), 0);
    assert_eq!(body["handoffs_received"].as_array().unwrap().len(), 0);
    // worker_anomaly_event is part of the migrated schema, so the
    // section is populated (count = 0) — the soft-link guard fires
    // only when the table is genuinely absent.
    assert_eq!(body["anomalies"]["count"], 0);
}

#[tokio::test]
async fn empty_worker_returns_markdown_with_six_section_headers() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let worker_id = "11111111-2222-7333-8444-555555555556";

    let weekly = run(
        &home,
        work.path(),
        &[
            "worker", "review", worker_id,
            "--period", "weekly",
        ],
    );
    assert_success(&weekly);
    let stdout = String::from_utf8_lossy(&weekly.stdout);
    assert!(
        stdout.starts_with("# Worker (unknown worker) — weekly review"),
        "markdown header missing/wrong:\n{stdout}"
    );
    for section in [
        "## Summary",
        "## KPIs",
        "## Success rate",
        "## Operator feedback",
        "## Handoffs",
        "### Sent",
        "### Received",
        "## Anomalies",
    ] {
        assert!(stdout.contains(section), "missing {section} in:\n{stdout}");
    }
    assert!(stdout.contains("Sessions: **0**"));
    assert!(stdout.contains("Approval rate: **n/a**"));

    // monthly with --since override also exits cleanly and renders.
    let with_since = run(
        &home,
        work.path(),
        &[
            "worker", "review", worker_id,
            "--period", "monthly",
            "--since", "2026-01-01T00:00:00Z",
            "--format", "markdown",
        ],
    );
    assert_success(&with_since);
    let stdout = String::from_utf8_lossy(&with_since.stdout);
    assert!(stdout.contains("monthly review"));
    assert!(stdout.contains("period start: 2026-01-01"));

    // bad period rejected cleanly.
    let bad = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .current_dir(work.path())
        .args(["worker", "review", worker_id, "--period", "yearly"])
        .output()
        .unwrap();
    assert!(!bad.status.success());
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert!(stderr.contains("invalid --period"), "stderr:\n{stderr}");
}

fn run(home: &TempDir, cwd: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
