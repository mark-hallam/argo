//! Smoke tests for `argo budget alerts`.

mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

#[test]
fn synthetic_alert_records_and_lists_json() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(work.path().join(".skycot")).unwrap();

    let fired = run(
        &home,
        work.path(),
        &[
            "budget",
            "alerts",
            "test",
            "--worker",
            "w1",
            "--threshold",
            "100",
            "--json",
        ],
    );
    assert_success(&fired);
    let body: Value = serde_json::from_slice(&fired.stdout).unwrap();
    assert_eq!(body["ok"], true);
    assert_eq!(body["alert"]["worker_id"], "w1");
    assert_eq!(body["alert"]["threshold"], 100);
    assert_eq!(body["alert"]["synthetic"], true);

    let listed = run(
        &home,
        work.path(),
        &["budget", "alerts", "list", "--worker", "w1", "--json"],
    );
    assert_success(&listed);
    let lines: Vec<&[u8]> = listed
        .stdout
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
        .collect();
    assert_eq!(lines.len(), 1);
    let row: Value = serde_json::from_slice(lines[0]).unwrap();
    assert_eq!(row["worker_id"], "w1");
    assert_eq!(row["threshold"], 100);
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
