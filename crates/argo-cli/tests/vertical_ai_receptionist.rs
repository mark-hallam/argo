//! Lock the AI Receptionist vertical's manifest classification
//! (#743). Proves the OS is genuinely vertical-agnostic — same
//! manifest schema, different domain (receptionist instead of
//! marketplace seller), same safety primitives.
//!
//! Asserts:
//!   1. The vertical's `[actions]` declarations classify
//!      correctly (autonomous-safe vs approval-gated) per the
//!      SKYCOT safety boundary.
//!   2. `argo vertical lint ai-receptionist` is clean — no
//!      template typos, no orphan tools, no other warnings.

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

fn run_argo(args: &[&str]) -> std::process::Output {
    Command::new(argo_bin())
        .args(args)
        .env("ARGO_VERTICALS_DIR", workspace_root().join("verticals"))
        .output()
        .expect("spawn argo")
}

#[test]
fn ai_receptionist_loads_via_argo_vertical_list() {
    let out = run_argo(&["vertical", "list"]);
    assert!(
        out.status.success(),
        "vertical list: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("ai-receptionist"),
        "expected ai-receptionist in vertical list; got:\n{stdout}"
    );
}

#[test]
fn ai_receptionist_lint_clean() {
    let out = run_argo(&["vertical", "lint", "ai-receptionist"]);
    assert!(
        out.status.success(),
        "vertical lint: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("lint clean"),
        "expected ai-receptionist lint clean; got:\n{stdout}"
    );
}

/// Lock the SKYCOT safety boundary at the manifest level. The
/// production receptionist manifest must classify the same way
/// the system prompt instructs the model — anything that
/// touches calendar / human / email is approval-gated.
#[test]
fn ai_receptionist_manifest_declares_expected_action_classification() {
    let dir = workspace_root().join("verticals");
    let (registry, errs) = argo_vertical::load_dir(&dir).unwrap();
    assert!(errs.is_empty(), "load errors: {errs:?}");
    let loaded = registry
        .get("ai-receptionist")
        .expect("ai-receptionist must be loaded");
    let actions = loaded.manifest.actions.as_ref().expect("[actions] section");
    let assert_class = |name: &str, requires_approval: bool| {
        let decl = actions
            .actions
            .get(name)
            .unwrap_or_else(|| panic!("missing action '{name}'"));
        assert_eq!(
            decl.requires_approval, requires_approval,
            "action '{name}' classification drifted from manifest expectation",
        );
    };
    // Autonomous-safe: research / drafting / message capture.
    assert_class("lookup_contact", false);
    assert_class("draft_meeting_request", false);
    assert_class("take_message", false);
    // Approval-gated: calendar / forwarding / human-touching.
    assert_class("send_calendar_invite", true);
    assert_class("forward_call", true);
    assert_class("send_followup_email", true);
}
