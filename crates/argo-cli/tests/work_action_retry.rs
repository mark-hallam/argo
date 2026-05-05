//! End-to-end test for `argo worker actions retry <id>` (#737).
//!
//! Walks an action through pending → executing → failed via the
//! binary, then retries it and asserts:
//!
//!   1. `retry` on a missing id fails with a clear error.
//!   2. `retry` on a failed id emits a NEW pending row whose
//!      action_id differs from the source.
//!   3. The new row's `evidence` array references the original
//!      action via `argo://action/<failed-id>`.
//!   4. `retry` on a non-failed action (e.g. still pending)
//!      refuses with a "not 'failed'" error.
//!
//! Lives in its own file so `work_action_transitions.rs` stays
//! under the per-file 500-LOC budget.

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
    assert!(init.status.success());
    let _add = run_argo(
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
    let _serve = run_argo(
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
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

#[test]
fn retry_emits_new_pending_action_referencing_failed_one() {
    let (home, work) = fresh_workspace_with_actions();
    let rows = list_actions(home.path(), work.path());
    let verify = rows
        .iter()
        .find(|r| r["action_type"] == "verify_price")
        .expect("verify_price row");
    let id = verify["action_id"].as_str().expect("action_id");

    // Walk verify_price into failed: pending → executing → failed.
    let _ = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "execute", id, "--json"],
    );
    let _ = run_argo(
        home.path(),
        work.path(),
        &[
            "worker",
            "actions",
            "fail",
            id,
            "--reason",
            "lookup timed out",
            "--json",
        ],
    );

    // Retry on a missing id must fail with a clear error.
    let bogus = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "retry", "missing", "--json"],
    );
    assert!(!bogus.status.success());
    assert!(String::from_utf8_lossy(&bogus.stderr).contains("missing"));

    // Retry the failed verify_price action.
    let retry = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "retry", id, "--json"],
    );
    assert!(
        retry.status.success(),
        "retry: {}",
        String::from_utf8_lossy(&retry.stderr)
    );
    let v: Value = serde_json::from_slice(&retry.stdout).expect("retry json");
    assert_eq!(v["retried_from"], id);
    let new_id = v["new_action"]["action_id"]
        .as_str()
        .expect("new action_id");
    assert_ne!(new_id, id);
    assert_eq!(v["new_action"]["state"], "pending");
    assert_eq!(v["new_action"]["action_type"], "verify_price");
    let evidence = v["new_action"]["evidence"]
        .as_array()
        .expect("evidence array");
    assert!(
        evidence
            .iter()
            .any(|e| e.as_str().map(|s| s.contains(id)).unwrap_or(false)),
        "new action's evidence should reference the failed id; got: {evidence:?}"
    );

    // The ledger now has an additional pending row.
    let rows_after = list_actions(home.path(), work.path());
    assert!(
        rows_after.iter().any(|r| r["action_id"] == new_id
            && r["state"] == "pending"
            && r["action_type"] == "verify_price"),
        "new pending row missing from ledger; got: {rows_after:?}"
    );

    // Retry on a non-failed (currently pending) action must refuse.
    let pl = rows_after
        .iter()
        .find(|r| r["action_type"] == "post_listing")
        .expect("post_listing row");
    let pl_id = pl["action_id"].as_str().expect("post_id");
    let non_failed = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "retry", pl_id, "--json"],
    );
    assert!(
        !non_failed.status.success(),
        "retry must refuse non-failed source"
    );
    assert!(String::from_utf8_lossy(&non_failed.stderr).contains("not 'failed'"));
}
