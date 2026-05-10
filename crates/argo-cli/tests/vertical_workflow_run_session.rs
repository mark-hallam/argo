//! Integration tests for `argo vertical workflow … --session <id>`
//! (#668 and follow-ups). Split out of `vertical_workflow_run.rs`
//! to keep that file under the 500-LOC ceiling.
//!
//! These tests cover the session-bound flavor of the workflow runner:
//!
//! - tool-call audit rows landing in the session's audit chain with
//!   the `workflow_runner` source label,
//! - loud failure on an unknown UUIDv7 session id,
//! - loud failure on a malformed session id string.
//!
//! The non-session paths and the approval / deny / deferred /
//! anchor-workflow surfaces stay in `vertical_workflow_run.rs`.

mod common;

use common::argo_bin;
use std::path::Path;
use std::process::Command;

fn write_minimal_vertical(verticals_dir: &Path, id: &str, workflow_body: &str) {
    let v = verticals_dir.join(id);
    std::fs::create_dir_all(v.join("workflows")).unwrap();
    std::fs::write(
        v.join("vertical.toml"),
        format!(
            "schema_version = \"1\"\n\
             [vertical]\nid = \"{id}\"\nname = \"X\"\nversion = \"0.1.0\"\n\
             [tools]\nallowed = [\"file_read\"]\ndenied = [\"file_write\"]\n\
             [workflows]\nfiles = [\"workflows/main.workflow.toml\"]\n"
        ),
    )
    .unwrap();
    std::fs::write(v.join("workflows/main.workflow.toml"), workflow_body).unwrap();
}

#[test]
fn cli_workflow_with_session_emits_tool_call_audit_rows_into_session_chain() {
    // #668: when --session is passed, every dispatched tool_call
    // step lands in the session's audit chain alongside agent
    // turns. Assertion: the session's audit show output mentions
    // tool_call + tool_result kinds AND the workflow_runner
    // source label.
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let wd = tempfile::tempdir().unwrap();
    std::fs::write(wd.path().join("hello.txt"), "audit me").unwrap();
    write_minimal_vertical(
        verticals.path(),
        "demo",
        "schema_version = \"1\"\n\
         [workflow]\nname = \"go\"\n\
         [[step]]\nname = \"r\"\nkind = \"tool_call\"\ntool = \"file_read\"\n\
         args = { path = \"hello.txt\" }\n",
    );

    // Create a session bound to the vertical so the audit row's
    // session_id is meaningful for downstream rollups.
    let new_out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args([
            "new",
            "--provider",
            "mock",
            "--model",
            "mock",
            "--vertical",
            "demo",
            "--title",
            "wf-audit-test",
        ])
        .output()
        .expect("spawn new");
    assert!(
        new_out.status.success(),
        "new stderr: {}",
        String::from_utf8_lossy(&new_out.stderr)
    );
    let stdout = String::from_utf8(new_out.stdout).unwrap();
    let line = stdout.lines().next().expect("empty stdout from `new`");
    let sid = line
        .strip_prefix("created session ")
        .expect("first line should be `created session <uuid>`")
        .trim();

    // Run the workflow against this session.
    let wf_out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .current_dir(wd.path())
        .args([
            "vertical",
            "workflow",
            "demo",
            "go",
            "--session",
            sid,
            "--json",
        ])
        .output()
        .expect("spawn workflow");
    assert!(
        wf_out.status.success(),
        "workflow stderr: {}",
        String::from_utf8_lossy(&wf_out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(wf_out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);

    // Confirm tool_call + tool_result audit rows landed.
    let audit = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["audit", "show", "--session", sid])
        .output()
        .expect("spawn audit show");
    assert!(audit.status.success());
    let body = String::from_utf8(audit.stdout).unwrap();
    assert!(body.contains("tool_call"), "body: {body}");
    assert!(body.contains("tool_result"), "body: {body}");

    // And confirm the payload carries the workflow_runner source
    // label via export.
    let exp = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["audit", "export", "--session", sid])
        .output()
        .expect("spawn audit export");
    assert!(exp.status.success());
    let exported = String::from_utf8(exp.stdout).unwrap();
    assert!(
        exported.contains("workflow_runner"),
        "expected workflow_runner source label, got: {exported}"
    );
}

#[test]
fn cli_workflow_with_unknown_session_id_fails_loudly() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_minimal_vertical(
        verticals.path(),
        "demo",
        "schema_version = \"1\"\n\
         [workflow]\nname = \"go\"\n\
         [[step]]\nname = \"r\"\nkind = \"tool_call\"\ntool = \"file_read\"\n\
         args = { path = \"x\" }\n",
    );
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args([
            "vertical",
            "workflow",
            "demo",
            "go",
            "--session",
            "00000000-0000-7000-8000-000000000000",
        ])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("no session"), "stderr: {stderr}");
}

#[test]
fn cli_workflow_with_invalid_session_uuid_fails_loudly() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_minimal_vertical(
        verticals.path(),
        "demo",
        "schema_version = \"1\"\n\
         [workflow]\nname = \"go\"\n\
         [[step]]\nname = \"r\"\nkind = \"tool_call\"\ntool = \"file_read\"\n\
         args = { path = \"x\" }\n",
    );
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args([
            "vertical",
            "workflow",
            "demo",
            "go",
            "--session",
            "not-a-uuid",
        ])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("invalid session id"),
        "stderr: {stderr}"
    );
}
