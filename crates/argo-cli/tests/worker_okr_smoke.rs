//! End-to-end locks for `argo worker okr set/show/progress`. Three
//! focused tests: (1) set→show round-trip + markdown shape; (2) the
//! progress join to `worker_kpi_value` plus the brief's weighted-score
//! formula; (3) input-rejection paths (bad period, zero weight). The
//! tests share `ARGO_HOME` per-run so writes from `set` are visible to
//! the subsequent `show` / `progress` invocations.

mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

#[tokio::test]
async fn set_then_show_round_trip() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let worker_id = "11111111-1111-1111-1111-111111111111";
    let tenant_id = "22222222-2222-2222-2222-222222222222";

    let set = run(
        &home,
        work.path(),
        &[
            "worker", "okr", "set",
            worker_id,
            "--tenant", tenant_id,
            "--period", "2026Q2",
            "--title", "Lift CSAT",
            "--description", "Drive csat above 4.5 across the quarter",
            "--kr", "csat:gte:4.5:2.0",
            "--kr", "reply_sla_hours:lte:12.0:1.0",
            "--json",
        ],
    );
    assert_success(&set);
    let body: Value = serde_json::from_slice(&set.stdout).unwrap();
    assert_eq!(body["ok"], true);
    assert!(body["objective_id"].as_i64().unwrap_or(0) > 0);

    let md = run(
        &home,
        work.path(),
        &[
            "worker", "okr", "show",
            worker_id,
            "--tenant", tenant_id,
            "--period", "2026Q2",
            "--format", "markdown",
        ],
    );
    assert_success(&md);
    let stdout = String::from_utf8_lossy(&md.stdout);
    assert!(stdout.contains("# Worker"), "header present:\n{stdout}");
    assert!(stdout.contains("## 2026Q2 — Lift CSAT"), "objective heading:\n{stdout}");
    assert!(stdout.contains("| csat | gte | 4.5000 | 2.0000 |"), "csat KR:\n{stdout}");
    assert!(
        stdout.contains("| reply_sla_hours | lte | 12.0000 | 1.0000 |"),
        "sla KR:\n{stdout}"
    );

    // Period filter that misses returns count=0 in JSON output.
    let empty = run(
        &home,
        work.path(),
        &[
            "worker", "okr", "show",
            worker_id,
            "--tenant", tenant_id,
            "--period", "2027Q1",
            "--format", "json",
        ],
    );
    assert_success(&empty);
    let body: Value = serde_json::from_slice(&empty.stdout).unwrap();
    assert_eq!(body["count"], 0);
}

#[tokio::test]
async fn progress_joins_latest_kpi_observation() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let worker_id = "33333333-3333-3333-3333-333333333333";
    let tenant_id = "44444444-4444-4444-4444-444444444444";

    let set = run(
        &home,
        work.path(),
        &[
            "worker", "okr", "set",
            worker_id,
            "--tenant", tenant_id,
            "--period", "2026Q2",
            "--title", "Lift CSAT",
            "--kr", "csat:gte:4.5:2.0",
            "--kr", "reply_sla_hours:lte:12.0:1.0",
            "--json",
        ],
    );
    assert_success(&set);

    // Seed two KPI observations: csat 4.7 (passes), reply_sla_hours 24 (fails).
    assert_success(&run(
        &home,
        work.path(),
        &[
            "worker", "kpi", "record",
            worker_id, "csat", "4.7",
            "--tenant", tenant_id,
            "--json",
        ],
    ));
    assert_success(&run(
        &home,
        work.path(),
        &[
            "worker", "kpi", "record",
            worker_id, "reply_sla_hours", "24.0",
            "--tenant", tenant_id,
            "--json",
        ],
    ));

    // Weighted score per the brief: (2*1 + 1*0.5)/3 = 0.833...
    let progress = run(
        &home,
        work.path(),
        &[
            "worker", "okr", "progress",
            worker_id,
            "--tenant", tenant_id,
            "--period", "2026Q2",
            "--json",
        ],
    );
    assert_success(&progress);
    let body: Value = serde_json::from_slice(&progress.stdout).unwrap();
    assert_eq!(body["count"], 1);
    let report = &body["reports"][0];
    let score = report["weighted_score"].as_f64().unwrap();
    let expected = 2.5 / 3.0;
    assert!(
        (score - expected).abs() < 1e-6,
        "weighted_score: {score}, expected {expected}"
    );
    let krs = report["key_results"].as_array().unwrap();
    let csat = krs.iter().find(|k| k["kpi"] == "csat").unwrap();
    assert_eq!(csat["achieved"], true);
    let sla = krs
        .iter()
        .find(|k| k["kpi"] == "reply_sla_hours")
        .unwrap();
    assert_eq!(sla["achieved"], false);
}

#[tokio::test]
async fn rejects_invalid_period_and_zero_weight() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let worker_id = "55555555-5555-5555-5555-555555555555";
    let tenant_id = "66666666-6666-6666-6666-666666666666";

    let bad_period = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .current_dir(work.path())
        .args([
            "worker", "okr", "show",
            worker_id,
            "--tenant", tenant_id,
            "--period", "2026q2",
        ])
        .output()
        .unwrap();
    assert!(!bad_period.status.success(), "expected failure for bad period");
    let stderr = String::from_utf8_lossy(&bad_period.stderr);
    assert!(stderr.contains("YYYYQ"), "stderr:\n{stderr}");

    let zero_weight = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .current_dir(work.path())
        .args([
            "worker", "okr", "set",
            worker_id,
            "--tenant", tenant_id,
            "--period", "2026Q3",
            "--title", "x",
            "--kr", "csat:gte:4.5:0",
        ])
        .output()
        .unwrap();
    assert!(!zero_weight.status.success(), "expected failure for zero weight");
    let stderr = String::from_utf8_lossy(&zero_weight.stderr);
    assert!(stderr.contains("weight"), "stderr:\n{stderr}");
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
