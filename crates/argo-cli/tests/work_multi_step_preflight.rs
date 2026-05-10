//! Pre-flight regression tests for Worker v2 multi-step (#722, #723).
//!
//! Sibling to `work_multi_step.rs`. Owns the no-research fixture
//! that exercises the `research_tools_unavailable` stop path —
//! split out from `work_multi_step.rs` to stay under the 500-LOC
//! ceiling (see `mvp-split-work-multi-step`).
//!
//! Mock-mode only — no API key required.

mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Build a tempdir-backed verticals directory that contains a
/// single fixture vertical denying both web tools. Lets the
/// pre-flight test exercise the `research_tools_unavailable`
/// path even after #723 made the production gtm-engineer
/// manifest allow them again.
fn build_no_research_verticals_dir() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("verticals tempdir");
    let v_dir = root.path().join("no-research");
    std::fs::create_dir_all(v_dir.join("prompts")).expect("prompts dir");
    std::fs::write(
        v_dir.join("prompts").join("system.md"),
        "Test vertical that denies web research tools so the worker pre-flight fires.\n",
    )
    .unwrap();
    std::fs::write(
        v_dir.join("prompts").join("persona.md"),
        "Test persona for the no-research fixture vertical.\n",
    )
    .unwrap();
    std::fs::write(
        v_dir.join("vertical.toml"),
        r#"schema_version = "1"

[vertical]
id          = "no-research"
name        = "No Research Test"
description = "Test fixture: denies web tools so worker v2 pre-flight fires."
version     = "0.0.1"
authors     = ["test"]
tags        = ["test", "draft-only"]

[role]
system_prompt = "prompts/system.md"
persona       = "prompts/persona.md"

[provider]
default   = "mock"
fallback  = "mock"
cost_tier = "balanced"

[tools]
allowed = []
denied  = ["web_search", "web_extract"]
"#,
    )
    .unwrap();
    root
}

/// Run `argo` with `ARGO_VERTICALS_DIR` overridden to a custom
/// directory (for the no-research fixture). Mirrors `run_argo`
/// in `work_multi_step.rs` otherwise.
fn run_argo_with_verticals(
    home: &Path,
    work: &Path,
    verticals: &Path,
    args: &[&str],
) -> std::process::Output {
    Command::new(argo_bin())
        .args(args)
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals)
        .current_dir(work)
        .output()
        .expect("spawn argo")
}

/// The headline v2 + #723 acceptance test: with a vertical that
/// denies both web tools, the default multi-step worker run
/// must fail the pre-flight rather than silently produce an
/// unverified output. The production gtm-engineer manifest now
/// has the real tools in its allow list, so we use a tempdir
/// fixture vertical that restores the deny shape #722 originally
/// tested.
#[test]
fn multi_step_fails_fast_when_research_tools_unavailable() {
    let verticals = build_no_research_verticals_dir();

    // Bootstrap a workspace + ARGO_HOME pointing at the fixture
    // verticals dir so `validate_vertical` and `add` both see
    // `no-research` instead of the production set.
    let home = tempfile::tempdir().expect("home tempdir");
    let work = tempfile::tempdir().expect("work tempdir");
    let init = run_argo_with_verticals(
        home.path(),
        work.path(),
        verticals.path(),
        &["workspace", "init"],
    );
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );

    let add = run_argo_with_verticals(
        home.path(),
        work.path(),
        verticals.path(),
        &[
            "work",
            "add",
            "--vertical",
            "no-research",
            "--prompt",
            "Sell these skates.",
            "--json",
        ],
    );
    assert!(
        add.status.success(),
        "{}",
        String::from_utf8_lossy(&add.stderr)
    );
    let added: Value = serde_json::from_slice(&add.stdout).expect("json");
    let job_id = added["job_id"].as_str().expect("job_id").to_string();

    // No --max-steps → defaults to 5 → pre-flight fires.
    let serve = run_argo_with_verticals(
        home.path(),
        work.path(),
        verticals.path(),
        &[
            "work",
            "serve",
            "--vertical",
            "no-research",
            "--once",
            "--json",
        ],
    );
    assert!(
        serve.status.success(),
        "work serve must succeed even when pre-flight stops the job (so the dedup set converges); stderr: {}",
        String::from_utf8_lossy(&serve.stderr),
    );
    let processed: Value = serde_json::from_slice(&serve.stdout).expect("json");
    assert_eq!(processed["ok"], false, "ok=false signals pre-flight stop");
    assert_eq!(processed["stop_reason"], "research_tools_unavailable");
    assert_eq!(processed["steps_completed"], 0);
    let detail = processed["stop_detail"].as_str().expect("stop_detail");
    assert!(
        detail.contains("web_search") && detail.contains("web_extract"),
        "detail must name both denied tools; got: {detail}"
    );

    // completed_jobs row exists (so the queue dedup converges).
    let by_body = run_argo_with_verticals(
        home.path(),
        work.path(),
        verticals.path(),
        &[
            "memory",
            "list",
            "--vertical",
            "no-research",
            "--body-contains",
            "completed_jobs",
            "--json",
        ],
    );
    assert!(by_body.status.success());
    let rows: Vec<Value> = String::from_utf8_lossy(&by_body.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    assert!(!rows.is_empty(), "completed_jobs row must land");

    // Artifact directory still landed — operators can still see
    // WHY the job stopped.
    let output_dir_str = processed["output_dir"]
        .as_str()
        .expect("output_dir present even on pre-flight stop");
    let output_dir = PathBuf::from(output_dir_str);
    assert!(output_dir.exists());
    let approval = std::fs::read_to_string(output_dir.join("approval_checklist.md"))
        .expect("approval_checklist.md must exist");
    assert!(
        approval.contains("stop_reason: `research_tools_unavailable`"),
        "approval checklist must surface the stop reason; got:\n{approval}"
    );
    let _ = job_id; // silence dead-code if removed later
}
