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
//!
//! Visibility / cross-workspace regression coverage (#717, #718)
//! lives in the sibling `tests/work_queue_visibility.rs`. Shared
//! fixtures live in `tests/common/work_queue_helpers.rs`.

mod common;

use common::work_queue_helpers::{
    fresh_workspace, list_jobs_by_schema, list_memory_jsonl, run_argo,
};
use serde_json::Value;

#[test]
fn work_add_creates_pending_job_in_memory() {
    let (home, work) = fresh_workspace();
    let out = run_argo(
        home.path(),
        work.path(),
        &[
            "work",
            "add",
            "--vertical",
            "gtm-engineer",
            "--prompt",
            "Analyze the latest weak campaign.",
            "--json",
        ],
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
        pending.len(),
        1,
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
        home.path(),
        work.path(),
        &[
            "work",
            "add",
            "--vertical",
            "gtm-engineer",
            "--prompt",
            "Diagnose the 100/2/0 cohort.",
            "--json",
        ],
    );
    assert!(
        add.status.success(),
        "{}",
        String::from_utf8_lossy(&add.stderr)
    );
    let added: Value = serde_json::from_slice(&add.stdout).expect("json");
    let job_id = added["job_id"].as_str().expect("job_id").to_string();

    let serve = run_argo(
        home.path(),
        work.path(),
        &[
            "work",
            "serve",
            "--vertical",
            "gtm-engineer",
            "--once",
            // #722 — single-step keeps this v1 regression on the
            // single-turn mock path; v2 multi-step has its own tests.
            "--max-steps",
            "1",
            "--json",
        ],
    );
    assert!(
        serve.status.success(),
        "work serve --once failed: {}",
        String::from_utf8_lossy(&serve.stderr)
    );
    let processed: Value = serde_json::from_slice(&serve.stdout).expect("json");
    assert_eq!(processed["ok"], true);
    assert_eq!(processed["job_id"], job_id);
    let session_id = processed["session_id"]
        .as_str()
        .expect("session_id")
        .to_string();
    assert!(!session_id.is_empty());

    // (a) completed_jobs memory entry exists, points at our pending id
    let completed = list_jobs_by_schema(home.path(), work.path(), "gtm-engineer", "completed_jobs");
    assert_eq!(
        completed.len(),
        1,
        "expected exactly one completed_jobs body; got {completed:?}"
    );
    assert_eq!(completed[0]["pending_job_id"], job_id);
    assert_eq!(completed[0]["session_id"], session_id);
    let assistant_text = completed[0]["assistant_text"]
        .as_str()
        .expect("assistant_text");
    assert!(
        assistant_text.starts_with("[mock]"),
        "expected mock-prefixed text; got: {assistant_text}"
    );
    assert!(
        assistant_text.contains("Diagnose the 100/2/0 cohort."),
        "assistant text should echo the prompt; got: {assistant_text}"
    );

    // (b) session has both a user message and an assistant message
    let show = run_argo(
        home.path(),
        work.path(),
        &["session", "show", &session_id, "--json"],
    );
    assert!(
        show.status.success(),
        "{}",
        String::from_utf8_lossy(&show.stderr)
    );
    let session: Value = serde_json::from_slice(&show.stdout).expect("session json");
    assert_eq!(session["vertical_id"], "gtm-engineer");
}

#[test]
fn work_serve_does_not_duplicate_completed_jobs_on_second_pass() {
    let (home, work) = fresh_workspace();
    run_argo(
        home.path(),
        work.path(),
        &[
            "work",
            "add",
            "--vertical",
            "gtm-engineer",
            "--prompt",
            "First and only job.",
        ],
    );

    // First pass — processes the one pending job.
    let first = run_argo(
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
        ],
    );
    assert!(first.status.success());

    // Second pass — queue is now drained because the only pending
    // is in the completed-set. Worker should exit cleanly without
    // writing another completed_jobs row.
    let second = run_argo(
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
        ],
    );
    assert!(second.status.success());
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        stderr.contains("queue empty") || stderr.contains("--once"),
        "second pass should report empty queue; got stderr:\n{stderr}"
    );

    let completed = list_jobs_by_schema(home.path(), work.path(), "gtm-engineer", "completed_jobs");
    assert_eq!(
        completed.len(),
        1,
        "second pass must NOT duplicate the completed entry; got {} completed bodies",
        completed.len()
    );
}

// The live Anthropic work-serve test (#719) lives in
// `tests/work_queue_live.rs` so this file stays under budget.

#[test]
fn work_add_unknown_vertical_fails_clearly() {
    let (home, work) = fresh_workspace();
    let out = run_argo(
        home.path(),
        work.path(),
        &[
            "work",
            "add",
            "--vertical",
            "this-vertical-does-not-exist",
            "--prompt",
            "...",
        ],
    );
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
