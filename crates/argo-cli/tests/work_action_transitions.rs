//! End-to-end tests for the Worker v3 action **state-
//! transition** CLI (#730). Drives `argo worker actions
//! {approve, execute, complete, fail, status}` through the
//! binary against a real workspace + memory store.
//!
//! Scope here: pure transition mechanics. The companion file
//! `work_action_transitions_rendering.rs` covers the
//! workspace-level ledger files and per-action execution
//! bundles (render, auto-refresh, manifest templates).
//!
//! Setup: `fresh_workspace_with_actions` (mock-mode worker
//! produces `verify_price` pending + `post_listing` blocked).
//! Mock-mode only.

mod common;

use common::work_action_helpers::{fresh_workspace_with_actions, list_actions, run_argo};
use serde_json::Value;

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
