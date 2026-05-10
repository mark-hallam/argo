//! End-to-end tests for the Worker v3 action **rendering**
//! surface (#730/#732/#735): workspace-level ledger markdown
//! files (`approval_queue.md`, `blocked_actions.md`,
//! `completed_actions.md`), per-action execution bundles
//! (`instructions.md`, `evidence.md`, `action.json`), manifest
//! `instruction_template` substitution, and the auto-refresh
//! that fires on every transition.
//!
//! Pure transition mechanics live in the sibling
//! `work_action_transitions.rs`.
//!
//! Mock-mode only.

mod common;

use common::work_action_helpers::{fresh_workspace_with_actions, list_actions, run_argo};
use serde_json::Value;
use std::path::PathBuf;

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
