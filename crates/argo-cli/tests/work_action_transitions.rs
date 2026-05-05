//! End-to-end test for the Worker v3 action transition CLI
//! (#730). Drives the same `argo worker actions {approve,
//! execute, complete, fail, status, render}` surface an
//! operator uses — through the binary, against a real
//! workspace + memory store.
//!
//! Setup:
//!   1. fresh workspace (tempdir)
//!   2. mock-mode worker run produces 2 actions:
//!      - verify_price → state = pending (autonomous-safe)
//!      - post_listing → state = blocked (gated, evidence
//!        insufficient under mock)
//!   3. drive transitions:
//!      - approve refuses verify_price (autonomous)
//!      - execute advances verify_price pending → executing
//!      - complete advances executing → completed with note
//!      - status emits the full 3-row history
//!      - render writes 3 markdown ledger files under .skycot/
//!
//! Mock-mode only.

mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn run_argo(home: &Path, work: &Path, args: &[&str]) -> std::process::Output {
    Command::new(argo_bin())
        .args(args)
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", workspace_root().join("verticals"))
        .current_dir(work)
        .output()
        .expect("spawn argo")
}

fn fresh_workspace_with_actions() -> (tempfile::TempDir, tempfile::TempDir) {
    let home = tempfile::tempdir().expect("home tempdir");
    let work = tempfile::tempdir().expect("work tempdir");
    let init = run_argo(home.path(), work.path(), &["workspace", "init"]);
    assert!(
        init.status.success(),
        "init: {}",
        String::from_utf8_lossy(&init.stderr)
    );
    let add = run_argo(
        home.path(),
        work.path(),
        &[
            "work",
            "add",
            "--vertical",
            "gtm-engineer",
            "--prompt",
            "go",
            "--json",
        ],
    );
    assert!(
        add.status.success(),
        "{}",
        String::from_utf8_lossy(&add.stderr)
    );
    let serve = run_argo(
        home.path(),
        work.path(),
        &[
            "work",
            "serve",
            "--vertical",
            "gtm-engineer",
            "--once",
            "--max-steps",
            "1",
            "--json",
        ],
    );
    assert!(
        serve.status.success(),
        "serve: {}",
        String::from_utf8_lossy(&serve.stderr)
    );
    (home, work)
}

fn list_actions(home: &Path, work: &Path) -> Vec<Value> {
    let out = run_argo(
        home,
        work,
        &[
            "worker",
            "actions",
            "list",
            "--state",
            "all",
            "--vertical",
            "gtm-engineer",
            "--json",
        ],
    );
    assert!(
        out.status.success(),
        "list: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

#[test]
fn approval_refused_for_autonomous_safe_action() {
    let (home, work) = fresh_workspace_with_actions();
    let rows = list_actions(home.path(), work.path());
    let verify = rows
        .iter()
        .find(|r| r["action_type"] == "verify_price")
        .expect("verify_price row");
    let id = verify["action_id"].as_str().expect("action_id");

    let out = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "approve", id],
    );
    assert!(!out.status.success(), "approve must refuse autonomous-safe");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("autonomous-safe"),
        "stderr should explain why; got: {stderr}"
    );
}

#[test]
fn execute_then_complete_round_trips_an_autonomous_action() {
    let (home, work) = fresh_workspace_with_actions();
    let rows = list_actions(home.path(), work.path());
    let verify = rows
        .iter()
        .find(|r| r["action_type"] == "verify_price")
        .expect("verify_price row");
    let id = verify["action_id"].as_str().expect("action_id");

    let exec = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "execute", id, "--json"],
    );
    assert!(
        exec.status.success(),
        "execute: {}",
        String::from_utf8_lossy(&exec.stderr)
    );
    let v: Value = serde_json::from_slice(&exec.stdout).expect("execute json");
    assert_eq!(v["state"], "executing");
    assert!(v["executed_at"].as_str().is_some());

    let complete = run_argo(
        home.path(),
        work.path(),
        &[
            "worker",
            "actions",
            "complete",
            id,
            "--note",
            "ran offline; price OK",
            "--json",
        ],
    );
    assert!(
        complete.status.success(),
        "complete: {}",
        String::from_utf8_lossy(&complete.stderr)
    );
    let v: Value = serde_json::from_slice(&complete.stdout).expect("complete json");
    assert_eq!(v["state"], "completed");
    assert_eq!(v["result"]["summary"], "ran offline; price OK");
}

#[test]
fn status_emits_full_state_history_in_time_order() {
    let (home, work) = fresh_workspace_with_actions();
    let rows = list_actions(home.path(), work.path());
    let verify = rows
        .iter()
        .find(|r| r["action_type"] == "verify_price")
        .expect("verify_price row");
    let id = verify["action_id"].as_str().expect("action_id");

    let _ = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "execute", id, "--json"],
    );
    let _ = run_argo(
        home.path(),
        work.path(),
        &[
            "worker", "actions", "complete", id, "--note", "ok", "--json",
        ],
    );

    let status = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "status", id, "--json"],
    );
    assert!(
        status.status.success(),
        "status: {}",
        String::from_utf8_lossy(&status.stderr)
    );
    let states: Vec<String> = String::from_utf8_lossy(&status.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter_map(|v| v["state"].as_str().map(String::from))
        .collect();
    assert_eq!(states, vec!["pending", "executing", "completed"]);
}

#[test]
fn render_writes_workspace_level_ledger_artifacts() {
    let (home, work) = fresh_workspace_with_actions();
    let render = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "render", "--vertical", "gtm-engineer"],
    );
    assert!(
        render.status.success(),
        "render: {}",
        String::from_utf8_lossy(&render.stderr)
    );
    let dir = work.path().join(".skycot");
    let queue = std::fs::read_to_string(dir.join("approval_queue.md")).expect("approval_queue.md");
    let blocked =
        std::fs::read_to_string(dir.join("blocked_actions.md")).expect("blocked_actions.md");
    let completed =
        std::fs::read_to_string(dir.join("completed_actions.md")).expect("completed_actions.md");

    // Mock evidence is insufficient → post_listing landed
    // blocked, not pending. Approval queue is empty.
    assert!(queue.contains("(no actions awaiting approval)"));
    // post_listing should appear in blocked.
    assert!(
        blocked.contains("post_listing"),
        "expected post_listing in blocked; got:\n{blocked}"
    );
    // No completes yet.
    assert!(completed.contains("(no actions have completed yet)"));
}

#[test]
fn fail_records_reason_for_an_executing_action() {
    let (home, work) = fresh_workspace_with_actions();
    let rows = list_actions(home.path(), work.path());
    let verify = rows
        .iter()
        .find(|r| r["action_type"] == "verify_price")
        .expect("verify_price row");
    let id = verify["action_id"].as_str().expect("action_id");

    let _ = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "execute", id, "--json"],
    );
    let fail = run_argo(
        home.path(),
        work.path(),
        &[
            "worker",
            "actions",
            "fail",
            id,
            "--reason",
            "marketplace returned 503",
            "--json",
        ],
    );
    assert!(
        fail.status.success(),
        "fail: {}",
        String::from_utf8_lossy(&fail.stderr)
    );
    let v: Value = serde_json::from_slice(&fail.stdout).expect("fail json");
    assert_eq!(v["state"], "failed");
    assert_eq!(v["result"]["failure"], "marketplace returned 503");
}
