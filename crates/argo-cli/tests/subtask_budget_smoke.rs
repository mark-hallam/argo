//! End-to-end smoke for `argo subtask budget <root>` (Stream
//! `mission-delegation-budget-tree`). Seeds a two-level subtask
//! tree (root + two children, all rooted at a known id) directly
//! through the store, sets per-row budgets and spends, then runs
//! the CLI under `--json` to verify the envelope roll-up matches.

use argo_store::subtask::{insert, NewSubtask};
use argo_store::subtask_budget::{
    record_subtask_spend, set_subtask_budget, set_subtask_parent,
};
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

async fn seed_tree(home: &Path) -> (String, String, String) {
    let store = SessionStore::open(home.join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let pool = store.pool().clone();
    let root = insert(
        &pool,
        &NewSubtask {
            parent_goal: "delegate-launch",
            parent_session_id: Some("sess-budget"),
            worker_id: "wkr-root",
            plan_node_path: "/root",
            description: "root description",
        },
    )
    .await
    .unwrap();
    let a = insert(
        &pool,
        &NewSubtask {
            parent_goal: "delegate-launch",
            parent_session_id: Some("sess-budget"),
            worker_id: "wkr-a",
            plan_node_path: "/root/a",
            description: "child a",
        },
    )
    .await
    .unwrap();
    let b = insert(
        &pool,
        &NewSubtask {
            parent_goal: "delegate-launch",
            parent_session_id: Some("sess-budget"),
            worker_id: "wkr-b",
            plan_node_path: "/root/b",
            description: "child b",
        },
    )
    .await
    .unwrap();
    set_subtask_parent(&pool, &a, &root).await.unwrap();
    set_subtask_parent(&pool, &b, &root).await.unwrap();
    set_subtask_budget(&pool, &root, 1_000).await.unwrap();
    set_subtask_budget(&pool, &a, 500).await.unwrap();
    set_subtask_budget(&pool, &b, 500).await.unwrap();
    record_subtask_spend(&pool, &root, 25).await.unwrap();
    record_subtask_spend(&pool, &a, 100).await.unwrap();
    record_subtask_spend(&pool, &b, 50).await.unwrap();
    (root, a, b)
}

#[tokio::test]
async fn budget_tree_json_envelope_rolls_up_two_levels() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert_ok(
        &run(workspace.path(), home.path(), &["workspace", "init"]),
        "workspace init",
    );

    let (root_id, a_id, b_id) = seed_tree(home.path()).await;

    let out = run(
        workspace.path(),
        home.path(),
        &["subtask", "budget", &root_id, "--json"],
    );
    assert_ok(&out, "argo subtask budget --json");
    let env: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(env["ok"], true);
    assert_eq!(env["root_id"], root_id);
    assert_eq!(env["count"], 3);
    let rows = env["rows"].as_array().unwrap();
    let root_row = rows.iter().find(|r| r["id"] == root_id).unwrap();
    assert_eq!(root_row["budget_usd_cents"], 1_000);
    assert_eq!(root_row["spent_usd_cents"], 25);
    assert_eq!(root_row["aggregated_spent_usd_cents"], 25 + 100 + 50);
    let a_row = rows.iter().find(|r| r["id"] == a_id).unwrap();
    assert_eq!(a_row["aggregated_spent_usd_cents"], 100);
    assert_eq!(a_row["parent_subtask_id"], root_id);
    let b_row = rows.iter().find(|r| r["id"] == b_id).unwrap();
    assert_eq!(b_row["aggregated_spent_usd_cents"], 50);
    assert_eq!(b_row["parent_subtask_id"], root_id);
}

#[test]
fn budget_tree_unknown_root_prints_empty_message() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert_ok(
        &run(workspace.path(), home.path(), &["workspace", "init"]),
        "workspace init",
    );
    let out = run(
        workspace.path(),
        home.path(),
        &["subtask", "budget", "no-such-id"],
    );
    assert_ok(&out, "argo subtask budget on unknown root");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("no subtask budget tree"),
        "expected helpful empty message, got: {stdout}",
    );
}
