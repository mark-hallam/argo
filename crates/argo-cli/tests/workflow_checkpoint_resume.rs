//! End-to-end smoke for `argo workflow run/checkpoints/resume/rollback`
//! (Stream `claude-checkpoint-resume`).

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

fn write_workflow(workspace: &Path) -> std::path::PathBuf {
    let yaml = "id: ckpt-flow\n\
                tasks:\n  \
                - id: a\n    \
                worker: w\n  \
                - id: b\n    \
                worker: w\n    \
                depends_on: [a]\n  \
                - id: c\n    \
                worker: w\n    \
                depends_on: [b]\n";
    let path = workspace.join("flow.yaml");
    std::fs::write(&path, yaml).unwrap();
    path
}

fn extract_run_id(stdout: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(stdout)
        .unwrap_or_else(|e| panic!("expected JSON in stdout: {e}\n{stdout}"));
    v.get("workflow_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("no workflow_id in JSON: {v}"))
        .to_string()
}

#[test]
fn run_then_checkpoints_lists_one_row_per_task() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    run(workspace.path(), home.path(), &["workspace", "init"]);
    let flow = write_workflow(workspace.path());

    let r = run(
        workspace.path(),
        home.path(),
        &["workflow", "run", flow.to_str().unwrap(), "--json"],
    );
    assert!(r.status.success(), "run: {}", String::from_utf8_lossy(&r.stderr));
    let run_id = extract_run_id(&String::from_utf8_lossy(&r.stdout));

    let cp = run(
        workspace.path(),
        home.path(),
        &["workflow", "checkpoints", &run_id, "--json"],
    );
    assert!(
        cp.status.success(),
        "checkpoints: {}",
        String::from_utf8_lossy(&cp.stderr)
    );
    let arr: serde_json::Value = serde_json::from_slice(&cp.stdout).unwrap();
    let len = arr.as_array().map_or(0, Vec::len);
    assert_eq!(
        len, 3,
        "expected 3 checkpoints (one per task), got {len}: {arr}"
    );
}

#[test]
fn rollback_then_resume_replays_dropped_tasks() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    run(workspace.path(), home.path(), &["workspace", "init"]);
    let flow = write_workflow(workspace.path());

    // Initial run: 3 checkpoints.
    let r = run(
        workspace.path(),
        home.path(),
        &["workflow", "run", flow.to_str().unwrap(), "--json"],
    );
    assert!(r.status.success(), "run: {}", String::from_utf8_lossy(&r.stderr));
    let run_id = extract_run_id(&String::from_utf8_lossy(&r.stdout));

    // Roll back to seq=1 → only 'a' remains.
    let rb = run(
        workspace.path(),
        home.path(),
        &[
            "workflow",
            "rollback",
            &run_id,
            "--to-checkpoint",
            "1",
            "--json",
        ],
    );
    assert!(
        rb.status.success(),
        "rollback: {}",
        String::from_utf8_lossy(&rb.stderr)
    );

    // Confirm checkpoints now list 1 row.
    let cp = run(
        workspace.path(),
        home.path(),
        &["workflow", "checkpoints", &run_id, "--json"],
    );
    assert!(cp.status.success());
    let arr: serde_json::Value = serde_json::from_slice(&cp.stdout).unwrap();
    assert_eq!(arr.as_array().map_or(0, Vec::len), 1);

    // Resume re-runs 'b' and 'c' → 3 rows again, run ok.
    let res = run(
        workspace.path(),
        home.path(),
        &["workflow", "resume", &run_id, "--json"],
    );
    assert!(
        res.status.success(),
        "resume: {}",
        String::from_utf8_lossy(&res.stderr)
    );
    let cp2 = run(
        workspace.path(),
        home.path(),
        &["workflow", "checkpoints", &run_id, "--json"],
    );
    assert!(cp2.status.success());
    let arr2: serde_json::Value = serde_json::from_slice(&cp2.stdout).unwrap();
    assert_eq!(
        arr2.as_array().map_or(0, Vec::len),
        3,
        "after resume: expected 3 checkpoints, got {arr2}"
    );
}
