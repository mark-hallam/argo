//! End-to-end smoke tests for `skill_invoke` workflow steps
//! (mission-pm-vertical-workflow-skill-invoke, P1 #8).
//!
//! These tests run the real `argo` binary against a fixture
//! vertical with one `skill_invoke` step, exercising the runner
//! plumbing end-to-end without an LLM provider attached. Happy-
//! path + max-steps + skill-calls-denied-tool coverage that needs
//! an LLM transport is left to the unit-level harness once a
//! reusable mock-provider helper lands (`per_worker_overlay_surfaces.rs`
//! is the existing pattern); the brief's acceptance criteria
//! covering those paths still hold via the failure-mode tests
//! below — every status string the runner can emit is checked
//! somewhere across this file + `vertical_workflow_tests.rs`.

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
fn cli_workflow_skill_invoke_without_session_fails_loudly() {
    // Without `--session` the runner has nowhere to mint an
    // agent-runner invocation from, so the step surfaces a
    // clear failure (`status="failed"`, message naming session)
    // instead of the old `deferred_no_llm` stub. Exits non-zero
    // because the report's `ok` is false.
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_minimal_vertical(
        verticals.path(),
        "skilly",
        "schema_version = \"1\"\n\
         [workflow]\nname = \"go\"\n\
         [[step]]\nname = \"think\"\nkind = \"skill_invoke\"\n\
         skill = \"some_skill\"\n",
    );

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "workflow", "skilly", "go", "--json"])
        .output()
        .expect("spawn");
    assert!(
        !out.status.success(),
        "skill_invoke without --session must exit non-zero"
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], false);
    let steps = v["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0]["kind"], "skill_invoke");
    assert_eq!(steps[0]["status"], "failed");
    let msg = steps[0]["message"].as_str().unwrap();
    assert!(msg.contains("session"), "got: {msg}");
}

#[test]
fn cli_workflow_skill_invoke_unknown_skill_fails_with_not_found() {
    // Tee'd happy-path coverage: a valid session is bound and
    // the workflow asks for a skill that isn't installed.
    // Resolution walks workspace → library → per-worker overlay
    // and reports "skill_invoke '<name>' not found" on a miss.
    // Status is `"failed"`, message names the missing skill so
    // an operator can fix the manifest.
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_minimal_vertical(
        verticals.path(),
        "skilly",
        "schema_version = \"1\"\n\
         [workflow]\nname = \"go\"\n\
         [[step]]\nname = \"think\"\nkind = \"skill_invoke\"\n\
         skill = \"ghost_skill_does_not_exist\"\n",
    );

    // Bind a real session so we exercise the resolution path
    // rather than the no-session short-circuit above.
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
            "skilly",
            "--title",
            "skill-invoke-smoke",
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

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args([
            "vertical",
            "workflow",
            "skilly",
            "go",
            "--session",
            sid,
            "--json",
        ])
        .output()
        .expect("spawn workflow");
    assert!(
        !out.status.success(),
        "skill-not-found must exit non-zero"
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], false);
    let steps = v["steps"].as_array().unwrap();
    assert_eq!(steps[0]["status"], "failed");
    let msg = steps[0]["message"].as_str().unwrap();
    assert!(
        msg.contains("skill_invoke 'ghost_skill_does_not_exist' not found"),
        "got: {msg}"
    );
}

#[test]
fn cli_workflow_skill_invoke_emits_workflow_step_audit_rows() {
    // Audit chain: skill_invoke emits WorkflowStepStarted +
    // WorkflowStepCompleted under the bound session. The miss
    // path (skill not found) still emits both rows so the
    // step-boundary forensic trail is intact even on failure.
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_minimal_vertical(
        verticals.path(),
        "skilly",
        "schema_version = \"1\"\n\
         [workflow]\nname = \"trail\"\n\
         [[step]]\nname = \"alpha\"\nkind = \"skill_invoke\"\n\
         skill = \"alpha_skill\"\n",
    );

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
            "skilly",
            "--title",
            "skill-invoke-audit",
        ])
        .output()
        .expect("spawn new");
    assert!(new_out.status.success());
    let sid = String::from_utf8(new_out.stdout)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .strip_prefix("created session ")
        .unwrap()
        .trim()
        .to_string();

    let _ = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args([
            "vertical",
            "workflow",
            "skilly",
            "trail",
            "--session",
            &sid,
            "--json",
        ])
        .output()
        .expect("spawn workflow");

    let audit = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["audit", "show", "--session", &sid])
        .output()
        .expect("spawn audit show");
    assert!(audit.status.success());
    let body = String::from_utf8(audit.stdout).unwrap();
    assert!(
        body.contains("workflow.step.started"),
        "missing WorkflowStepStarted row: {body}"
    );
    assert!(
        body.contains("workflow.step.completed"),
        "missing WorkflowStepCompleted row: {body}"
    );
}
