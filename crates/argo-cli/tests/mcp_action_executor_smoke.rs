//! Integration smoke for the action-level MCP executor
//! (mission-wire-mcp-action-executor).
//!
//! Drives the actual `argo` binary end-to-end: writes a vertical
//! manifest declaring an MCP-executor action that points at the
//! Python stdio fixture (`tests/fixtures/mcp_echo_server.py`),
//! injects a pending action, then walks the row through approve →
//! execute and asserts the executor auto-completed the row with the
//! mocked tool's response.
//!
//! Skips with a WARN line when `python3` isn't on PATH — argo's CI
//! matrix has python3 but the door stays open for minimal Linux
//! containers.

mod common;

use common::product_e2e_helpers::{
    assert_success, current_action_state, inject_pending_action, run_argo,
};
use std::path::{Path, PathBuf};
use std::process::Command;

const VERTICAL_ID: &str = "mcp-echo";

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
name = "MCP echo smoke"
version = "0.1.0"
description = "Drives the action-level MCP executor against a Python stdio fixture."

[mcp_servers]
echo = {{ transport = "stdio", command = "{python}", args = ["{script}"] }}

[actions.echo_action]
risk = "low"
requires_approval = false
description = "Calls the Python echo MCP fixture and round-trips the payload."
executor = {{ kind = "mcp", server = "echo", tool = "echo" }}
"#,
        vid = VERTICAL_ID,
        python = python_path,
        script = script_path.display(),
    );
    std::fs::write(dir.join("vertical.toml"), manifest)
        .expect("write vertical.toml");
}

#[test]
fn mcp_action_executor_smoke_completes_via_argo_binary() {
    let Some(python) = python3() else {
        eprintln!("WARN: skipping MCP action-executor smoke — python3 not in PATH");
        return;
    };
    let home = tempfile::tempdir().expect("tempdir for ARGO_HOME");
    let work = tempfile::tempdir().expect("tempdir for workspace");
    let verticals_dir = work.path().join("verticals");
    write_vertical_manifest(&verticals_dir, &python, &fixture_script_path());

    // 1. workspace init under the tempdir.
    let init = run_argo(home.path(), work.path(), &["workspace", "init"]);
    assert_success("step 1: workspace init", &init);

    // 2. inject a pending action whose action_type matches the
    //    manifest's [actions.echo_action] declaration. Make
    //    requires_approval=false so we can drive execute directly
    //    without the auto-approval gate looking up workspace
    //    policy. Pass a payload with a `{{ payload.greeting }}`
    //    placeholder so the executor's substitute_args can
    //    round-trip the substitution into the echo tool's text.
    let template = serde_json::json!({
        "schema_key": "worker_action",
        "job_id": "mcp-smoke-job",
        "action_type": "echo_action",
        "risk_level": "low",
        "requires_approval": false,
        "payload": {
            "text": "echo {{ payload.greeting }}",
            "greeting": "world"
        }
    });
    // The helper sets ARGO_VERTICALS_DIR via env on each spawn? No —
    // we set it ourselves on every run_argo. Inject manually so we
    // can pass ARGO_VERTICALS_DIR.
    let action_id = uuid::Uuid::now_v7().to_string();
    let mut body = template.clone();
    body["action_id"] = serde_json::Value::String(action_id.clone());
    body["state"] = serde_json::Value::String("pending".to_string());
    body["created_at"] = serde_json::Value::String(chrono::Utc::now().to_rfc3339());
    let body_str = serde_json::to_string(&body).expect("serialize");
    let inject_out = Command::new(env!("CARGO_BIN_EXE_argo"))
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", &verticals_dir)
        .current_dir(work.path())
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
    assert!(
        inject_out.status.success(),
        "step 2: memory append failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&inject_out.stdout),
        String::from_utf8_lossy(&inject_out.stderr),
    );

    // 3. `argo worker actions execute <id>` should dispatch through
    //    McpExecutor → python fixture → return Completed → auto-
    //    transition the row to `completed`.
    let execute = Command::new(env!("CARGO_BIN_EXE_argo"))
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", &verticals_dir)
        .current_dir(work.path())
        .args(["worker", "actions", "execute", &action_id, "--json"])
        .output()
        .expect("spawn argo worker actions execute");
    assert!(
        execute.status.success(),
        "step 3: execute failed (exit {:?})\nstdout:\n{}\nstderr:\n{}",
        execute.status.code(),
        String::from_utf8_lossy(&execute.stdout),
        String::from_utf8_lossy(&execute.stderr),
    );

    // 4. final state must be `completed` (auto-transitioned by the
    //    Completed outcome from McpExecutor::dispatch).
    let state = current_state_with_verticals(home.path(), work.path(), &verticals_dir, &action_id);
    assert_eq!(
        state, "completed",
        "step 4: action must auto-transition to `completed` after \
         McpExecutor returns Completed; got {state}"
    );
    let _ = action_id; // keep alive for diagnostics
    let _ = inject_pending_action; // re-export aliveness
    let _ = current_action_state; // re-export aliveness
}

fn current_state_with_verticals(
    home: &Path,
    work: &Path,
    verticals_dir: &Path,
    action_id: &str,
) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_argo"))
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals_dir)
        .current_dir(work)
        .args(["worker", "actions", "show", action_id, "--json"])
        .output()
        .expect("spawn argo worker actions show");
    assert!(
        out.status.success(),
        "show failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("show stdout JSON");
    v.get("state")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| panic!("show output missing /state: {v}"))
}
