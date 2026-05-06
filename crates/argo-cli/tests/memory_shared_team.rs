mod common;

mod assert_cmd {
    pub use std::process::Command;
}

use assert_cmd::Command;
use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::time::Duration;

const TENANT: &str = "019df000-0000-7000-8000-000000000001";
const OTHER_TENANT: &str = "019df000-0000-7000-8000-000000000002";
const WORKER_A: &str = "019df000-0000-7000-8000-000000000101";
const WORKER_B: &str = "019df000-0000-7000-8000-000000000102";

fn argo(home: &Path, workspace: &Path) -> Command {
    let mut cmd = Command::new(argo_bin());
    cmd.env("ARGO_HOME", home).current_dir(workspace);
    cmd
}

fn write_entry(home: &Path, workspace: &Path, tenant: &str, author: &str, content: &str) -> Value {
    let output = argo(home, workspace)
        .args([
            "memory",
            "shared-team",
            "write",
            "--tenant",
            tenant,
            "--author",
            author,
            "--content",
            content,
            "--json",
        ])
        .output()
        .expect("spawn shared-team write");
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("shared-team write JSON")
}

#[test]
fn memory_shared_team_list_write_and_grep_work() {
    let home = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();

    write_entry(
        home.path(),
        workspace.path(),
        TENANT,
        WORKER_A,
        "alpha planning note",
    );
    std::thread::sleep(Duration::from_millis(2));
    write_entry(
        home.path(),
        workspace.path(),
        TENANT,
        WORKER_B,
        "beta handoff note",
    );
    std::thread::sleep(Duration::from_millis(2));
    let written = write_entry(
        home.path(),
        workspace.path(),
        TENANT,
        WORKER_A,
        "gamma final context",
    );
    write_entry(
        home.path(),
        workspace.path(),
        OTHER_TENANT,
        WORKER_B,
        "other tenant secret",
    );
    assert_eq!(written["tenant_id"], TENANT);
    assert_eq!(written["entry"]["author_worker_id"], WORKER_A);

    let listed = argo(home.path(), workspace.path())
        .args([
            "memory",
            "shared-team",
            "list",
            "--tenant",
            TENANT,
            "--limit",
            "2",
            "--json",
        ])
        .output()
        .expect("spawn shared-team list");
    assert!(
        listed.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&listed.stdout),
        String::from_utf8_lossy(&listed.stderr)
    );
    let listed: Value = serde_json::from_slice(&listed.stdout).expect("shared-team list JSON");
    let entries = listed["entries"].as_array().expect("entries array");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["content"], "gamma final context");
    assert_eq!(entries[1]["content"], "beta handoff note");

    let grep = argo(home.path(), workspace.path())
        .args([
            "memory",
            "shared-team",
            "grep",
            "--tenant",
            TENANT,
            "--pattern",
            "ALPHA|beta",
            "--json",
        ])
        .output()
        .expect("spawn shared-team grep");
    assert!(
        grep.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&grep.stdout),
        String::from_utf8_lossy(&grep.stderr)
    );
    let grep: Value = serde_json::from_slice(&grep.stdout).expect("shared-team grep JSON");
    let matches = grep["entries"].as_array().expect("entries array");
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0]["content"], "beta handoff note");
    assert_eq!(matches[1]["content"], "alpha planning note");
}

#[test]
fn memory_shared_team_requires_tenant_and_rejects_bad_regex() {
    let home = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();

    let missing_tenant = argo(home.path(), workspace.path())
        .args(["memory", "shared-team", "list", "--json"])
        .output()
        .expect("spawn missing tenant");
    assert!(!missing_tenant.status.success());

    let bad = argo(home.path(), workspace.path())
        .args([
            "memory",
            "shared-team",
            "grep",
            "--tenant",
            TENANT,
            "--pattern",
            "[",
        ])
        .output()
        .expect("spawn bad regex");
    assert!(!bad.status.success());
    let stderr = String::from_utf8(bad.stderr).expect("utf8 stderr");
    assert!(stderr.contains("400 invalid regex"), "got: {stderr}");
}
