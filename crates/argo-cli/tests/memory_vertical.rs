//! Integration tests for `argo memory append/list --vertical`
//! (#686 — surfaces the #680 vertical-namespaced storage to
//! the operator CLI).

mod common;

use common::argo_bin;
use std::process::Command;

fn workspace_init(home: &std::path::Path, root: &std::path::Path) {
    // `workspace init` works on cwd — no flags. Run it from
    // the test's tempdir so it writes argo.toml there.
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .current_dir(root)
        .args(["workspace", "init"])
        .output()
        .expect("spawn workspace init");
    assert!(
        out.status.success(),
        "workspace init stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn append(
    home: &std::path::Path,
    root: &std::path::Path,
    file: &str,
    body: &str,
    vertical: Option<&str>,
) {
    let mut args: Vec<String> = vec![
        "memory".into(),
        "append".into(),
        file.into(),
        body.into(),
    ];
    if let Some(v) = vertical {
        args.push("--vertical".into());
        args.push(v.into());
    }
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .current_dir(root)
        .args(&args)
        .output()
        .expect("spawn memory append");
    assert!(
        out.status.success(),
        "append stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Parse the list JSON output (one object per line — NDJSON-ish)
/// into a Vec of values for assertions.
fn parse_list(stdout: &str) -> Vec<serde_json::Value> {
    stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("valid JSON"))
        .collect()
}

#[test]
fn cli_memory_append_with_vertical_tags_the_row() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    workspace_init(home.path(), root.path());

    append(
        home.path(),
        root.path(),
        "memory",
        "alpha-only-secret",
        Some("alpha"),
    );

    // List with --vertical alpha → sees exactly the alpha-tagged row.
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .current_dir(root.path())
        .args(["memory", "list", "--vertical", "alpha", "--json"])
        .output()
        .expect("spawn list");
    assert!(out.status.success());
    let rows = parse_list(&String::from_utf8(out.stdout).unwrap());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["vertical_id"], "alpha");
}

#[test]
fn cli_memory_list_with_vertical_excludes_other_verticals_rows() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    workspace_init(home.path(), root.path());

    append(home.path(), root.path(), "memory", "alpha-row", Some("alpha"));
    append(home.path(), root.path(), "memory", "beta-row", Some("beta"));

    // alpha view sees only alpha's row.
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .current_dir(root.path())
        .args(["memory", "list", "--vertical", "alpha", "--json"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let rows = parse_list(&String::from_utf8(out.stdout).unwrap());
    assert_eq!(rows.len(), 1, "alpha view must contain exactly its row");
    assert_eq!(rows[0]["vertical_id"], "alpha");

    // beta view sees only beta's row.
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .current_dir(root.path())
        .args(["memory", "list", "--vertical", "beta", "--json"])
        .output()
        .expect("spawn");
    let rows = parse_list(&String::from_utf8(out.stdout).unwrap());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["vertical_id"], "beta");
}

#[test]
fn cli_memory_list_with_vertical_excludes_legacy_workspace_global_rows() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    workspace_init(home.path(), root.path());

    // Legacy write — no --vertical → vertical_id = NULL.
    append(home.path(), root.path(), "memory", "legacy-row", None);

    // alpha-scoped view must be EMPTY (default-deny excludes legacy NULL).
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .current_dir(root.path())
        .args(["memory", "list", "--vertical", "alpha", "--json"])
        .output()
        .expect("spawn");
    let rows = parse_list(&String::from_utf8(out.stdout).unwrap());
    assert!(
        rows.is_empty(),
        "vertical scope must exclude legacy NULL row; got {rows:?}"
    );

    // The legacy `memory list` (no --vertical) still sees it.
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .current_dir(root.path())
        .args(["memory", "list", "--json"])
        .output()
        .expect("spawn");
    let rows = parse_list(&String::from_utf8(out.stdout).unwrap());
    assert_eq!(rows.len(), 1, "legacy list-all must keep showing NULL rows");
    assert!(rows[0]["vertical_id"].is_null());
}

#[test]
fn cli_memory_list_unknown_vertical_returns_empty() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    workspace_init(home.path(), root.path());

    append(home.path(), root.path(), "memory", "alpha-row", Some("alpha"));

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .current_dir(root.path())
        .args(["memory", "list", "--vertical", "ghost", "--json"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let rows = parse_list(&String::from_utf8(out.stdout).unwrap());
    assert!(rows.is_empty(), "ghost-vertical view must be empty; got {rows:?}");
}
