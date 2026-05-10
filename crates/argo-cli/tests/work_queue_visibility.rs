//! Visibility / cross-workspace regression tests for the work
//! queue v0 (#717, #718). Split out of `work_queue.rs` so each
//! file stays under the 500-LOC ceiling. These tests exist to
//! lock down the bugs the user reported when:
//!
//!   - `argo memory list --vertical` resolved against
//!     `first_workspace()` instead of the cwd's workspace, so
//!     pending/completed rows written from a newer cwd were
//!     invisible across the CLI surface (#717), and
//!   - read-side handlers shared the same first-workspace bias,
//!     so a second workspace's writes leaked under a stale
//!     workspace's reads (#718).
//!
//! Core happy-path / error-path coverage stays in `work_queue.rs`.

mod common;

use common::work_queue_helpers::{
    fresh_workspace, list_jobs_by_schema, run_argo, workspace_root,
};
use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::Command;

/// #717 regression: drives the EXACT CLI sequence the user reported
/// broken — `argo memory list --vertical gtm-engineer --json` must
/// surface both the pending and completed job rows that `work add`
/// and `work serve --once` write. Also asserts that
/// `argo session show` renders non-empty first/last message snippets
/// so work-created sessions display useful content. Pre-#717 this
/// test would fail on two counts: (a) memory list resolved
/// `first_workspace()` instead of the cwd's workspace (so the rows
/// were invisible across the CLI surface), and (b) work handler
/// stored message content as raw strings instead of
/// `[{"type":"text","text":"..."}]` typed blocks (so session show's
/// snippet extractor returned empty).
#[test]
fn work_queue_rows_visible_via_memory_list_and_session_show() {
    let (home, work) = fresh_workspace();

    // Step 1: enqueue a job — exact prompt from the user's report.
    let add = run_argo(
        home.path(), work.path(),
        &["work", "add",
          "--vertical", "gtm-engineer",
          "--prompt", "Analyze this weak campaign: 100 emails, 2 replies, 0 positive. Recommend the next GTM move.",
          "--json"],
    );
    assert!(
        add.status.success(),
        "{}",
        String::from_utf8_lossy(&add.stderr)
    );
    let added: Value = serde_json::from_slice(&add.stdout).expect("json");
    let job_id = added["job_id"].as_str().expect("job_id").to_string();

    // Step 2: BEFORE serve — `memory list --vertical` MUST show the
    // pending row (this is the regression check vs first_workspace()).
    let pre = list_jobs_by_schema(home.path(), work.path(), "gtm-engineer", "pending_jobs");
    assert_eq!(
        pre.len(),
        1,
        "memory list --vertical gtm-engineer must surface the pending job; got {pre:?}"
    );
    assert_eq!(pre[0]["vertical_id"], "gtm-engineer");
    assert_eq!(pre[0]["status"], "pending");

    // Step 3: serve --once → completed row written.
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
        "{}",
        String::from_utf8_lossy(&serve.stderr)
    );
    let processed: Value = serde_json::from_slice(&serve.stdout).expect("json");
    let session_id = processed["session_id"]
        .as_str()
        .expect("session_id")
        .to_string();
    let assistant_id = processed["assistant_id"]
        .as_str()
        .expect("assistant_id")
        .to_string();

    // Step 4: AFTER serve — completed row must surface with all the
    // fields the user listed as required.
    let post = list_jobs_by_schema(home.path(), work.path(), "gtm-engineer", "completed_jobs");
    assert_eq!(
        post.len(),
        1,
        "expected one completed_jobs row; got {post:?}"
    );
    assert_eq!(post[0]["vertical_id"], "gtm-engineer");
    assert_eq!(post[0]["pending_job_id"], job_id);
    assert_eq!(post[0]["session_id"], session_id);
    assert_eq!(post[0]["assistant_id"], assistant_id);
    let asst_text = post[0]["assistant_text"].as_str().expect("assistant_text");
    assert!(!asst_text.is_empty(), "assistant_text must be persisted");
    assert!(
        post[0]["completed_at"].as_str().is_some(),
        "completed_at required"
    );

    // Step 5: argo session show — both first (user) and last
    // (assistant) bodies must extract a non-empty snippet.
    let show = run_argo(home.path(), work.path(), &["session", "show", &session_id]);
    assert!(
        show.status.success(),
        "{}",
        String::from_utf8_lossy(&show.stderr)
    );
    let stdout = String::from_utf8_lossy(&show.stdout);
    let first_line = stdout
        .lines()
        .find(|l| l.trim_start().starts_with("first ("))
        .unwrap_or_else(|| panic!("session show missing 'first (' line; got:\n{stdout}"));
    let last_line = stdout
        .lines()
        .find(|l| l.trim_start().starts_with("last  ("))
        .unwrap_or_else(|| panic!("session show missing 'last  (' line; got:\n{stdout}"));

    // The colon delimits role from snippet — anything after must be
    // non-empty text. Pre-fix this would be `first (user):` (empty
    // tail); post-fix it should carry a chunk of the prompt / mock
    // response.
    let first_snippet = first_line
        .split_once("):")
        .map(|(_, s)| s.trim())
        .unwrap_or("");
    let last_snippet = last_line
        .split_once("):")
        .map(|(_, s)| s.trim())
        .unwrap_or("");
    assert!(
        !first_snippet.is_empty(),
        "session show first (user) snippet must be non-empty; full output:\n{stdout}"
    );
    assert!(
        !last_snippet.is_empty(),
        "session show last (assistant) snippet must be non-empty; full output:\n{stdout}"
    );
    assert!(
        first_snippet.contains("100 emails") || first_snippet.contains("Analyze"),
        "first snippet should echo the prompt; got: {first_snippet:?}"
    );
}

/// #718 regression: prove the cwd-resolved workspace isolation
/// across multiple workspaces sharing one ARGO_HOME (the
/// real-world scenario the user hit). Pre-#718, all read-side
/// handlers used `store.first_workspace()` which returns whichever
/// workspace was inserted FIRST — so writes from a newer cwd
/// vanished from `memory list --vertical`. After #718 every
/// reader resolves to the same workspace its cwd-mate writers
/// already use.
#[test]
fn cwd_workspace_isolates_reads_across_multiple_workspaces() {
    let home = tempfile::tempdir().expect("home tempdir");
    let stale_cwd = tempfile::tempdir().expect("stale cwd tempdir");
    let active_cwd = tempfile::tempdir().expect("active cwd tempdir");

    let env = |cmd: &mut Command, cwd: &Path| {
        cmd.env("ARGO_HOME", home.path())
            .env("ARGO_VERTICALS_DIR", workspace_root().join("verticals"))
            .current_dir(cwd);
    };

    // Bootstrap workspace 1 (the "stale" one). It carries a
    // gtm-engineer pending_jobs entry that should NOT leak into
    // reads from the second workspace's cwd.
    let mut c = Command::new(argo_bin());
    c.arg("workspace").arg("init");
    env(&mut c, stale_cwd.path());
    assert!(c.output().expect("init1").status.success());

    let mut c = Command::new(argo_bin());
    c.args([
        "work",
        "add",
        "--vertical",
        "gtm-engineer",
        "--prompt",
        "stale-job-should-not-leak",
    ]);
    env(&mut c, stale_cwd.path());
    assert!(c.output().expect("stale work add").status.success());

    // Bootstrap workspace 2 (the "active" one) under the same
    // ARGO_HOME. Different cwd → different workspace_id (lookup
    // by root_path inside ensure_workspace).
    let mut c = Command::new(argo_bin());
    c.arg("workspace").arg("init");
    env(&mut c, active_cwd.path());
    assert!(c.output().expect("init2").status.success());

    let mut c = Command::new(argo_bin());
    c.args([
        "work",
        "add",
        "--vertical",
        "gtm-engineer",
        "--prompt",
        "active-job-should-be-visible",
    ]);
    env(&mut c, active_cwd.path());
    assert!(c.output().expect("active work add").status.success());

    // Read from active_cwd. After #718 the cwd-resolved workspace
    // is workspace 2; only its single pending_jobs row should
    // surface — workspace 1's "stale-job-should-not-leak" must
    // not appear.
    let pending = list_jobs_by_schema(
        home.path(),
        active_cwd.path(),
        "gtm-engineer",
        "pending_jobs",
    );
    assert_eq!(
        pending.len(),
        1,
        "active cwd should see exactly its own pending row; got {pending:?}"
    );
    assert_eq!(pending[0]["prompt"], "active-job-should-be-visible");

    // Symmetric check: from stale_cwd we see only workspace 1's row.
    let stale_pending = list_jobs_by_schema(
        home.path(),
        stale_cwd.path(),
        "gtm-engineer",
        "pending_jobs",
    );
    assert_eq!(stale_pending.len(), 1);
    assert_eq!(stale_pending[0]["prompt"], "stale-job-should-not-leak");
}
