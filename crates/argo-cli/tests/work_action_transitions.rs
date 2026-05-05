//! End-to-end test for the Worker v3 action transition CLI
//! (#730). Drives the same `argo worker actions {approve,
//! execute, complete, fail, status, render}` surface an
//! operator uses — through the binary, against a real
//! workspace + memory store.
//!
//! Setup:
//!   1. fresh workspace (tempdir)
//!   2. mock-mode worker run produces 2 actions:
//!      - verify_price → state = pending (autonomous-safe)
//!      - post_listing → state = blocked (gated, evidence
//!        insufficient under mock)
//!   3. drive transitions:
//!      - approve refuses verify_price (autonomous)
//!      - execute advances verify_price pending → executing
//!      - complete advances executing → completed with note
//!      - status emits the full 3-row history
//!      - render writes 3 markdown ledger files under .skycot/
//!
//! Mock-mode only.

mod common;

use common::argo_bin;
use serde_json::Value;
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

fn fresh_workspace_with_actions() -> (tempfile::TempDir, tempfile::TempDir) {
    let home = tempfile::tempdir().expect("home tempdir");
    let work = tempfile::tempdir().expect("work tempdir");
    let init = run_argo(home.path(), work.path(), &["workspace", "init"]);
    assert!(
        init.status.success(),
        "init: {}",
        String::from_utf8_lossy(&init.stderr)
    );
    let add = run_argo(
        home.path(),
        work.path(),
        &[
            "work",
            "add",
            "--vertical",
            "gtm-engineer",
            "--prompt",
            "go",
            "--json",
        ],
    );
    assert!(
        add.status.success(),
        "{}",
        String::from_utf8_lossy(&add.stderr)
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
    assert!(
        serve.status.success(),
        "serve: {}",
        String::from_utf8_lossy(&serve.stderr)
    );
    (home, work)
}

fn list_actions(home: &Path, work: &Path) -> Vec<Value> {
    let out = run_argo(
        home,
        work,
        &[
            "worker",
            "actions",
            "list",
            "--state",
            "all",
            "--vertical",
            "gtm-engineer",
            "--json",
        ],
    );
    assert!(
        out.status.success(),
        "list: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

#[test]
fn approval_refused_for_autonomous_safe_action() {
    let (home, work) = fresh_workspace_with_actions();
    let rows = list_actions(home.path(), work.path());
    let verify = rows
        .iter()
        .find(|r| r["action_type"] == "verify_price")
        .expect("verify_price row");
    let id = verify["action_id"].as_str().expect("action_id");

    let out = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "approve", id],
    );
    assert!(!out.status.success(), "approve must refuse autonomous-safe");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("autonomous-safe"),
        "stderr should explain why; got: {stderr}"
    );
}

#[test]
fn execute_then_complete_round_trips_an_autonomous_action() {
    let (home, work) = fresh_workspace_with_actions();
    let rows = list_actions(home.path(), work.path());
    let verify = rows
        .iter()
        .find(|r| r["action_type"] == "verify_price")
        .expect("verify_price row");
    let id = verify["action_id"].as_str().expect("action_id");

    let exec = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "execute", id, "--json"],
    );
    assert!(
        exec.status.success(),
        "execute: {}",
        String::from_utf8_lossy(&exec.stderr)
    );
    let v: Value = serde_json::from_slice(&exec.stdout).expect("execute json");
    assert_eq!(v["state"], "executing");
    assert!(v["executed_at"].as_str().is_some());

    let complete = run_argo(
        home.path(),
        work.path(),
        &[
            "worker",
            "actions",
            "complete",
            id,
            "--note",
            "ran offline; price OK",
            "--json",
        ],
    );
    assert!(
        complete.status.success(),
        "complete: {}",
        String::from_utf8_lossy(&complete.stderr)
    );
    let v: Value = serde_json::from_slice(&complete.stdout).expect("complete json");
    assert_eq!(v["state"], "completed");
    assert_eq!(v["result"]["summary"], "ran offline; price OK");
}

#[test]
fn status_emits_full_state_history_in_time_order() {
    let (home, work) = fresh_workspace_with_actions();
    let rows = list_actions(home.path(), work.path());
    let verify = rows
        .iter()
        .find(|r| r["action_type"] == "verify_price")
        .expect("verify_price row");
    let id = verify["action_id"].as_str().expect("action_id");

    let _ = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "execute", id, "--json"],
    );
    let _ = run_argo(
        home.path(),
        work.path(),
        &[
            "worker", "actions", "complete", id, "--note", "ok", "--json",
        ],
    );

    let status = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "status", id, "--json"],
    );
    assert!(
        status.status.success(),
        "status: {}",
        String::from_utf8_lossy(&status.stderr)
    );
    let states: Vec<String> = String::from_utf8_lossy(&status.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter_map(|v| v["state"].as_str().map(String::from))
        .collect();
    assert_eq!(states, vec!["pending", "executing", "completed"]);
}

#[test]
fn render_writes_workspace_level_ledger_artifacts() {
    let (home, work) = fresh_workspace_with_actions();
    let render = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "render", "--vertical", "gtm-engineer"],
    );
    assert!(
        render.status.success(),
        "render: {}",
        String::from_utf8_lossy(&render.stderr)
    );
    let dir = work.path().join(".skycot");
    let queue = std::fs::read_to_string(dir.join("approval_queue.md")).expect("approval_queue.md");
    let blocked =
        std::fs::read_to_string(dir.join("blocked_actions.md")).expect("blocked_actions.md");
    let completed =
        std::fs::read_to_string(dir.join("completed_actions.md")).expect("completed_actions.md");

    // Mock evidence is insufficient → post_listing landed
    // blocked, not pending. Approval queue is empty.
    assert!(queue.contains("(no actions awaiting approval)"));
    // post_listing should appear in blocked.
    assert!(
        blocked.contains("post_listing"),
        "expected post_listing in blocked; got:\n{blocked}"
    );
    // No completes yet.
    assert!(completed.contains("(no actions have completed yet)"));
}

#[test]
fn execute_writes_per_action_bundle_via_human_executor() {
    let (home, work) = fresh_workspace_with_actions();
    let rows = list_actions(home.path(), work.path());
    let verify = rows
        .iter()
        .find(|r| r["action_type"] == "verify_price")
        .expect("verify_price row");
    let id = verify["action_id"].as_str().expect("action_id");
    let job_id = verify["job_id"].as_str().expect("job_id");

    let exec = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "execute", id, "--json"],
    );
    assert!(
        exec.status.success(),
        "execute: {}",
        String::from_utf8_lossy(&exec.stderr)
    );
    let v: Value = serde_json::from_slice(&exec.stdout).expect("execute json");
    let bundle_path = v["execution_bundle_path"]
        .as_str()
        .expect("execution_bundle_path on execute --json");
    let bundle = PathBuf::from(bundle_path);
    // Bundle layout: .skycot/outputs/<job_id>/actions/<action_id>/
    assert!(
        bundle_path.contains(job_id),
        "bundle_path should include job_id; got {bundle_path}"
    );
    assert!(
        bundle_path.contains(id),
        "bundle_path should include action_id; got {bundle_path}"
    );
    let instructions =
        std::fs::read_to_string(bundle.join("instructions.md")).expect("instructions.md");
    let evidence = std::fs::read_to_string(bundle.join("evidence.md")).expect("evidence.md");
    let action_json: Value =
        serde_json::from_slice(&std::fs::read(bundle.join("action.json")).unwrap()).unwrap();
    // gtm-engineer's verify_price now ships an instruction_template
    // (#732) — the rendered body is template-driven rather than
    // the generic dump, so we assert on template content.
    assert!(
        instructions.contains("Verify price"),
        "expected verify_price template content; got:\n{instructions}"
    );
    assert!(instructions.contains("argo worker actions complete"));
    assert!(evidence.contains("Evidence for action"));
    assert_eq!(action_json["action_type"], "verify_price");
    assert_eq!(action_json["state"], "executing");
}

#[test]
fn execute_renders_manifest_template_when_one_is_declared() {
    // gtm-engineer's `verify_price` declares an
    // `instruction_template` that includes `{{ payload.target }}`
    // and an `argo worker actions complete <id>` recipe. Confirm
    // the rendered bundle uses that template (not the generic
    // dump) when execute fires.
    let (home, work) = fresh_workspace_with_actions();
    let rows = list_actions(home.path(), work.path());
    let verify = rows
        .iter()
        .find(|r| r["action_type"] == "verify_price")
        .expect("verify_price row");
    let id = verify["action_id"].as_str().expect("action_id");

    let exec = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "execute", id, "--json"],
    );
    assert!(
        exec.status.success(),
        "execute: {}",
        String::from_utf8_lossy(&exec.stderr)
    );
    let v: Value = serde_json::from_slice(&exec.stdout).expect("execute json");
    let bundle = PathBuf::from(v["execution_bundle_path"].as_str().expect("bundle path"));
    let instr = std::fs::read_to_string(bundle.join("instructions.md"))
        .expect("instructions.md from template");
    // The template body has these literal substrings — they
    // can't appear in the generic dump.
    assert!(
        instr.contains("Verify price for"),
        "expected manifest template body; got:\n{instr}"
    );
    assert!(
        instr.contains("argo worker actions complete"),
        "manifest template should still steer the operator to the next CLI step; got:\n{instr}"
    );
    // payload.target was 0 in the mock action — confirm it
    // round-trips through the substitution.
    assert!(
        instr.contains("$0"),
        "expected ${{payload.target}} (= 0) to render; got:\n{instr}"
    );
}

#[test]
fn approve_auto_refreshes_workspace_ledger_files() {
    // Operators shouldn't have to remember to run
    // `argo worker actions render` after every transition.
    // #735 wires an auto-refresh into approve/execute/complete/
    // fail so the 3 ledger files always reflect the latest
    // ledger state.
    let (home, work) = fresh_workspace_with_actions();
    let rows = list_actions(home.path(), work.path());
    let post = rows
        .iter()
        .find(|r| r["action_type"] == "post_listing")
        .expect("post_listing row");
    let post_id = post["action_id"].as_str().expect("post_id");
    // post_listing landed `blocked` under mock evidence — so
    // before any transition, blocked_actions.md doesn't exist
    // (no render has fired yet).
    let dir = work.path().join(".skycot");
    assert!(
        !dir.join("blocked_actions.md").exists(),
        "auto-render must NOT have fired before any transition"
    );

    // Now run a transition (use verify_price → execute since
    // it's the easiest transition and isn't blocked).
    let verify = rows
        .iter()
        .find(|r| r["action_type"] == "verify_price")
        .expect("verify_price row");
    let verify_id = verify["action_id"].as_str().expect("verify_id");
    let exec = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "execute", verify_id, "--json"],
    );
    assert!(exec.status.success());

    // After the transition, all 3 ledger files exist (auto-
    // refresh fired). The blocked file lists post_listing.
    let blocked = std::fs::read_to_string(dir.join("blocked_actions.md"))
        .expect("blocked_actions.md auto-rendered");
    assert!(
        blocked.contains(post_id),
        "auto-rendered blocked_actions.md should contain {post_id}; got:\n{blocked}"
    );
    assert!(
        dir.join("approval_queue.md").exists(),
        "approval_queue.md should have auto-rendered"
    );
    assert!(
        dir.join("completed_actions.md").exists(),
        "completed_actions.md should have auto-rendered"
    );
}

#[test]
fn fail_records_reason_for_an_executing_action() {
    let (home, work) = fresh_workspace_with_actions();
    let rows = list_actions(home.path(), work.path());
    let verify = rows
        .iter()
        .find(|r| r["action_type"] == "verify_price")
        .expect("verify_price row");
    let id = verify["action_id"].as_str().expect("action_id");

    let _ = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "execute", id, "--json"],
    );
    let fail = run_argo(
        home.path(),
        work.path(),
        &[
            "worker",
            "actions",
            "fail",
            id,
            "--reason",
            "marketplace returned 503",
            "--json",
        ],
    );
    assert!(
        fail.status.success(),
        "fail: {}",
        String::from_utf8_lossy(&fail.stderr)
    );
    let v: Value = serde_json::from_slice(&fail.stdout).expect("fail json");
    assert_eq!(v["state"], "failed");
    assert_eq!(v["result"]["failure"], "marketplace returned 503");
}
