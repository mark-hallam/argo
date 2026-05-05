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

/// gtm-engineer's `verify_price` declares `max_attempts = 3`
/// (#755). Walk a verify_price action through the full retry
/// budget, verify the limit holds, and verify `--force`
/// overrides — each behaviour locked end-to-end through the
/// binary.
#[test]
fn retry_enforces_max_attempts_and_force_overrides() {
    let (home, work) = fresh_workspace_with_actions();

    // Helper: walk an action_id pending → executing → failed
    // so it's a valid retry source.
    let walk_to_failed = |id: &str| {
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
                "transient",
                "--json",
            ],
        );
    };

    // Original verify_price (attempt #1).
    let rows = list_actions(home.path(), work.path());
    let verify = rows
        .iter()
        .find(|r| r["action_type"] == "verify_price")
        .expect("verify_price row");
    let id1 = verify["action_id"].as_str().expect("action_id").to_string();
    walk_to_failed(&id1);

    // First retry → attempt #2 (within max_attempts=3).
    let r1 = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "retry", &id1, "--json"],
    );
    assert!(
        r1.status.success(),
        "retry #1 should succeed: {}",
        String::from_utf8_lossy(&r1.stderr)
    );
    let v1: Value = serde_json::from_slice(&r1.stdout).expect("json");
    assert_eq!(v1["retry_attempt"], 2);
    assert_eq!(v1["max_attempts"], 3);
    assert_eq!(v1["forced"], false);
    let id2 = v1["new_action"]["action_id"]
        .as_str()
        .expect("id2")
        .to_string();

    // Walk #2 to failed, then retry → attempt #3 (still within
    // max_attempts=3).
    walk_to_failed(&id2);
    let r2 = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "retry", &id2, "--json"],
    );
    assert!(
        r2.status.success(),
        "retry #2 should succeed: {}",
        String::from_utf8_lossy(&r2.stderr)
    );
    let v2: Value = serde_json::from_slice(&r2.stdout).expect("json");
    assert_eq!(v2["retry_attempt"], 3);
    let id3 = v2["new_action"]["action_id"]
        .as_str()
        .expect("id3")
        .to_string();

    // Walk #3 to failed, then attempt #4 — this would breach
    // max_attempts=3, so retry MUST refuse without --force.
    walk_to_failed(&id3);
    let r3 = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "retry", &id3, "--json"],
    );
    assert!(
        !r3.status.success(),
        "retry #3 (attempt #4) must be refused"
    );
    let stderr = String::from_utf8_lossy(&r3.stderr);
    assert!(
        stderr.contains("max_attempts = 3"),
        "stderr should explain the policy; got: {stderr}"
    );
    assert!(
        stderr.contains("--force"),
        "stderr should mention --force override; got: {stderr}"
    );

    // --force lets the operator override the cap. Logs to
    // stderr; succeeds; sets `forced: true` in the JSON.
    let r3f = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "retry", &id3, "--force", "--json"],
    );
    assert!(
        r3f.status.success(),
        "retry --force should succeed: {}",
        String::from_utf8_lossy(&r3f.stderr)
    );
    let v3f: Value = serde_json::from_slice(&r3f.stdout).expect("json");
    assert_eq!(v3f["retry_attempt"], 4);
    assert_eq!(v3f["forced"], true);
    let stderr_f = String::from_utf8_lossy(&r3f.stderr);
    assert!(
        stderr_f.contains("WARN"),
        "--force must log a WARN breadcrumb for audit; got: {stderr_f}"
    );
}

/// `argo worker actions show <id> --json` surfaces the retry
/// counters so monitoring + dashboards don't have to re-parse
/// the evidence chain or re-load the manifest.
#[test]
fn show_json_carries_retry_attempt_and_max_attempts() {
    let (home, work) = fresh_workspace_with_actions();
    let rows = list_actions(home.path(), work.path());
    let verify = rows
        .iter()
        .find(|r| r["action_type"] == "verify_price")
        .expect("verify_price row");
    let id = verify["action_id"].as_str().expect("action_id");

    let show = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "show", id, "--json"],
    );
    assert!(show.status.success());
    let v: Value = serde_json::from_slice(&show.stdout).expect("json");
    // Original action: attempt 1 of 3 (verify_price's manifest cap).
    assert_eq!(v["retry_attempt"], 1);
    assert_eq!(v["retry_max_attempts"], 3);
}
