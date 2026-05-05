//! End-to-end CLI tests for `argo worker actions {list, show}`
//! (#728). Drives the binary so the surface is exercised the
//! way operators use it. Mock-mode only — no API key required.
//!
//! Coverage:
//!   - `list` returns every persisted action across all loaded
//!     verticals when no filter is set
//!   - `--state pending|approved|...` filters correctly
//!   - `--job-id` filters correctly
//!   - `--action-type` filters correctly
//!   - When the same action_id has multiple state-rows, the
//!     LATEST snapshot wins (matches the append-only state-
//!     machine contract from #728)
//!   - `show <action_id>` returns the latest snapshot
//!   - `show` with an unknown action_id fails clearly

mod common;

use common::argo_bin;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn fresh_workspace() -> (tempfile::TempDir, tempfile::TempDir) {
    let home = tempfile::tempdir().expect("home tempdir");
    let work = tempfile::tempdir().expect("work tempdir");
    let init = Command::new(argo_bin())
        .arg("workspace")
        .arg("init")
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", workspace_root().join("verticals"))
        .current_dir(work.path())
        .output()
        .expect("workspace init");
    assert!(
        init.status.success(),
        "workspace init failed: {}",
        String::from_utf8_lossy(&init.stderr)
    );
    (home, work)
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

/// Persist a `worker_action` row by piggy-backing on
/// `argo memory append --vertical gtm-engineer memory '{...}'`.
/// The action ledger persists actions through the same memory
/// surface as `pending_jobs` / `completed_jobs`, so this is a
/// faithful synthetic of what #729 will write programmatically.
#[allow(clippy::too_many_arguments)]
fn append_action(
    home: &Path,
    work: &Path,
    action_id: &str,
    job_id: &str,
    action_type: &str,
    state: &str,
    platform: Option<&str>,
    created_at: &str,
) {
    let body = json!({
        "schema_key": "worker_action",
        "action_id": action_id,
        "job_id": job_id,
        "action_type": action_type,
        "state": state,
        "payload": {"title": "Pro skates", "body": "lightly used"},
        "platform": platform,
        "evidence": ["https://example.com/comp"],
        "created_at": created_at,
    })
    .to_string();
    let out = run_argo(
        home,
        work,
        &[
            "memory",
            "append",
            "--vertical",
            "gtm-engineer",
            "memory",
            &body,
        ],
    );
    assert!(
        out.status.success(),
        "memory append failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn list_json(home: &Path, work: &Path, args: &[&str]) -> Vec<Value> {
    let mut full = vec!["worker", "actions", "list", "--json"];
    full.extend_from_slice(args);
    let out = run_argo(home, work, &full);
    assert!(
        out.status.success(),
        "worker actions list failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

// ─────────────────────────────────────────────── tests

#[test]
fn list_returns_persisted_actions_across_all_verticals_by_default() {
    let (home, work) = fresh_workspace();
    append_action(
        home.path(),
        work.path(),
        "act-001",
        "job-A",
        "create_listing_draft",
        "pending",
        Some("facebook_marketplace"),
        "2026-05-05T15:00:00Z",
    );
    append_action(
        home.path(),
        work.path(),
        "act-002",
        "job-A",
        "post_listing",
        "pending",
        Some("ebay"),
        "2026-05-05T15:01:00Z",
    );

    let rows = list_json(home.path(), work.path(), &[]);
    assert_eq!(rows.len(), 2);
    let ids: Vec<&str> = rows
        .iter()
        .map(|r| r["action_id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"act-001"));
    assert!(ids.contains(&"act-002"));
}

#[test]
fn list_filters_by_state_correctly() {
    let (home, work) = fresh_workspace();
    append_action(
        home.path(),
        work.path(),
        "a-pending",
        "job-A",
        "create_listing_draft",
        "pending",
        None,
        "2026-05-05T15:00:00Z",
    );
    append_action(
        home.path(),
        work.path(),
        "a-approved",
        "job-A",
        "post_listing",
        "approved",
        Some("ebay"),
        "2026-05-05T15:00:00Z",
    );
    append_action(
        home.path(),
        work.path(),
        "a-completed",
        "job-A",
        "create_listing_draft",
        "completed",
        None,
        "2026-05-05T15:00:00Z",
    );

    let pending = list_json(home.path(), work.path(), &["--state", "pending"]);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0]["action_id"], "a-pending");

    let approved = list_json(home.path(), work.path(), &["--state", "approved"]);
    assert_eq!(approved.len(), 1);
    assert_eq!(approved[0]["action_id"], "a-approved");

    let completed = list_json(home.path(), work.path(), &["--state", "completed"]);
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0]["action_id"], "a-completed");

    let all = list_json(home.path(), work.path(), &["--state", "all"]);
    assert_eq!(all.len(), 3);
}

#[test]
fn list_filters_by_job_id_and_action_type() {
    let (home, work) = fresh_workspace();
    append_action(
        home.path(),
        work.path(),
        "a",
        "job-A",
        "verify_price",
        "pending",
        None,
        "2026-05-05T15:00:00Z",
    );
    append_action(
        home.path(),
        work.path(),
        "b",
        "job-B",
        "verify_price",
        "pending",
        None,
        "2026-05-05T15:00:00Z",
    );
    append_action(
        home.path(),
        work.path(),
        "c",
        "job-A",
        "post_listing",
        "pending",
        None,
        "2026-05-05T15:00:00Z",
    );

    let job_a = list_json(home.path(), work.path(), &["--job-id", "job-A"]);
    assert_eq!(job_a.len(), 2);

    let verify = list_json(home.path(), work.path(), &["--action-type", "verify_price"]);
    assert_eq!(verify.len(), 2);

    let combined = list_json(
        home.path(),
        work.path(),
        &["--job-id", "job-A", "--action-type", "verify_price"],
    );
    assert_eq!(combined.len(), 1);
    assert_eq!(combined[0]["action_id"], "a");
}

/// State-machine snapshot semantics: when an action transitions
/// pending → approved → completed, three rows with the same
/// action_id land in the memory store. `list` must only show
/// the LATEST one. This locks the append-only contract against
/// regressions where `list` would either show all 3 or pick
/// the wrong row.
#[test]
fn list_returns_latest_snapshot_when_action_has_multiple_state_rows() {
    let (home, work) = fresh_workspace();
    let id = "act-multi";
    append_action(
        home.path(),
        work.path(),
        id,
        "job-A",
        "post_listing",
        "pending",
        None,
        "2026-05-05T15:00:00Z",
    );
    append_action(
        home.path(),
        work.path(),
        id,
        "job-A",
        "post_listing",
        "approved",
        None,
        "2026-05-05T15:01:00Z",
    );
    append_action(
        home.path(),
        work.path(),
        id,
        "job-A",
        "post_listing",
        "completed",
        None,
        "2026-05-05T15:02:00Z",
    );

    let rows = list_json(home.path(), work.path(), &[]);
    assert_eq!(rows.len(), 1, "exactly one row per action_id");
    assert_eq!(rows[0]["action_id"], id);
    assert_eq!(
        rows[0]["state"], "completed",
        "latest snapshot wins; got: {}",
        rows[0]
    );
}

#[test]
fn show_returns_latest_snapshot_for_known_action() {
    let (home, work) = fresh_workspace();
    let id = "act-show";
    append_action(
        home.path(),
        work.path(),
        id,
        "job-A",
        "post_listing",
        "pending",
        Some("ebay"),
        "2026-05-05T15:00:00Z",
    );
    append_action(
        home.path(),
        work.path(),
        id,
        "job-A",
        "post_listing",
        "approved",
        Some("ebay"),
        "2026-05-05T15:01:00Z",
    );

    let out = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "show", id, "--json"],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let payload: Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(payload["action_id"], id);
    assert_eq!(payload["state"], "approved");
    assert_eq!(payload["platform"], "ebay");
    assert_eq!(payload["created_at"], "2026-05-05T15:01:00Z");
}

#[test]
fn show_unknown_action_id_fails_clearly() {
    let (home, work) = fresh_workspace();
    let out = run_argo(
        home.path(),
        work.path(),
        &[
            "worker",
            "actions",
            "show",
            "00000000-0000-7000-8000-000000000000",
        ],
    );
    assert!(
        !out.status.success(),
        "expected non-zero exit for unknown action_id"
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert!(
        combined.contains("no action") || combined.contains("00000000"),
        "expected clear error referencing the unknown id; got:\n{combined}"
    );
}

#[test]
fn list_with_no_actions_prints_friendly_empty_marker_in_text_mode() {
    let (home, work) = fresh_workspace();
    let out = run_argo(home.path(), work.path(), &["worker", "actions", "list"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("(no actions)"),
        "empty result must print a clear marker, not nothing; got:\n{stdout}"
    );
}
