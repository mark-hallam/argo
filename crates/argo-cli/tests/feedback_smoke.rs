//! End-to-end smoke for `argo feedback up/down/list` (Stream
//! `claude-operator-feedback-channel`).

use std::path::Path;
use std::process::{Command, Output};

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn run(workspace: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .current_dir(workspace)
        .env("ARGO_HOME", home)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

fn assert_ok(out: &Output, label: &str) {
    assert!(
        out.status.success(),
        "{label} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn up_then_down_then_list_filters_correctly() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert_ok(
        &run(workspace.path(), home.path(), &["workspace", "init"]),
        "workspace init",
    );

    // Up vote with comment.
    let up = run(
        workspace.path(),
        home.path(),
        &[
            "feedback",
            "up",
            "act-001",
            "--comment",
            "great closer",
            "--worker",
            "wkr-1",
            "--json",
        ],
    );
    assert_ok(&up, "feedback up");
    let up_json: serde_json::Value = serde_json::from_slice(&up.stdout).unwrap();
    assert_eq!(up_json["vote"], "up");
    assert_eq!(up_json["action_id"], "act-001");

    // Down vote, no comment.
    let down = run(
        workspace.path(),
        home.path(),
        &[
            "feedback",
            "down",
            "act-002",
            "--worker",
            "wkr-1",
            "--json",
        ],
    );
    assert_ok(&down, "feedback down");

    // Another up for a different worker.
    assert_ok(
        &run(
            workspace.path(),
            home.path(),
            &["feedback", "up", "act-003", "--worker", "wkr-2", "--json"],
        ),
        "feedback up wkr-2",
    );

    // List all → 3 rows.
    let list_all = run(
        workspace.path(),
        home.path(),
        &["feedback", "list", "--json"],
    );
    assert_ok(&list_all, "feedback list all");
    let arr: serde_json::Value = serde_json::from_slice(&list_all.stdout).unwrap();
    assert_eq!(
        arr.as_array().map_or(0, Vec::len),
        3,
        "expected 3 rows: {arr}"
    );

    // List filtered by worker=wkr-1 → 2 rows.
    let list_w1 = run(
        workspace.path(),
        home.path(),
        &["feedback", "list", "--worker", "wkr-1", "--json"],
    );
    assert_ok(&list_w1, "feedback list wkr-1");
    let arr: serde_json::Value = serde_json::from_slice(&list_w1.stdout).unwrap();
    assert_eq!(
        arr.as_array().map_or(0, Vec::len),
        2,
        "expected 2 rows for wkr-1: {arr}"
    );

    // List filtered by vote=down → 1 row.
    let list_down = run(
        workspace.path(),
        home.path(),
        &["feedback", "list", "--vote", "down", "--json"],
    );
    assert_ok(&list_down, "feedback list down");
    let arr: serde_json::Value = serde_json::from_slice(&list_down.stdout).unwrap();
    assert_eq!(
        arr.as_array().map_or(0, Vec::len),
        1,
        "expected 1 down row: {arr}"
    );
    let row = &arr[0];
    assert_eq!(row["vote"], "down");
    assert_eq!(row["action_id"], "act-002");

    // Combined filter: worker=wkr-1 + vote=up → 1 row.
    let list_w1_up = run(
        workspace.path(),
        home.path(),
        &[
            "feedback",
            "list",
            "--worker",
            "wkr-1",
            "--vote",
            "up",
            "--json",
        ],
    );
    assert_ok(&list_w1_up, "feedback list wkr-1 up");
    let arr: serde_json::Value = serde_json::from_slice(&list_w1_up.stdout).unwrap();
    assert_eq!(arr.as_array().map_or(0, Vec::len), 1);
    assert_eq!(arr[0]["action_id"], "act-001");
    assert_eq!(arr[0]["comment"], "great closer");
}

#[test]
fn invalid_vote_filter_errors_cleanly() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert_ok(
        &run(workspace.path(), home.path(), &["workspace", "init"]),
        "workspace init",
    );
    let bad = run(
        workspace.path(),
        home.path(),
        &["feedback", "list", "--vote", "maybe"],
    );
    assert!(!bad.status.success(), "expected non-zero exit on bad vote");
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert!(
        stderr.contains("--vote must be") || stderr.contains("'up' or 'down'"),
        "expected helpful error, got: {stderr}",
    );
}
