//! End-to-end locks for `argo worker deps add/list/remove/graph/impact`.
//! Three focused tests — round-trip, cycle rejection, transitive impact.
//! Tests share `ARGO_HOME` per-run so writes from `add` are visible to
//! the subsequent `list` / `graph` / `impact` invocations.

mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

#[tokio::test]
async fn add_list_remove_round_trip() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    let add = run(
        &home,
        work.path(),
        &[
            "worker", "deps", "add",
            "alpha", "bravo",
            "--kind", "data",
            "--note", "ledger pulls",
            "--json",
        ],
    );
    assert_success(&add);
    let body: Value = serde_json::from_slice(&add.stdout).unwrap();
    assert_eq!(body["ok"], true);
    assert_eq!(body["a"], "alpha");
    assert_eq!(body["b"], "bravo");
    assert_eq!(body["kind"], "data");

    // out direction returns alpha → bravo
    let list_out = run(
        &home,
        work.path(),
        &["worker", "deps", "list", "alpha", "--direction", "out", "--format", "json"],
    );
    assert_success(&list_out);
    let body: Value = serde_json::from_slice(&list_out.stdout).unwrap();
    assert_eq!(body["count"], 1);
    assert_eq!(body["deps"][0]["a"], "alpha");
    assert_eq!(body["deps"][0]["b"], "bravo");

    // in direction (from bravo) finds the same edge
    let list_in = run(
        &home,
        work.path(),
        &["worker", "deps", "list", "bravo", "--direction", "in", "--format", "json"],
    );
    assert_success(&list_in);
    let body: Value = serde_json::from_slice(&list_in.stdout).unwrap();
    assert_eq!(body["count"], 1);

    // markdown smoke
    let md = run(
        &home,
        work.path(),
        &["worker", "deps", "list", "alpha", "--direction", "out"],
    );
    assert_success(&md);
    let stdout = String::from_utf8_lossy(&md.stdout);
    assert!(stdout.contains("# Worker alpha dependencies (out)"), "header:\n{stdout}");
    assert!(stdout.contains("| data | alpha | bravo | ledger pulls |"), "row:\n{stdout}");

    // remove + verify gone
    let rm = run(
        &home,
        work.path(),
        &["worker", "deps", "remove", "alpha", "bravo", "--kind", "data", "--json"],
    );
    assert_success(&rm);
    let body: Value = serde_json::from_slice(&rm.stdout).unwrap();
    assert_eq!(body["removed"], 1);
    let after = run(
        &home,
        work.path(),
        &["worker", "deps", "list", "alpha", "--direction", "out", "--format", "json"],
    );
    assert_success(&after);
    let body: Value = serde_json::from_slice(&after.stdout).unwrap();
    assert_eq!(body["count"], 0);
}

#[tokio::test]
async fn cycle_rejection_exits_non_zero_with_path() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    assert_success(&run(
        &home,
        work.path(),
        &["worker", "deps", "add", "alpha", "bravo", "--kind", "data"],
    ));

    let cycle = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .current_dir(work.path())
        .args([
            "worker", "deps", "add", "bravo", "alpha", "--kind", "data",
        ])
        .output()
        .unwrap();
    assert!(!cycle.status.success(), "expected failure for cycle");
    let stderr = String::from_utf8_lossy(&cycle.stderr);
    assert!(stderr.contains("cycle detected"), "stderr:\n{stderr}");
    assert!(stderr.contains("alpha"), "stderr:\n{stderr}");
    assert!(stderr.contains("bravo"), "stderr:\n{stderr}");

    // Cross-kind reverse edge succeeds — same workers, different relation.
    assert_success(&run(
        &home,
        work.path(),
        &["worker", "deps", "add", "bravo", "alpha", "--kind", "approval"],
    ));
}

#[tokio::test]
async fn graph_dot_and_impact_transitive_closure() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    // D depends on C, C depends on B, B depends on A — A's outage hits 3 workers.
    for (a, b) in [("B", "A"), ("C", "B"), ("D", "C")] {
        assert_success(&run(
            &home,
            work.path(),
            &["worker", "deps", "add", a, b, "--kind", "handoff"],
        ));
    }

    let dot = run(
        &home,
        work.path(),
        &["worker", "deps", "graph", "--format", "dot"],
    );
    assert_success(&dot);
    let stdout = String::from_utf8_lossy(&dot.stdout);
    assert!(stdout.starts_with("digraph worker_deps {"), "preamble:\n{stdout}");
    assert!(stdout.contains("rankdir=LR;"), "layout:\n{stdout}");
    assert!(
        stdout.contains("\"B\" -> \"A\" [color=\"green\", label=\"handoff\"];"),
        "handoff edge:\n{stdout}"
    );
    assert!(stdout.trim_end().ends_with('}'), "closes:\n{stdout}");

    let impact = run(
        &home,
        work.path(),
        &["worker", "deps", "impact", "A", "--json"],
    );
    assert_success(&impact);
    let body: Value = serde_json::from_slice(&impact.stdout).unwrap();
    assert_eq!(body["count"], 3);
    let depths: Vec<(i64, &str)> = body["impacted"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| (v["depth"].as_i64().unwrap(), v["worker_id"].as_str().unwrap()))
        .collect();
    assert_eq!(depths, vec![(1, "B"), (2, "C"), (3, "D")]);
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
