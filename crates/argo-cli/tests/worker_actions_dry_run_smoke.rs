//! Integration smoke for `argo worker actions execute --dry-run`.

mod common;

use common::product_e2e_helpers::{assert_success, run_argo};
use std::path::Path;
use std::process::{Command, Output};

const VERTICAL_ID: &str = "dry-run-smoke";

fn write_vertical_manifest(verticals_dir: &Path) {
    let dir = verticals_dir.join(VERTICAL_ID);
    std::fs::create_dir_all(&dir).expect("verticals/<id> dir");
    let manifest = format!(
        r#"schema_version = "1"

[vertical]
id = "{VERTICAL_ID}"
name = "Dry-run smoke"
version = "0.1.0"
description = "Exercises worker action dry-run preview."

[actions.echo_action]
risk = "low"
requires_approval = false
description = "Human handoff action used by the dry-run smoke."
executor = "human"
"#
    );
    std::fs::write(dir.join("vertical.toml"), manifest).expect("write vertical.toml");
}

fn argo_with_verticals(home: &Path, work: &Path, verticals_dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_argo"))
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals_dir)
        .current_dir(work)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

fn append_action(
    home: &Path,
    work: &Path,
    verticals_dir: &Path,
    requires_approval: bool,
) -> String {
    let action_id = uuid::Uuid::now_v7().to_string();
    let body = serde_json::json!({
        "schema_key": "worker_action",
        "action_id": action_id,
        "job_id": "dry-run-job",
        "action_type": "echo_action",
        "risk_level": "low",
        "requires_approval": requires_approval,
        "state": "pending",
        "payload": {"message": "preview only"},
        "created_at": chrono::Utc::now().to_rfc3339(),
    });
    let body_str = serde_json::to_string(&body).expect("serialize action");
    let append = argo_with_verticals(
        home,
        work,
        verticals_dir,
        &[
            "memory",
            "append",
            "--vertical",
            VERTICAL_ID,
            "memory",
            &body_str,
        ],
    );
    assert_success("memory append", &append);
    action_id
}

fn action_state(home: &Path, work: &Path, verticals_dir: &Path, action_id: &str) -> String {
    let show = argo_with_verticals(
        home,
        work,
        verticals_dir,
        &["worker", "actions", "show", action_id, "--json"],
    );
    assert_success("worker actions show", &show);
    let v: serde_json::Value = serde_json::from_slice(&show.stdout).expect("show JSON");
    v.get("state")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| panic!("show output missing /state: {v}"))
}

fn history_len(home: &Path, work: &Path, verticals_dir: &Path, action_id: &str) -> usize {
    let status = argo_with_verticals(
        home,
        work,
        verticals_dir,
        &["worker", "actions", "status", action_id, "--json"],
    );
    assert_success("worker actions status", &status);
    String::from_utf8_lossy(&status.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
}

#[test]
fn execute_help_shows_dry_run_flag() {
    let home = tempfile::tempdir().expect("tempdir for ARGO_HOME");
    let work = tempfile::tempdir().expect("tempdir for workspace");
    let help = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "execute", "--help"],
    );
    assert_success("execute help", &help);
    let stdout = String::from_utf8_lossy(&help.stdout);
    assert!(stdout.contains("--dry-run"), "help output:\n{stdout}");
}

#[test]
fn dry_run_json_previews_without_state_or_bundle_side_effects() {
    let home = tempfile::tempdir().expect("tempdir for ARGO_HOME");
    let work = tempfile::tempdir().expect("tempdir for workspace");
    let verticals_dir = work.path().join("verticals");
    write_vertical_manifest(&verticals_dir);
    let init = run_argo(home.path(), work.path(), &["workspace", "init"]);
    assert_success("workspace init", &init);
    let action_id = append_action(home.path(), work.path(), &verticals_dir, false);

    let dry_run = argo_with_verticals(
        home.path(),
        work.path(),
        &verticals_dir,
        &[
            "worker",
            "actions",
            "execute",
            &action_id,
            "--dry-run",
            "--json",
        ],
    );
    assert_success("worker actions execute --dry-run", &dry_run);
    let v: serde_json::Value = serde_json::from_slice(&dry_run.stdout).expect("dry-run JSON");
    assert_eq!(
        v.get("dry_run").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        v.get("action_id").and_then(serde_json::Value::as_str),
        Some(action_id.as_str())
    );
    assert_eq!(
        v.pointer("/would_execute/executor_kind")
            .and_then(serde_json::Value::as_str),
        Some("human")
    );
    assert_eq!(
        v.pointer("/would_execute/next_state")
            .and_then(serde_json::Value::as_str),
        Some("executing")
    );

    assert_eq!(
        action_state(home.path(), work.path(), &verticals_dir, &action_id),
        "pending"
    );
    assert_eq!(
        history_len(home.path(), work.path(), &verticals_dir, &action_id),
        1
    );
    let bundle = work
        .path()
        .join(".skycot")
        .join("outputs")
        .join("dry-run-job")
        .join("actions")
        .join(&action_id);
    assert!(
        !bundle.exists(),
        "dry-run created bundle at {}",
        bundle.display()
    );
}

#[test]
fn dry_run_preserves_approval_required_failure_without_appending_rows() {
    let home = tempfile::tempdir().expect("tempdir for ARGO_HOME");
    let work = tempfile::tempdir().expect("tempdir for workspace");
    let verticals_dir = work.path().join("verticals");
    write_vertical_manifest(&verticals_dir);
    let init = run_argo(home.path(), work.path(), &["workspace", "init"]);
    assert_success("workspace init", &init);
    let action_id = append_action(home.path(), work.path(), &verticals_dir, true);

    let dry_run = argo_with_verticals(
        home.path(),
        work.path(),
        &verticals_dir,
        &[
            "worker",
            "actions",
            "execute",
            &action_id,
            "--dry-run",
            "--json",
        ],
    );
    assert!(
        !dry_run.status.success(),
        "approval-gated dry-run should fail"
    );
    let stderr = String::from_utf8_lossy(&dry_run.stderr);
    assert!(stderr.contains("approval required"), "stderr:\n{stderr}");
    assert_eq!(
        history_len(home.path(), work.path(), &verticals_dir, &action_id),
        1
    );
}
