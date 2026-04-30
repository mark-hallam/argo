//! End-to-end tests for the `--json` flag on the read-side CLI.
//!
//! Sets up a smoke workspace, inserts data via append/edit, and
//! confirms the JSON-mode output is valid NDJSON / JSON that
//! parses cleanly with serde_json. Each assertion pins a specific
//! field name + type so a future schema change is caught before
//! it ships.
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

fn parse_uuid_after(prefix: &str, stdout: &str) -> String {
    let line = stdout
        .lines()
        .find(|l| l.starts_with(prefix))
        .unwrap_or_else(|| panic!("no '{prefix}' line in:\n{stdout}"));
    let rest = line.trim_start_matches(prefix).trim_start();
    rest.split_whitespace()
        .next()
        .unwrap_or_else(|| panic!("no token after '{prefix}': {line}"))
        .to_string()
}

fn parse_ndjson(stdout: &str) -> Vec<serde_json::Value> {
    stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            serde_json::from_str::<serde_json::Value>(l)
                .unwrap_or_else(|e| panic!("not valid JSON: {e}\nline: {l}"))
        })
        .collect()
}

fn ws() -> (tempfile::TempDir, tempfile::TempDir) {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let init = run(workspace.path(), home.path(), &["workspace", "init"]);
    assert!(init.status.success());
    (workspace, home)
}

#[test]
fn memory_list_json_emits_valid_ndjson() {
    let (workspace, home) = ws();
    let append = run(
        workspace.path(),
        home.path(),
        &["memory", "append", "memory", "first entry"],
    );
    assert!(append.status.success());
    let id = parse_uuid_after("appended", &String::from_utf8_lossy(&append.stdout));

    let out = run(workspace.path(), home.path(), &["memory", "list", "--json"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let rows = parse_ndjson(&stdout);
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert_eq!(r["id"], id);
    assert_eq!(r["file"], "Memory");
    assert_eq!(r["bytes"], 11); // "first entry"
    assert!(r["hash"].as_str().unwrap().len() == 64); // SHA-256 hex
}

#[test]
fn memory_view_json_includes_body() {
    let (workspace, home) = ws();
    let append = run(
        workspace.path(),
        home.path(),
        &["memory", "append", "user", "hello USER.md"],
    );
    let id = parse_uuid_after("appended", &String::from_utf8_lossy(&append.stdout));
    let out = run(
        workspace.path(),
        home.path(),
        &["memory", "view", &id, "--json"],
    );
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v["id"], id);
    assert_eq!(v["file"], "User");
    assert_eq!(v["body"], "hello USER.md");
}

#[test]
fn memory_list_file_filter_scopes_correctly() {
    let (workspace, home) = ws();
    run(
        workspace.path(),
        home.path(),
        &["memory", "append", "memory", "in MEMORY"],
    );
    run(
        workspace.path(),
        home.path(),
        &["memory", "append", "user", "in USER"],
    );
    let out = run(
        workspace.path(),
        home.path(),
        &["memory", "list", "--json", "--file", "user"],
    );
    let rows = parse_ndjson(&String::from_utf8_lossy(&out.stdout));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["file"], "User");
}

#[test]
fn memory_verify_all_succeeds_on_clean_workspace() {
    let (workspace, home) = ws();
    run(
        workspace.path(),
        home.path(),
        &["memory", "append", "memory", "alpha"],
    );
    run(
        workspace.path(),
        home.path(),
        &["memory", "append", "user", "beta"],
    );
    let out = run(
        workspace.path(),
        home.path(),
        &["memory", "verify", "--all"],
    );
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("all 2 memory entries verified"));
}

#[test]
fn audit_verify_json_on_empty_chain_returns_ok_zero() {
    let (workspace, home) = ws();
    let out = run(
        workspace.path(),
        home.path(),
        &["audit", "verify", "--json"],
    );
    assert!(out.status.success());
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out.stdout).trim()).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["rows"], 0);
}

#[test]
fn costs_show_json_emits_expected_keys() {
    let (workspace, home) = ws();
    let out = run(workspace.path(), home.path(), &["costs", "show", "--json"]);
    assert!(out.status.success());
    let v: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out.stdout).trim()).unwrap();
    for key in [
        "per_session_usd_micros",
        "per_day_usd_micros",
        "per_month_usd_micros",
        "notify_at_pct",
    ] {
        assert!(v.get(key).is_some(), "missing key {key}");
    }
}
