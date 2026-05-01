//! End-to-end smoke for status / cost / health markdown
//! renders. Lives in its own file so test additions don't push
//! `end_to_end.rs` (workspace+memory) or
//! `end_to_end_session.rs` (session lifecycle) over the
//! 500-LOC budget.
use std::path::Path;
use std::process::{Command, Output};

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn run(workspace: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .current_dir(workspace)
        .env("ARGO_HOME", home)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

/// `argo workspace status --format markdown` (#499) renders
/// the four-section status doc.
#[cfg(unix)]
#[test]
fn workspace_status_markdown_renders_four_sections() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert!(run(workspace.path(), home.path(), &["workspace", "init"])
        .status
        .success());

    let out = run(
        workspace.path(),
        home.path(),
        &["workspace", "status", "--format", "markdown"],
    );
    assert!(out.status.success());
    let body = String::from_utf8_lossy(&out.stdout);

    assert!(body.contains("# Workspace status"));
    assert!(body.contains("## Caps (USD)"));
    assert!(body.contains("## Rows"));
    assert!(body.contains("**cwd**: `"));
    assert!(body.contains("**ARGO_HOME**: `"));
}

/// `argo doctor --format markdown` (#478) renders 5 sections.
#[cfg(unix)]
#[test]
fn doctor_markdown_renders_five_sections() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert!(run(workspace.path(), home.path(), &["workspace", "init"])
        .status
        .success());

    let out = run(workspace.path(), home.path(), &["doctor", "--format", "markdown"]);
    assert!(out.status.success());
    let body = String::from_utf8_lossy(&out.stdout);

    assert!(body.contains("# argo doctor"));
    assert!(body.contains("## Workspace"));
    assert!(body.contains("## Database"));
    assert!(body.contains("## Audit chain"));
    assert!(body.contains("## Daemon / runtime"));
    assert!(body.contains("## Credentials"));
}

/// `argo health --format markdown` (#489) renders 3 sections
/// with ✓ markers when checks pass.
#[cfg(unix)]
#[test]
fn health_markdown_renders_status_and_three_sections() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert!(run(workspace.path(), home.path(), &["workspace", "init"])
        .status
        .success());

    let out = run(workspace.path(), home.path(), &["health", "--format", "markdown"]);
    assert!(out.status.success());
    let body = String::from_utf8_lossy(&out.stdout);

    assert!(body.contains("# argo health"));
    assert!(body.contains("**Status**: all checks ok ✓"));
    assert!(body.contains("## Audit chain"));
    assert!(body.contains("## Skill provenance"));
    assert!(body.contains("## Memory provenance"));
}

/// `argo costs show --format markdown` (#477) renders ceilings
/// + spent sections.
#[cfg(unix)]
#[test]
fn costs_show_markdown_renders_ceilings_and_spent() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert!(run(workspace.path(), home.path(), &["workspace", "init"])
        .status
        .success());

    let out = run(
        workspace.path(),
        home.path(),
        &["costs", "show", "--format", "markdown"],
    );
    assert!(out.status.success());
    let body = String::from_utf8_lossy(&out.stdout);

    assert!(body.contains("# Costs"));
    assert!(body.contains("## Ceilings (USD)"));
    assert!(body.contains("## Spent so far (USD, all tenants)"));
    assert!(body.contains("**per-session**:"));
    assert!(body.contains("**Today**:"));
}

/// `argo tools list` (#525) returns all 8 in-core tools.
/// Pure offline — no daemon required.
#[cfg(unix)]
#[test]
fn tools_list_returns_eight_in_core_tools() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let out = run(workspace.path(), home.path(), &["tools", "list", "--json"]);
    assert!(out.status.success(), "tools list failed");
    let body = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<_> = body.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 8, "expected 8 tools, got {body}");

    // Check the canonical 8 are present.
    for expected in [
        "delegate_task",
        "file_patch",
        "file_read",
        "file_write",
        "memory",
        "terminal",
        "web_extract",
        "web_search",
    ] {
        assert!(
            body.contains(&format!("\"name\":\"{expected}\"")),
            "missing tool {expected}: {body}"
        );
    }
}

/// `argo tools show <name>` returns the schema for one tool;
/// markdown by default, JSON with `--json`.
#[cfg(unix)]
#[test]
fn tools_show_renders_markdown_default_and_json_flag() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let out = run(
        workspace.path(),
        home.path(),
        &["tools", "show", "file_read"],
    );
    assert!(out.status.success());
    let body = String::from_utf8_lossy(&out.stdout);
    assert!(body.contains("# file_read"), "default should be md: {body}");
    assert!(body.contains("## Parameters"));

    let out = run(
        workspace.path(),
        home.path(),
        &["tools", "show", "file_read", "--json"],
    );
    let body = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(body.trim()).expect("valid JSON");
    assert_eq!(v["name"], "file_read");

    // Unknown tool errors cleanly.
    let out = run(
        workspace.path(),
        home.path(),
        &["tools", "show", "nonexistent"],
    );
    assert!(!out.status.success(), "missing tool must fail");
}

/// `argo db status --format markdown` (#500) renders the
/// schema header + per-table row counts table.
#[cfg(unix)]
#[test]
fn db_status_markdown_renders_schema_and_table() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert!(run(workspace.path(), home.path(), &["workspace", "init"])
        .status
        .success());

    let out = run(
        workspace.path(),
        home.path(),
        &["db", "status", "--format", "markdown"],
    );
    assert!(out.status.success());
    let body = String::from_utf8_lossy(&out.stdout);

    assert!(body.contains("# Database status"));
    assert!(body.contains("**Schema**: v"));
    assert!(body.contains("## Rows per table"));
    assert!(body.contains("`sessions`"));
    assert!(body.contains("`spend_ledger`"));
}
