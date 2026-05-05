//! Lock the Sales Follow-up vertical's manifest classification
//! (#746). Third vertical binding, mirroring the
//! `vertical_ai_receptionist.rs` shape — proves the OS layer
//! is genuinely vertical-agnostic across THREE distinct
//! domains (marketplace seller, receptionist, sales follow-up).

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
fn sales_follow_up_loads_via_argo_vertical_list() {
    let out = run_argo(&["vertical", "list"]);
    assert!(
        out.status.success(),
        "vertical list: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("sales-follow-up"),
        "expected sales-follow-up in vertical list; got:\n{stdout}"
    );
}

#[test]
fn sales_follow_up_lint_clean() {
    let out = run_argo(&["vertical", "lint", "sales-follow-up"]);
    assert!(
        out.status.success(),
        "vertical lint: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("lint clean"),
        "expected sales-follow-up lint clean; got:\n{stdout}"
    );
}

/// Manifest-level safety boundary lock for the Sales Follow-up
/// vertical. Anything that touches a prospect's inbox or the
/// CRM is approval-gated; research + drafting stays autonomous.
#[test]
fn sales_follow_up_manifest_declares_expected_action_classification() {
    let dir = workspace_root().join("verticals");
    let (registry, errs) = argo_vertical::load_dir(&dir).unwrap();
    assert!(errs.is_empty(), "load errors: {errs:?}");
    let loaded = registry
        .get("sales-follow-up")
        .expect("sales-follow-up must be loaded");
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
    // Autonomous-safe: research + drafting only.
    assert_class("research_lead", false);
    assert_class("find_contact", false);
    assert_class("draft_followup", false);
    // Approval-gated: anything that reaches prospect / CRM.
    assert_class("send_followup_email", true);
    assert_class("schedule_followup_cadence", true);
    assert_class("log_to_crm", true);
}
