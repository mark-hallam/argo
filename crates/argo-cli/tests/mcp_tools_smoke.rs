//! Integration smoke for `argo mcp tools <server>`
//! (`claude-mcp-tool-discovery-vis`).
//!
//! Drives the actual `argo` binary against the same Python stdio
//! fixture used by the action-executor smoke (`mcp_echo_server.py`,
//! 3 tools: `echo` / `tool_error` / `slow`) but through the
//! tools-discovery surface.
//!
//! Skips with a WARN line when `python3` isn't on PATH.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

const VERTICAL_ID: &str = "mcp-tools-smoke";

fn fixture_script_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("mcp_echo_server.py")
}

fn python3() -> Option<String> {
    let output = Command::new("which").arg("python3").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn write_vertical_manifest(verticals_dir: &Path, python_path: &str, script_path: &Path) {
    let dir = verticals_dir.join(VERTICAL_ID);
    std::fs::create_dir_all(&dir).expect("verticals/<id> dir");
    let manifest = format!(
        r#"schema_version = "1"

[vertical]
id = "{vid}"
name = "MCP tools-discovery smoke"
version = "0.1.0"
description = "Drives `argo mcp tools` against the Python echo fixture."

[mcp_servers]
echo = {{ transport = "stdio", command = "{python}", args = ["{script}"] }}
"#,
        vid = VERTICAL_ID,
        python = python_path,
        script = script_path.display(),
    );
    std::fs::write(dir.join("vertical.toml"), manifest)
        .expect("write vertical.toml");
}

fn run_tools(verticals_dir: &Path, args: &[&str]) -> std::process::Output {
    let home = tempfile::tempdir().expect("tempdir for ARGO_HOME");
    let work = tempfile::tempdir().expect("tempdir for cwd");
    Command::new(env!("CARGO_BIN_EXE_argo"))
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals_dir)
        .current_dir(work.path())
        .args(args)
        .output()
        .expect("spawn argo mcp tools")
}

#[test]
fn mcp_tools_lists_three_tools_via_argo_binary() {
    let Some(python) = python3() else {
        eprintln!("WARN: skipping argo mcp tools smoke — python3 not in PATH");
        return;
    };
    let verticals_root = tempfile::tempdir().expect("tempdir for verticals/");
    write_vertical_manifest(verticals_root.path(), &python, &fixture_script_path());

    let out = run_tools(
        verticals_root.path(),
        &["mcp", "tools", "echo", "--vertical", VERTICAL_ID],
    );
    assert!(
        out.status.success(),
        "mcp tools markdown failed (exit {:?})\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8(out.stdout).expect("utf-8 stdout");
    assert!(stdout.contains("3 tools discovered."), "header missing tool count:\n{stdout}");
    assert!(stdout.contains("## `echo`"), "echo section missing:\n{stdout}");
    assert!(stdout.contains("## `tool_error`"), "tool_error section missing:\n{stdout}");
    assert!(stdout.contains("## `slow`"), "slow section missing:\n{stdout}");

    // --json emits NDJSON (one JSON object per line)
    let out = run_tools(
        verticals_root.path(),
        &["mcp", "tools", "echo", "--vertical", VERTICAL_ID, "--json"],
    );
    assert!(out.status.success(), "ndjson run failed: {out:?}");
    let ndjson = String::from_utf8(out.stdout).expect("utf-8");
    let lines: Vec<&str> = ndjson.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 3, "expected 3 NDJSON lines, got {} lines:\n{ndjson}", lines.len());
    for line in &lines {
        let v: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("ndjson line not JSON: {e}\nline: {line}"));
        assert!(v.get("name").is_some(), "ndjson missing name: {line}");
        assert!(v.get("input_schema").is_some(), "ndjson missing input_schema: {line}");
        assert!(v.get("description").is_some(), "ndjson missing description: {line}");
    }

    // --filter narrows
    let out = run_tools(
        verticals_root.path(),
        &["mcp", "tools", "echo", "--vertical", VERTICAL_ID, "--filter", "^echo$", "--json"],
    );
    assert!(out.status.success(), "filtered run failed: {out:?}");
    let lines: Vec<String> = String::from_utf8(out.stdout)
        .expect("utf-8")
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    assert_eq!(lines.len(), 1, "filter narrows to 1 line");
    let v: serde_json::Value = serde_json::from_str(&lines[0]).expect("json");
    assert_eq!(v["name"], "echo");

    // --schema-only drops descriptions
    let out = run_tools(
        verticals_root.path(),
        &["mcp", "tools", "echo", "--vertical", VERTICAL_ID, "--json", "--schema-only"],
    );
    assert!(out.status.success(), "schema-only run failed: {out:?}");
    let line0 = String::from_utf8(out.stdout)
        .expect("utf-8")
        .lines()
        .find(|l| !l.is_empty())
        .map(str::to_string)
        .expect("at least one line");
    let v: serde_json::Value = serde_json::from_str(&line0).expect("json");
    assert!(v.get("description").is_none(), "schema-only line must drop description: {line0}");
}

#[test]
fn mcp_tools_unknown_server_surfaces_clean_error() {
    let Some(python) = python3() else {
        eprintln!("WARN: skipping argo mcp tools unknown-server smoke — python3 not in PATH");
        return;
    };
    let verticals_root = tempfile::tempdir().expect("tempdir for verticals/");
    write_vertical_manifest(verticals_root.path(), &python, &fixture_script_path());
    let out = run_tools(
        verticals_root.path(),
        &["mcp", "tools", "does-not-exist", "--vertical", VERTICAL_ID],
    );
    assert!(!out.status.success(), "unknown server must exit non-zero");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("does-not-exist") && stderr.contains("[mcp_servers]"),
        "stderr should call out the missing server, got: {stderr}"
    );
}
