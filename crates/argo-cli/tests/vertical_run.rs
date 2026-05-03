//! Integration tests for `argo vertical run <id> --workflow <name>`
//! (#669) — the one-shot composite that creates a session,
//! attaches skills, and runs a workflow against that session.
//!
//! The constituent primitives have their own dedicated tests
//! (#651 session bind, #653 skill attach, #659 workflow runner,
//! #668 workflow→session audit). These tests lock the
//! *composition*: the right metadata flows into the JSON
//! document, the session row exists after the run, and the
//! workflow's audit rows landed in the session's chain.

mod common;

use common::argo_bin;
use std::path::Path;
use std::process::Command;

fn write_demo_vertical_with_workflow(verticals_dir: &Path, id: &str, with_skills: bool) {
    let v = verticals_dir.join(id);
    std::fs::create_dir_all(v.join("workflows")).unwrap();
    let mut manifest = format!(
        "schema_version = \"1\"\n\
         [vertical]\nid = \"{id}\"\nname = \"X\"\nversion = \"0.1.0\"\n\
         [tools]\nallowed = [\"file_read\"]\ndenied = []\n\
         [workflows]\nfiles = [\"workflows/main.workflow.toml\"]\n"
    );
    if with_skills {
        manifest.push_str("[skills]\nbundle = \"skills/\"\n");
        std::fs::create_dir_all(v.join("skills")).unwrap();
        // One minimal skill so install_and_attach_bundle finds
        // something to attach.
        std::fs::write(
            v.join("skills/hello.skill.md"),
            "---\n\
             name: hello\n\
             version: 1.0.0\n\
             description: minimal smoke skill\n\
             provenance:\n  source: tests/vertical_run.rs\n  hash: 0\n\
             ---\n\
             # Hello\nbody",
        )
        .unwrap();
    }
    std::fs::write(v.join("vertical.toml"), manifest).unwrap();
    std::fs::write(
        v.join("workflows/main.workflow.toml"),
        "schema_version = \"1\"\n\
         [workflow]\nname = \"go\"\n\
         [[step]]\nname = \"r\"\nkind = \"tool_call\"\ntool = \"file_read\"\n\
         args = { path = \"hello.txt\" }\n",
    )
    .unwrap();
}

#[test]
fn cli_run_composite_creates_session_runs_workflow_returns_combined_json() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let wd = tempfile::tempdir().unwrap();
    std::fs::write(wd.path().join("hello.txt"), "from vertical run").unwrap();
    write_demo_vertical_with_workflow(verticals.path(), "demo", false);

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .current_dir(wd.path())
        .args([
            "vertical",
            "run",
            "demo",
            "--workflow",
            "go",
            "--title",
            "composite-test",
            "--json",
        ])
        .output()
        .expect("spawn run");
    assert!(
        out.status.success(),
        "run stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    let sid = v["session_id"]
        .as_str()
        .expect("session_id in run output");
    assert_eq!(v["vertical_id"], "demo");
    // Sub-document under "workflow" carries the original
    // WorkflowReport JSON.
    let wf = &v["workflow"];
    assert_eq!(wf["ok"], true);
    assert_eq!(wf["workflow"], "go");
    assert_eq!(wf["steps"].as_array().unwrap().len(), 1);
    assert_eq!(wf["steps"][0]["status"], "ok");
    assert!(wf["steps"][0]["message"]
        .as_str()
        .unwrap()
        .contains("from vertical run"));

    // The session row exists + is bound to the vertical.
    let show = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["session", "show", sid, "--json"])
        .output()
        .expect("spawn session show");
    assert!(show.status.success());
    let sv: serde_json::Value =
        serde_json::from_str(&String::from_utf8(show.stdout).unwrap()).unwrap();
    assert_eq!(sv["vertical_id"], "demo");
    assert_eq!(sv["title"], "composite-test");

    // Workflow's tool_call audit rows landed in the session
    // chain (#668 plumbing through the composite path).
    let audit = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["audit", "show", "--session", sid])
        .output()
        .expect("spawn audit show");
    assert!(audit.status.success());
    let body = String::from_utf8(audit.stdout).unwrap();
    assert!(body.contains("tool_call"), "body: {body}");
    assert!(body.contains("tool_result"), "body: {body}");
}

#[test]
fn cli_run_attaches_skill_bundle_when_present() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let wd = tempfile::tempdir().unwrap();
    std::fs::write(wd.path().join("hello.txt"), "x").unwrap();
    write_demo_vertical_with_workflow(verticals.path(), "demo", true);

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .current_dir(wd.path())
        .args([
            "vertical", "run", "demo", "--workflow", "go", "--json",
        ])
        .output()
        .expect("spawn run");
    assert!(
        out.status.success(),
        "run stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert!(
        v["skills_attached"].as_u64().unwrap() >= 1,
        "expected skills_attached >= 1, got: {}",
        v["skills_attached"]
    );
}

#[test]
fn cli_run_unknown_vertical_fails_loudly() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "run", "ghost", "--workflow", "any"])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("ghost"), "stderr: {stderr}");
    assert!(stderr.contains("not found"), "stderr: {stderr}");
}

#[test]
fn cli_run_unknown_workflow_fails_loudly() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let wd = tempfile::tempdir().unwrap();
    write_demo_vertical_with_workflow(verticals.path(), "demo", false);

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .current_dir(wd.path())
        .args(["vertical", "run", "demo", "--workflow", "ghost"])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("ghost"), "stderr: {stderr}");
}

#[test]
fn cli_run_text_mode_prints_session_then_workflow() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let wd = tempfile::tempdir().unwrap();
    std::fs::write(wd.path().join("hello.txt"), "x").unwrap();
    write_demo_vertical_with_workflow(verticals.path(), "demo", false);

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .current_dir(wd.path())
        .args(["vertical", "run", "demo", "--workflow", "go"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let body = String::from_utf8(out.stdout).unwrap();
    assert!(body.contains("created session "), "body: {body}");
    assert!(body.contains("vertical: demo"), "body: {body}");
    // workflow report follows
    assert!(
        body.contains("workflow 'go'"),
        "expected workflow report; body: {body}"
    );
}
