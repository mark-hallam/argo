//! Integration smoke for the action-level Browser executor.
//!
//! Spawns the `browser_mock_mcp.py` stdio fixture as the browser
//! MCP server (via `ARGO_BROWSER_MCP_BIN`), authors a vertical
//! whose `executor = { kind = "browser", binding = ... }` action
//! drives the mock, appends a worker_action through the running
//! `argo` binary, executes it via `argo worker actions execute`,
//! and asserts the action auto-completes (Completed outcome).
//!
//! Mirrors the api_action_executor_smoke shape so the surface is
//! consistent across executor families.

mod common;

use common::product_e2e_helpers::{assert_success, run_argo};
use std::path::{Path, PathBuf};
use std::process::Command;

const VERTICAL_ID: &str = "browser-mock";

fn fixture_script_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("browser_mock_mcp.py")
}

fn python3() -> Option<String> {
    let output = Command::new("which").arg("python3").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!path.is_empty()).then_some(path)
}

fn write_vertical_manifest(verticals_dir: &Path) {
    let dir = verticals_dir.join(VERTICAL_ID);
    std::fs::create_dir_all(&dir).expect("verticals/<id> dir");
    let binding = serde_json::json!({
        "target_url": "https://example.com/post",
        "mode": "script",
        "session_scope": "per_worker",
        "steps": [
            {"action": "fill", "selector": "#title", "value": "{{ payload.title }}"},
            {"action": "click", "selector": "#post"},
        ],
    });
    let manifest = format!(
        r#"schema_version = "1"

[vertical]
id = "{vid}"
name = "Browser smoke"
version = "0.1.0"
description = "Drives the action-level Browser executor against a Python stdio MCP fixture."

[actions.post_listing]
risk = "low"
requires_approval = false
description = "Posts a listing through the mocked browser MCP."
executor = {{ kind = "browser", binding = '''{binding}''' }}
"#,
        vid = VERTICAL_ID,
        binding = binding,
    );
    std::fs::write(dir.join("vertical.toml"), manifest).expect("write vertical.toml");
}

fn append_action(home: &Path, work: &Path, verticals_dir: &Path) -> String {
    let action_id = uuid::Uuid::now_v7().to_string();
    let body = serde_json::json!({
        "schema_key": "worker_action",
        "action_id": action_id,
        "job_id": "browser-smoke-job",
        "action_type": "post_listing",
        "risk_level": "low",
        "requires_approval": false,
        "state": "pending",
        "payload": {"title": "Vintage lamp"},
        "created_at": chrono::Utc::now().to_rfc3339(),
    });
    let body_str = serde_json::to_string(&body).expect("serialize action");
    let append = Command::new(env!("CARGO_BIN_EXE_argo"))
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals_dir)
        .current_dir(work)
        .args([
            "memory",
            "append",
            "--vertical",
            VERTICAL_ID,
            "memory",
            &body_str,
        ])
        .output()
        .expect("spawn argo memory append");
    assert_success("memory append post_listing", &append);
    action_id
}

fn current_state(home: &Path, work: &Path, verticals_dir: &Path, action_id: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_argo"))
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals_dir)
        .current_dir(work)
        .args(["worker", "actions", "show", action_id, "--json"])
        .output()
        .expect("spawn argo worker actions show");
    assert_success("show", &out);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("show stdout JSON");
    v.get("state")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| panic!("show output missing /state: {v}"))
}

#[test]
fn browser_action_executor_smoke_completes_via_argo_binary() {
    let Some(python) = python3() else {
        eprintln!("WARN: skipping browser action-executor smoke — python3 not in PATH");
        return;
    };
    let home = tempfile::tempdir().expect("tempdir for ARGO_HOME");
    let work = tempfile::tempdir().expect("tempdir for workspace");
    let verticals_dir = work.path().join("verticals");
    write_vertical_manifest(&verticals_dir);

    let init = run_argo(home.path(), work.path(), &["workspace", "init"]);
    assert_success("step 1: workspace init", &init);

    let action_id = append_action(home.path(), work.path(), &verticals_dir);
    let bin_spec = format!("{} {}", python, fixture_script_path().display());
    let execute = Command::new(env!("CARGO_BIN_EXE_argo"))
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", &verticals_dir)
        .env("ARGO_BROWSER_MCP_BIN", &bin_spec)
        .current_dir(work.path())
        .args(["worker", "actions", "execute", &action_id, "--json"])
        .output()
        .expect("spawn argo worker actions execute");
    assert_success("execute", &execute);
    let state = current_state(home.path(), work.path(), &verticals_dir, &action_id);
    assert_eq!(
        state, "completed",
        "browser action should auto-complete on Completed outcome"
    );
}
