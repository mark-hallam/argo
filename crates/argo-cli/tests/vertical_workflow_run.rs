//! Integration tests for `argo vertical workflow <id> <name>` (#659).
//!
//! These tests boot the real `argo` binary against a tempdir
//! that holds a hand-written vertical so we exercise the full
//! manifest → workflow file → runner → tool dispatch path. They
//! guard against silent regressions in:
//!
//! - the runner finding workflow files via the manifest's
//!   `[workflows].files` list,
//! - the per-step dispatch + capability + deny-list checks,
//! - the JSON shape the runner emits (operator-facing contract),
//! - non-zero exit when any step fails.

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

fn write_vertical_with_approval(
    verticals_dir: &Path,
    id: &str,
    approval: &[&str],
    workflow_body: &str,
) {
    let v = verticals_dir.join(id);
    std::fs::create_dir_all(v.join("workflows")).unwrap();
    let approval_toml = approval
        .iter()
        .map(|s| format!("\"{s}\""))
        .collect::<Vec<_>>()
        .join(", ");
    std::fs::write(
        v.join("vertical.toml"),
        format!(
            "schema_version = \"1\"\n\
             [vertical]\nid = \"{id}\"\nname = \"X\"\nversion = \"0.1.0\"\n\
             [tools]\nallowed = [\"file_read\", \"file_write\"]\ndenied = []\n\
             [escalation]\nhuman_approval_required = [{approval_toml}]\n\
             [workflows]\nfiles = [\"workflows/main.workflow.toml\"]\n"
        ),
    )
    .unwrap();
    std::fs::write(v.join("workflows/main.workflow.toml"), workflow_body).unwrap();
}

#[test]
fn cli_workflow_runs_tool_call_step_end_to_end() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let wd = tempfile::tempdir().unwrap();
    std::fs::write(wd.path().join("hello.txt"), "hi from the workflow runner").unwrap();

    write_minimal_vertical(
        verticals.path(),
        "demo",
        "schema_version = \"1\"\n\
         [workflow]\nname = \"go\"\n\
         [[step]]\nname = \"r\"\nkind = \"tool_call\"\ntool = \"file_read\"\n\
         args = { path = \"hello.txt\" }\n",
    );

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .current_dir(wd.path())
        .args(["vertical", "workflow", "demo", "go", "--json"])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout),
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["vertical_id"], "demo");
    assert_eq!(v["workflow"], "go");
    assert_eq!(v["ok"], true);
    let steps = v["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0]["status"], "ok");
    assert_eq!(steps[0]["kind"], "tool_call");
    let msg = steps[0]["message"].as_str().unwrap();
    assert!(
        msg.contains("hi from the workflow runner"),
        "expected file content preview, got: {msg}"
    );
}

#[test]
fn cli_workflow_blocks_denied_tool_with_deny_status_and_non_zero_exit() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_minimal_vertical(
        verticals.path(),
        "denyer",
        "schema_version = \"1\"\n\
         [workflow]\nname = \"bad\"\n\
         [[step]]\nname = \"x\"\nkind = \"tool_call\"\ntool = \"file_write\"\n\
         args = { path = \"x.txt\", content = \"y\" }\n",
    );

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "workflow", "denyer", "bad", "--json"])
        .output()
        .expect("spawn");
    assert!(!out.status.success(), "denied workflow must exit non-zero");
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], false);
    let steps = v["steps"].as_array().unwrap();
    assert_eq!(steps[0]["status"], "denied_by_vertical");
}

#[test]
fn cli_workflow_marks_skill_invoke_and_prompt_as_deferred() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_minimal_vertical(
        verticals.path(),
        "todo",
        "schema_version = \"1\"\n\
         [workflow]\nname = \"mixed\"\n\
         [[step]]\nname = \"a\"\nkind = \"skill_invoke\"\nskill = \"identify_invariant_violations\"\n\
         [[step]]\nname = \"b\"\nkind = \"prompt\"\nprompt = \"what is 2+2?\"\n",
    );

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "workflow", "todo", "mixed", "--json"])
        .output()
        .expect("spawn");
    // Deferred steps are NOT failures — workflow exits 0, both
    // statuses are `deferred_no_llm`. This is the contract the
    // operator can rely on while #660+ wire the LLM transport.
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    let steps = v["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0]["status"], "deferred_no_llm");
    assert_eq!(steps[1]["status"], "deferred_no_llm");
}

#[test]
fn cli_workflow_unknown_vertical_fails_loudly() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "workflow", "ghost", "any"])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("ghost"), "got: {stderr}");
    assert!(stderr.contains("not found"), "got: {stderr}");
}

#[test]
fn cli_workflow_unknown_workflow_name_fails_loudly() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_minimal_vertical(
        verticals.path(),
        "demo",
        "schema_version = \"1\"\n\
         [workflow]\nname = \"only\"\n\
         [[step]]\nname = \"r\"\nkind = \"prompt\"\nprompt = \"hi\"\n",
    );
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "workflow", "demo", "ghost"])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("ghost"), "got: {stderr}");
}

#[test]
fn cli_workflow_pauses_on_tool_in_human_approval_required_list() {
    // Approval gate (#660): a tool in the bound vertical's
    // [escalation].human_approval_required list does NOT
    // dispatch — the runner records `escalation_required` and
    // exits non-zero so CI surfaces the pause.
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let wd = tempfile::tempdir().unwrap();
    write_vertical_with_approval(
        verticals.path(),
        "gated",
        &["file_write"],
        "schema_version = \"1\"\n\
         [workflow]\nname = \"go\"\n\
         [[step]]\nname = \"r\"\nkind = \"tool_call\"\ntool = \"file_write\"\n\
         args = { path = \"will_not_be_written.txt\", content = \"x\" }\n",
    );

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .current_dir(wd.path())
        .args(["vertical", "workflow", "gated", "go", "--json"])
        .output()
        .expect("spawn");
    assert!(
        !out.status.success(),
        "approval-required workflow must exit non-zero"
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], false);
    let steps = v["steps"].as_array().unwrap();
    assert_eq!(steps[0]["status"], "escalation_required");
    let msg = steps[0]["message"].as_str().unwrap();
    assert!(msg.contains("file_write"), "got: {msg}");
    assert!(msg.contains("human approval"), "got: {msg}");
    // Critically: the file MUST NOT have been written.
    assert!(
        !wd.path().join("will_not_be_written.txt").exists(),
        "approval gate must block dispatch"
    );
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

#[test]
fn cli_workflow_pr_checker_real_anchor_workflow_runs_clean() {
    // Lock the shipped PR-Checker workflow asset — future edits
    // that break the runner end-to-end against this vertical
    // will fail this test.
    let repo_verticals = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join("verticals");
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf();
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", &repo_verticals)
        .current_dir(&repo_root)
        .args(["vertical", "workflow", "pr-checker", "read_repo_anchors", "--json"])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    let steps = v["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 3);
    for s in steps {
        assert_eq!(
            s["status"], "ok",
            "expected ok, got step={}",
            serde_json::to_string(s).unwrap_or_default()
        );
    }
}
