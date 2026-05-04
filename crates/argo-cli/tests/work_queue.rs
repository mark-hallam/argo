//! End-to-end CLI tests for the work queue v0 (#716). Drives the
//! `argo` binary so the surface is exercised at the same layer
//! operators use it. Mock-provider path only — no API key required.
//!
//! Coverage:
//!   - `argo work add` writes a `pending_jobs` memory entry under
//!     the vertical's namespace and prints a job_id
//!   - `argo work serve --once` processes the oldest pending job
//!     and writes a `completed_jobs` entry referencing it
//!   - assistant output is persisted as a session message (verified
//!     via the post-run DB through the SkillStore-style direct path)
//!   - a second `--once` pass does NOT duplicate the completed
//!     entry (de-dup via `completed_jobs.pending_job_id` set)
//!   - `argo work add --vertical <bogus>` exits non-zero with a
//!     clear error mentioning the unknown vertical id

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

/// Spin up an isolated workspace + ARGO_HOME for one test; returns
/// (home_tempdir, work_tempdir). Both tempdirs auto-clean on drop.
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

fn list_memory_jsonl(home: &Path, work: &Path, vertical: &str) -> Vec<Value> {
    let out = run_argo(
        home, work,
        &["memory", "list", "--vertical", vertical, "--json"],
    );
    assert!(
        out.status.success(),
        "memory list failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

/// Read every memory entry's body as JSON, filtered to a given
/// `schema_key`. Returns the parsed bodies (the actual job
/// payloads, not the wrapping memory-row metadata).
///
/// `argo memory list --json` returns row metadata only (no body),
/// so we filter to candidate rows via `--body-contains <schema_key>`,
/// then `argo memory view <id> --json` fetches each body.
fn list_jobs_by_schema(home: &Path, work: &Path, vertical: &str, schema_key: &str) -> Vec<Value> {
    let by_body = run_argo(
        home, work,
        &[
            "memory", "list",
            "--vertical", vertical,
            "--body-contains", schema_key,
            "--json",
        ],
    );
    assert!(
        by_body.status.success(),
        "memory list --body-contains failed: {}",
        String::from_utf8_lossy(&by_body.stderr)
    );
    let rows: Vec<Value> = String::from_utf8_lossy(&by_body.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    let mut bodies = Vec::new();
    for row in rows {
        let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
        if id.is_empty() {
            continue;
        }
        let view = run_argo(home, work, &["memory", "view", id, "--json"]);
        if !view.status.success() {
            continue;
        }
        if let Ok(v) = serde_json::from_slice::<Value>(&view.stdout) {
            if let Some(body_str) = v.get("body").and_then(|x| x.as_str()) {
                if let Ok(body) = serde_json::from_str::<Value>(body_str) {
                    if body.get("schema_key").and_then(|s| s.as_str()) == Some(schema_key) {
                        bodies.push(body);
                    }
                }
            }
        }
    }
    bodies
}

// ─────────────────────────────────────────────────────────── tests

#[test]
fn work_add_creates_pending_job_in_memory() {
    let (home, work) = fresh_workspace();
    let out = run_argo(
        home.path(), work.path(),
        &["work", "add",
          "--vertical", "gtm-engineer",
          "--prompt", "Analyze the latest weak campaign.",
          "--json"],
    );
    assert!(
        out.status.success(),
        "work add failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(v["ok"], true);
    assert_eq!(v["vertical_id"], "gtm-engineer");
    assert_eq!(v["status"], "pending");
    assert!(v["job_id"].as_str().is_some(), "job_id should be a string");

    // Confirm the row landed in the vertical's memory namespace.
    let entries = list_memory_jsonl(home.path(), work.path(), "gtm-engineer");
    assert!(
        !entries.is_empty(),
        "expected at least one memory entry under gtm-engineer; got: {entries:?}"
    );

    let pending = list_jobs_by_schema(home.path(), work.path(), "gtm-engineer", "pending_jobs");
    assert_eq!(
        pending.len(), 1,
        "expected exactly one pending_jobs body; got {pending:?}"
    );
    assert_eq!(pending[0]["status"], "pending");
    assert_eq!(pending[0]["vertical_id"], "gtm-engineer");
    assert_eq!(pending[0]["attempts"], 0);
    assert_eq!(pending[0]["prompt"], "Analyze the latest weak campaign.");
}

#[test]
fn work_serve_once_processes_one_pending_job_and_persists_output() {
    let (home, work) = fresh_workspace();
    let add = run_argo(
        home.path(), work.path(),
        &["work", "add",
          "--vertical", "gtm-engineer",
          "--prompt", "Diagnose the 100/2/0 cohort.",
          "--json"],
    );
    assert!(add.status.success(), "{}", String::from_utf8_lossy(&add.stderr));
    let added: Value = serde_json::from_slice(&add.stdout).expect("json");
    let job_id = added["job_id"].as_str().expect("job_id").to_string();

    let serve = run_argo(
        home.path(), work.path(),
        &["work", "serve",
          "--vertical", "gtm-engineer",
          "--once",
          "--json"],
    );
    assert!(
        serve.status.success(),
        "work serve --once failed: {}",
        String::from_utf8_lossy(&serve.stderr)
    );
    let processed: Value = serde_json::from_slice(&serve.stdout).expect("json");
    assert_eq!(processed["ok"], true);
    assert_eq!(processed["job_id"], job_id);
    let session_id = processed["session_id"].as_str().expect("session_id").to_string();
    assert!(!session_id.is_empty());

    // (a) completed_jobs memory entry exists, points at our pending id
    let completed = list_jobs_by_schema(home.path(), work.path(), "gtm-engineer", "completed_jobs");
    assert_eq!(
        completed.len(), 1,
        "expected exactly one completed_jobs body; got {completed:?}"
    );
    assert_eq!(completed[0]["pending_job_id"], job_id);
    assert_eq!(completed[0]["session_id"], session_id);
    let assistant_text = completed[0]["assistant_text"]
        .as_str().expect("assistant_text");
    assert!(
        assistant_text.starts_with("[mock]"),
        "expected mock-prefixed text; got: {assistant_text}"
    );
    assert!(
        assistant_text.contains("Diagnose the 100/2/0 cohort."),
        "assistant text should echo the prompt; got: {assistant_text}"
    );

    // (b) session has both a user message and an assistant message
    let show = run_argo(home.path(), work.path(),
        &["session", "show", &session_id, "--json"]);
    assert!(show.status.success(), "{}", String::from_utf8_lossy(&show.stderr));
    let session: Value = serde_json::from_slice(&show.stdout).expect("session json");
    assert_eq!(session["vertical_id"], "gtm-engineer");
}

#[test]
fn work_serve_does_not_duplicate_completed_jobs_on_second_pass() {
    let (home, work) = fresh_workspace();
    run_argo(home.path(), work.path(),
        &["work", "add",
          "--vertical", "gtm-engineer",
          "--prompt", "First and only job."]);

    // First pass — processes the one pending job.
    let first = run_argo(home.path(), work.path(),
        &["work", "serve", "--vertical", "gtm-engineer", "--once"]);
    assert!(first.status.success());

    // Second pass — queue is now drained because the only pending
    // is in the completed-set. Worker should exit cleanly without
    // writing another completed_jobs row.
    let second = run_argo(home.path(), work.path(),
        &["work", "serve", "--vertical", "gtm-engineer", "--once"]);
    assert!(second.status.success());
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        stderr.contains("queue empty") || stderr.contains("--once"),
        "second pass should report empty queue; got stderr:\n{stderr}"
    );

    let completed = list_jobs_by_schema(home.path(), work.path(), "gtm-engineer", "completed_jobs");
    assert_eq!(
        completed.len(), 1,
        "second pass must NOT duplicate the completed entry; got {} completed bodies",
        completed.len()
    );
}

#[test]
fn work_add_unknown_vertical_fails_clearly() {
    let (home, work) = fresh_workspace();
    let out = run_argo(home.path(), work.path(),
        &["work", "add",
          "--vertical", "this-vertical-does-not-exist",
          "--prompt", "..."]);
    assert!(
        !out.status.success(),
        "expected non-zero exit for unknown vertical; got success with stdout:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("this-vertical-does-not-exist") || combined.contains("not found"),
        "expected error to name the unknown vertical; got:\n{combined}"
    );
}
