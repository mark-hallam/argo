//! End-to-end smoke for the subtask ledger
//! (Stream `claude-subtask-ledger-store`).
//!
//! Drives the full surface: the `argo-runtime::Planner` produces a
//! decomposition, `record_decomposition` writes through the
//! `SqliteSubtaskSink`, then the `argo subtask {list,show,tree}`
//! commands read those rows back via the JSON envelope.

use argo_runtime::{record_decomposition, Planner, PlannerContext, TemplatePlanner};
use argo_store::subtask::SqliteSubtaskSink;
use argo_store::SessionStore;
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

async fn seed_decomposition(home: &Path, parent_session: &str) -> Vec<String> {
    let store = SessionStore::open(home.join("sessions.db")).await.unwrap();
    store.migrate().await.unwrap();
    let sink = SqliteSubtaskSink::new(store.pool().clone());
    let planner = TemplatePlanner::shipped_defaults();
    let ctx = PlannerContext::for_worker("wkr-smoke");
    let result = planner
        .decompose("promote our latest blog post", &ctx)
        .await
        .expect("decompose");
    record_decomposition(
        &sink,
        "promote our latest blog post",
        Some(parent_session),
        &ctx,
        &result,
    )
    .await
    .expect("record");
    result
        .workflow
        .tasks
        .iter()
        .map(|t| format!("/{}/{}", result.workflow.id, t.id))
        .collect()
}

#[tokio::test]
async fn end_to_end_decompose_then_list_show_tree() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert_ok(
        &run(workspace.path(), home.path(), &["workspace", "init"]),
        "workspace init",
    );

    let session_id = "sess-smoke-xyz";
    let expected_paths = seed_decomposition(home.path(), session_id).await;
    assert!(
        !expected_paths.is_empty(),
        "planner produced no tasks (sanity check)"
    );

    // `argo subtask list --session <id> --json` returns every row.
    let list_out = run(
        workspace.path(),
        home.path(),
        &["subtask", "list", "--session", session_id, "--json"],
    );
    assert_ok(&list_out, "argo subtask list --json");
    let envelope: serde_json::Value = serde_json::from_slice(&list_out.stdout).unwrap();
    assert_eq!(envelope["ok"], true);
    let count = envelope["count"].as_u64().unwrap();
    assert_eq!(count as usize, expected_paths.len());
    let rows = envelope["rows"].as_array().expect("rows array");
    let returned_paths: Vec<String> = rows
        .iter()
        .map(|r| r["plan_node_path"].as_str().unwrap().to_string())
        .collect();
    for p in &expected_paths {
        assert!(
            returned_paths.iter().any(|r| r == p),
            "missing expected path {p} in {returned_paths:?}",
        );
    }

    // `argo subtask tree <session> --json` returns rows ordered by
    // created_at ascending, matching insert order.
    let tree_out = run(
        workspace.path(),
        home.path(),
        &["subtask", "tree", session_id, "--json"],
    );
    assert_ok(&tree_out, "argo subtask tree --json");
    let tree_env: serde_json::Value = serde_json::from_slice(&tree_out.stdout).unwrap();
    assert_eq!(tree_env["ok"], true);
    assert_eq!(tree_env["session"], session_id);
    let tree_rows = tree_env["rows"].as_array().unwrap();
    assert_eq!(tree_rows.len(), expected_paths.len());

    // `argo subtask show <id> --json` round-trips one row.
    let first_id = tree_rows[0]["id"].as_str().unwrap();
    let show_out = run(
        workspace.path(),
        home.path(),
        &["subtask", "show", first_id, "--json"],
    );
    assert_ok(&show_out, "argo subtask show --json");
    let show_env: serde_json::Value = serde_json::from_slice(&show_out.stdout).unwrap();
    assert_eq!(show_env["ok"], true);
    assert_eq!(show_env["row"]["id"], first_id);
    assert_eq!(show_env["row"]["state"], "pending");
    assert_eq!(show_env["row"]["worker_id"], "wkr-smoke");
    assert_eq!(
        show_env["row"]["parent_goal"],
        "promote our latest blog post"
    );
}

#[test]
fn show_unknown_id_errors_cleanly() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert_ok(
        &run(workspace.path(), home.path(), &["workspace", "init"]),
        "workspace init",
    );
    let bad = run(
        workspace.path(),
        home.path(),
        &["subtask", "show", "no-such-id", "--json"],
    );
    assert!(
        !bad.status.success(),
        "expected non-zero on missing subtask"
    );
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert!(
        stderr.contains("no subtask row"),
        "expected helpful error, got: {stderr}",
    );
}

#[test]
fn list_with_unknown_state_errors() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert_ok(
        &run(workspace.path(), home.path(), &["workspace", "init"]),
        "workspace init",
    );
    let bad = run(
        workspace.path(),
        home.path(),
        &["subtask", "list", "--state", "garbage"],
    );
    assert!(!bad.status.success(), "expected non-zero on bad --state");
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert!(
        stderr.contains("unknown --state"),
        "expected helpful error, got: {stderr}",
    );
}
