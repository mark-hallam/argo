//! End-to-end lock for `argo worker kpi record/list/series`. Drives
//! the real `argo` binary against a temp store with a couple of
//! observations seeded via the CLI itself.

mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

#[tokio::test]
async fn record_list_series_round_trip() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    // The smoke deliberately does NOT seed a real Worker row — the
    // KPI store keys off plain `(worker_id, tenant_id)` strings, not
    // a foreign key into `worker(id)`. That keeps the smoke focused
    // on the new store path; the worker-charter smoke covers the
    // identity-row case.
    let worker_id = "11111111-1111-1111-1111-111111111111";
    let tenant_id = "22222222-2222-2222-2222-222222222222";

    // record #1: csat = 4.5 with a note
    let rec1 = run(
        &home,
        work.path(),
        &[
            "worker", "kpi", "record",
            worker_id, "csat", "4.5",
            "--tenant", tenant_id,
            "--note", "post-launch survey",
            "--json",
        ],
    );
    assert_success(&rec1);
    let body: Value = serde_json::from_slice(&rec1.stdout).unwrap();
    assert_eq!(body["ok"], true);
    assert_eq!(body["kpi"], "csat");
    assert!(body["id"].as_i64().unwrap_or(0) > 0);

    // record #2: csat = 4.7 (no note)
    let rec2 = run(
        &home,
        work.path(),
        &[
            "worker", "kpi", "record",
            worker_id, "csat", "4.7",
            "--tenant", tenant_id,
            "--json",
        ],
    );
    assert_success(&rec2);

    // record #3: a different KPI — must not bleed into csat queries
    let rec3 = run(
        &home,
        work.path(),
        &[
            "worker", "kpi", "record",
            worker_id, "reply_sla_hours", "12",
            "--tenant", tenant_id,
            "--json",
        ],
    );
    assert_success(&rec3);

    // list (no filter) — three rows
    let list_all = run(
        &home,
        work.path(),
        &[
            "worker", "kpi", "list",
            worker_id,
            "--tenant", tenant_id,
            "--json",
        ],
    );
    assert_success(&list_all);
    let body: Value = serde_json::from_slice(&list_all.stdout).unwrap();
    assert_eq!(body["count"], 3);

    // list --kpi csat — two rows
    let list_csat = run(
        &home,
        work.path(),
        &[
            "worker", "kpi", "list",
            worker_id,
            "--tenant", tenant_id,
            "--kpi", "csat",
            "--json",
        ],
    );
    assert_success(&list_csat);
    let body: Value = serde_json::from_slice(&list_csat.stdout).unwrap();
    assert_eq!(body["count"], 2);

    // series --bucket day — one bucket (all observations same day)
    let series = run(
        &home,
        work.path(),
        &[
            "worker", "kpi", "series",
            worker_id, "csat",
            "--tenant", tenant_id,
            "--bucket", "day",
            "--json",
        ],
    );
    assert_success(&series);
    let body: Value = serde_json::from_slice(&series.stdout).unwrap();
    assert_eq!(body["bucket"], "day");
    assert_eq!(body["count"], 1);
    let bucket_row = &body["rows"][0];
    assert_eq!(bucket_row["count"], 2);
    let avg = bucket_row["avg"].as_f64().unwrap();
    assert!((avg - 4.6).abs() < 1e-9, "avg of 4.5 + 4.7 == 4.6, got {avg}");

    // text-mode list also exits cleanly and has the operator-readable
    // header so we don't accidentally regress to JSON-only output.
    let text = run(
        &home,
        work.path(),
        &["worker", "kpi", "list", worker_id, "--tenant", tenant_id],
    );
    assert_success(&text);
    let stdout = String::from_utf8_lossy(&text.stdout);
    assert!(stdout.contains("observed_at"), "text header present:\n{stdout}");
    assert!(stdout.contains("csat"), "values rendered:\n{stdout}");

    // series rejects a bad bucket label cleanly.
    let bad = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .current_dir(work.path())
        .args([
            "worker", "kpi", "series",
            worker_id, "csat",
            "--tenant", tenant_id,
            "--bucket", "decade",
        ])
        .output()
        .unwrap();
    assert!(!bad.status.success());
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert!(stderr.contains("invalid bucket"), "stderr:\n{stderr}");
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
