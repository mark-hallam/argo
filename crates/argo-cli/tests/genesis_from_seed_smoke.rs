//! End-to-end smoke for `argo genesis from-vertical` —
//! Mission `mission-genesis-transfer-from-tenant`.
//!
//! Seeds the Genesis pipeline from the in-tree `sales-follow-up`
//! vertical and asserts the resulting manifest pulled in the
//! seed's action vocabulary plus the lineage tag.
//!
//! The brief's notes nominated `sales-sdr` as the seed; that
//! vertical's `memory/schema.toml` uses the legacy `[[scope]]`
//! shape which the current `argo-vertical::memory_schema` parser
//! rejects (pre-existing drift, out of scope for this brief).
//! `sales-follow-up` loads cleanly and has a near-identical
//! shape — see the PR body's "Sensible deviations" note.

use std::path::Path;
use std::process::{Command, Output};

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

/// Path to the in-tree `verticals/` dir, resolved from
/// `CARGO_MANIFEST_DIR` (which is the argo-cli crate root). This
/// keeps the test independent of the caller's cwd.
fn workspace_verticals_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("verticals")
}

fn run(workspace: &Path, home: &Path, verticals_dir: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .current_dir(workspace)
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals_dir)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

fn assert_ok(out: &Output, label: &str) {
    assert!(
        out.status.success(),
        "{label} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn from_vertical_seeds_actions_from_sales_follow_up() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let verticals_dir = workspace_verticals_dir();
    assert!(
        verticals_dir.join("sales-follow-up/vertical.toml").is_file(),
        "expected sales-follow-up seed at {}",
        verticals_dir.display()
    );

    let charter_path = workspace.path().join("charter.txt");
    std::fs::write(
        &charter_path,
        "be my legal-paralegal, screen new client intake forms and draft initial responses",
    )
    .unwrap();
    let output_dir = workspace.path().join("out");

    let out = run(
        workspace.path(),
        home.path(),
        &verticals_dir,
        &[
            "genesis",
            "from-vertical",
            charter_path.to_str().unwrap(),
            "--seed",
            "sales-follow-up",
            "--persona",
            "luna",
            "--output",
            output_dir.to_str().unwrap(),
            "--json",
        ],
    );
    assert_ok(&out, "genesis from-vertical --json");

    let envelope: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout was not JSON: {e}\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        )
    });
    assert_eq!(
        envelope["seeded_from"].as_str(),
        Some("sales-follow-up"),
        "lineage tag must surface in --json envelope: {envelope}"
    );
    let action_count = envelope["action_count"].as_u64().unwrap_or(0);
    assert!(action_count > 0, "expected non-empty manifest: {envelope}");

    // Inspect the produced manifest on disk — it must contain a
    // sales-follow-up action that the baseline's archetype would
    // not ship. `send_followup_email` is the canonical
    // sales-follow-up action and doesn't appear in any
    // non-sales-follow-up archetype's seed vocab.
    let manifest_path = output_dir.join("vertical.toml");
    let body = std::fs::read_to_string(&manifest_path).unwrap_or_else(|e| {
        panic!("expected manifest at {}: {e}", manifest_path.display())
    });
    assert!(
        body.contains("[actions.send_followup_email]"),
        "manifest missing seed's send_followup_email action:\n{body}"
    );
    assert!(
        body.contains("seeded-from:sales-follow-up"),
        "manifest missing seeded-from lineage tag:\n{body}"
    );
}
