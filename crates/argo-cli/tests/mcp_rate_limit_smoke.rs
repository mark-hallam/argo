//! Integration smoke for MCP dispatch rate limiting.

use std::path::{Path, PathBuf};
use std::process::Command;

const VERTICAL_ID: &str = "mcp-rate-limit-smoke";

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
    (!s.is_empty()).then_some(s)
}

fn run_argo(home: &Path, cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_argo"))
        .env("ARGO_HOME", home)
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("spawn argo")
}

fn write_vertical_manifest(verticals_dir: &Path, python: &str, script: &Path) {
    let dir = verticals_dir.join(VERTICAL_ID);
    std::fs::create_dir_all(&dir).expect("vertical dir");
    let manifest = format!(
        r#"schema_version = "1"

[vertical]
id = "{vid}"
name = "MCP rate-limit smoke"
version = "0.1.0"

[mcp_servers]
echo = {{ transport = "stdio", command = "{python}", args = ["{script}"] }}

[actions.echo_action]
risk = "low"
requires_approval = false
description = "Calls the Python echo MCP fixture."
executor = {{ kind = "mcp", server = "echo", tool = "echo" }}
"#,
        vid = VERTICAL_ID,
        python = python,
        script = script.display(),
    );
    std::fs::write(dir.join("vertical.toml"), manifest).expect("write vertical");
}

fn write_rate_limit(path: &Path) {
    std::fs::write(
        path,
        r#"
default_capacity = 60
default_refill_rate_per_sec = 10
max_wait_ms = 5

[rate_limit.echo]
capacity = 0
refill_rate_per_sec = 0
"#,
    )
    .expect("write rate limit config");
}

fn append_action(home: &Path, work: &Path, verticals: &Path, action_id: &str) {
    let body = serde_json::json!({
        "schema_key": "worker_action",
        "action_id": action_id,
        "job_id": "mcp-rate-limit-job",
        "action_type": "echo_action",
        "risk_level": "low",
        "requires_approval": false,
        "state": "pending",
        "created_at": chrono::Utc::now().to_rfc3339(),
        "payload": {"text": "hello"},
    });
    let out = Command::new(env!("CARGO_BIN_EXE_argo"))
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals)
        .current_dir(work)
        .args([
            "memory",
            "append",
            "--vertical",
            VERTICAL_ID,
            "memory",
            &body.to_string(),
        ])
        .output()
        .expect("spawn memory append");
    assert!(out.status.success(), "append failed: {out:?}");
}

#[test]
fn mcp_dispatch_returns_clean_rate_limit_error() {
    let Some(python) = python3() else {
        eprintln!("WARN: skipping MCP rate-limit smoke - python3 not in PATH");
        return;
    };
    let home = tempfile::tempdir().expect("home");
    let work = tempfile::tempdir().expect("work");
    let verticals = work.path().join("verticals");
    let rate_path = work.path().join("mcp_rate_limit.toml");
    write_vertical_manifest(&verticals, &python, &fixture_script_path());
    write_rate_limit(&rate_path);
    let init = run_argo(home.path(), work.path(), &["workspace", "init"]);
    assert!(init.status.success(), "workspace init failed: {init:?}");

    let action_id = uuid::Uuid::now_v7().to_string();
    append_action(home.path(), work.path(), &verticals, &action_id);
    let out = Command::new(env!("CARGO_BIN_EXE_argo"))
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", &verticals)
        .env("ARGO_MCP_RATE_LIMIT_PATH", &rate_path)
        .current_dir(work.path())
        .args(["worker", "actions", "execute", &action_id, "--json"])
        .output()
        .expect("spawn execute");
    assert!(!out.status.success(), "execute should be rate-limited");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("rate limit exceeded for echo"), "{stderr}");
}
