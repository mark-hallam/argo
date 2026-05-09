//! End-to-end CLI smoke for `argo worker success`. Drives the real
//! `argo` binary against a temp store with no seeded approval rows —
//! the focus is on the round-trip wiring (clap → handler → output)
//! rather than on the rollup arithmetic, which is covered by
//! `worker_metrics_tests.rs`.

mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

#[tokio::test]
async fn empty_worker_returns_zero_decisions_and_null_rates() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let worker_id = "11111111-1111-1111-1111-111111111111";

    // Markdown surface — exit-clean + heading + zero totals.
    let md = run(&home, work.path(), &["worker", "success", worker_id]);
    assert_success(&md);
    let stdout = String::from_utf8_lossy(&md.stdout);
    assert!(stdout.contains("# Worker"), "heading missing:\n{stdout}");
    assert!(
        stdout.contains("**Total decisions**: 0"),
        "expected zero rollup:\n{stdout}"
    );
    assert!(
        stdout.contains("**Approval rate**: n/a"),
        "rate must surface as `n/a`, never NaN:\n{stdout}"
    );

    // JSON surface — stable schema for downstream consumers.
    let js = run(
        &home,
        work.path(),
        &["worker", "success", worker_id, "--format", "json"],
    );
    assert_success(&js);
    let body: Value = serde_json::from_slice(&js.stdout).unwrap();
    assert_eq!(body["worker_id"], worker_id);
    assert_eq!(body["bucket"], "none");
    let summary = &body["summary"];
    assert_eq!(summary["granted"], 0);
    assert_eq!(summary["revoked"], 0);
    assert_eq!(summary["consumed"], 0);
    assert_eq!(summary["total"], 0);
    assert!(summary["approval_rate"].is_null());
    assert!(summary["consumed_rate"].is_null());
    assert!(body["buckets"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn rejects_bad_worker_id_and_bad_bucket_label() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    // Bad worker id — UUID parse error surfaces.
    let bad_uuid = run(
        &home,
        work.path(),
        &["worker", "success", "not-a-uuid"],
    );
    assert!(!bad_uuid.status.success());
    let stderr = String::from_utf8_lossy(&bad_uuid.stderr);
    assert!(
        stderr.contains("invalid worker_id"),
        "expected friendly worker_id error:\n{stderr}"
    );

    // Bad bucket label — handler validates explicitly so the
    // error message stays operator-readable rather than a clap
    // value-enum panic.
    let worker_id = "11111111-1111-1111-1111-111111111111";
    let bad_bucket = run(
        &home,
        work.path(),
        &[
            "worker", "success", worker_id, "--bucket", "decade",
        ],
    );
    assert!(!bad_bucket.status.success());
    let stderr = String::from_utf8_lossy(&bad_bucket.stderr);
    assert!(
        stderr.contains("invalid --bucket"),
        "expected friendly bucket error:\n{stderr}"
    );
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
