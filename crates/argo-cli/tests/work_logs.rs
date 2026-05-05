//! Integration test for `argo work logs <job_id>` (#751).
//!
//! Drives the full flow: queue + serve a mock-mode job, then
//! verify the new `work logs` command surfaces the job's
//! artifact files without the operator needing to know the
//! `.skycot/outputs/<job_id>/...` disk layout.

mod common;

use common::argo_bin;
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

fn fresh_workspace_with_job() -> (tempfile::TempDir, tempfile::TempDir, String) {
    let home = tempfile::tempdir().expect("home");
    let work = tempfile::tempdir().expect("work");
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
            "logs test",
            "--json",
        ],
    );
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
    assert!(serve.status.success());
    let serve_out = String::from_utf8_lossy(&serve.stdout).to_string();
    let job_id = serve_out
        .lines()
        .find_map(|l| {
            let v: serde_json::Value = serde_json::from_str(l).ok()?;
            v["job_id"].as_str().map(String::from)
        })
        .expect("job_id in serve output");
    (home, work, job_id)
}

#[test]
fn work_logs_default_surfaces_transcript_for_a_completed_job() {
    let (home, work, job_id) = fresh_workspace_with_job();
    let out = run_argo(home.path(), work.path(), &["work", "logs", &job_id]);
    assert!(
        out.status.success(),
        "logs: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body = String::from_utf8_lossy(&out.stdout);
    // Mock provider's transcript includes the deterministic
    // synthetic-response marker — proves we got the right file.
    assert!(
        body.contains("deterministic synthetic response"),
        "expected mock transcript; got:\n{body}"
    );
}

#[test]
fn work_logs_kind_assistant_surfaces_assistant_output() {
    let (home, work, job_id) = fresh_workspace_with_job();
    let out = run_argo(
        home.path(),
        work.path(),
        &["work", "logs", &job_id, "--kind", "assistant"],
    );
    assert!(out.status.success());
    let body = String::from_utf8_lossy(&out.stdout);
    assert!(
        body.contains("[STOP: TASK_COMPLETE]"),
        "assistant_output should contain mock's stop marker; got:\n{body}"
    );
}

#[test]
fn work_logs_kind_all_prints_headers_per_file() {
    let (home, work, job_id) = fresh_workspace_with_job();
    let out = run_argo(
        home.path(),
        work.path(),
        &["work", "logs", &job_id, "--kind", "all"],
    );
    assert!(out.status.success());
    let body = String::from_utf8_lossy(&out.stdout);
    for f in [
        "transcript.md",
        "assistant_output.md",
        "approval_checklist.md",
        "execution_plan.md",
    ] {
        assert!(
            body.contains(&format!("===== {f}")),
            "kind=all should print a header for {f}; got:\n{body}"
        );
    }
}

#[test]
fn work_logs_unknown_job_id_fails_clearly() {
    let (home, work, _) = fresh_workspace_with_job();
    let out = run_argo(
        home.path(),
        work.path(),
        &["work", "logs", "00000000-0000-0000-0000-000000000000"],
    );
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("no job with job_id"));
}
