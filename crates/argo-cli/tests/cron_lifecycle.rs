//! End-to-end smoke for `argo cron` create/list/tick/delete.
//!
//! Spins up a fresh workspace under `TempDir` and exercises:
//!   workspace init → cron create → cron list → cron tick --dry-run
//!                  → cron delete → cron list (empty)
//!
//! Validates the on-disk layout (`cron/jobs/<name>.json`), the
//! tick lock semantics (no daemon held → tick succeeds), and the
//! "already exists" error path.
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

#[test]
fn cron_create_list_tick_delete() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let init = run(workspace.path(), home.path(), &["workspace", "init"]);
    assert!(init.status.success());

    // 1. cron create writes the job file.
    let create = run(
        workspace.path(),
        home.path(),
        &[
            "cron",
            "create",
            "morning-summary",
            "0 9 * * *",
            "Summarize yesterday",
            "slack",
            "C123",
        ],
    );
    assert!(
        create.status.success(),
        "cron create failed: stdout={} stderr={}",
        String::from_utf8_lossy(&create.stdout),
        String::from_utf8_lossy(&create.stderr)
    );
    let job_path = workspace
        .path()
        .join("cron")
        .join("jobs")
        .join("morning-summary.json");
    assert!(
        job_path.exists(),
        "expected {} to exist",
        job_path.display()
    );

    // 2. cron list shows the new job.
    let list = run(workspace.path(), home.path(), &["cron", "list"]);
    assert!(list.status.success());
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert!(stdout.contains("morning-summary"));
    assert!(stdout.contains("slack"));

    // 3. Re-creating the same job must fail with a clear error.
    let dup = run(
        workspace.path(),
        home.path(),
        &[
            "cron",
            "create",
            "morning-summary",
            "0 9 * * *",
            "noop",
            "slack",
            "C123",
        ],
    );
    assert!(!dup.status.success(), "duplicate create should have failed");
    assert!(String::from_utf8_lossy(&dup.stderr).contains("already exists"));

    // 4. cron tick --dry-run runs without firing.
    let tick = run(
        workspace.path(),
        home.path(),
        &["cron", "tick", "--dry-run"],
    );
    assert!(
        tick.status.success(),
        "tick --dry-run failed: stderr={}",
        String::from_utf8_lossy(&tick.stderr)
    );

    // 5. cron delete removes the file.
    let del = run(
        workspace.path(),
        home.path(),
        &["cron", "delete", "morning-summary"],
    );
    assert!(del.status.success());
    assert!(!job_path.exists());

    // 6. cron list now shows the empty state.
    let list2 = run(workspace.path(), home.path(), &["cron", "list"]);
    assert!(list2.status.success());
    assert!(String::from_utf8_lossy(&list2.stdout).contains("(no cron jobs"));
}
