//! Integration smoke for the action-level API executor.

mod common;

use common::product_e2e_helpers::{assert_success, run_argo};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

const VERTICAL_ID: &str = "api-echo";

fn fixture_script_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("api_echo_server.py")
}

fn python3() -> Option<String> {
    let output = Command::new("which").arg("python3").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!path.is_empty()).then_some(path)
}

fn start_server(python: &str) -> (Child, u16) {
    let mut child = Command::new(python)
        .arg(fixture_script_path())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn api echo fixture");
    let stdout = child.stdout.take().expect("fixture stdout");
    let mut line = String::new();
    BufReader::new(stdout)
        .read_line(&mut line)
        .expect("fixture port line");
    let port = line.trim().parse::<u16>().expect("fixture port");
    (child, port)
}

fn write_vertical_manifest(verticals_dir: &Path, port: u16) {
    let dir = verticals_dir.join(VERTICAL_ID);
    std::fs::create_dir_all(&dir).expect("verticals/<id> dir");
    let binding = serde_json::json!({
        "url": format!("http://127.0.0.1:{port}/run/{{{{ action_id }}}}"),
        "method": "POST",
        "headers": {"X-Api-Smoke": "{{ secret.API_SMOKE_TOKEN }}"},
        "body_template": r#"{"message":"{{ payload.message }}","id":"{{ action_id }}"}"#,
    });
    let manifest = format!(
        r#"schema_version = "1"

[vertical]
id = "{vid}"
name = "API echo smoke"
version = "0.1.0"
description = "Drives the action-level API executor against a Python HTTP fixture."

[actions.echo_api]
risk = "low"
requires_approval = false
description = "Calls the Python HTTP echo fixture and round-trips the payload."
executor = {{ kind = "api", binding = '''{binding}''' }}
"#,
        vid = VERTICAL_ID,
        binding = binding,
    );
    std::fs::write(dir.join("vertical.toml"), manifest).expect("write vertical.toml");
}

#[test]
fn api_action_executor_smoke_completes_via_argo_binary() {
    let Some(python) = python3() else {
        eprintln!("WARN: skipping API action-executor smoke — python3 not in PATH");
        return;
    };
    let (mut server, port) = start_server(&python);
    let home = tempfile::tempdir().expect("tempdir for ARGO_HOME");
    let work = tempfile::tempdir().expect("tempdir for workspace");
    let verticals_dir = work.path().join("verticals");
    write_vertical_manifest(&verticals_dir, port);

    let init = run_argo(home.path(), work.path(), &["workspace", "init"]);
    assert_success("step 1: workspace init", &init);

    let action_id = uuid::Uuid::now_v7().to_string();
    let body = serde_json::json!({
        "schema_key": "worker_action",
        "action_id": action_id,
        "job_id": "api-smoke-job",
        "action_type": "echo_api",
        "risk_level": "low",
        "requires_approval": false,
        "state": "pending",
        "payload": {"message": "hello-api"},
        "created_at": chrono::Utc::now().to_rfc3339(),
    });
    let body_str = serde_json::to_string(&body).expect("serialize action");
    let append = Command::new(env!("CARGO_BIN_EXE_argo"))
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
    assert_success("step 2: memory append", &append);

    let execute = Command::new(env!("CARGO_BIN_EXE_argo"))
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", &verticals_dir)
        .env("API_SMOKE_TOKEN", "smoke-secret")
        .current_dir(work.path())
        .args(["worker", "actions", "execute", &action_id, "--json"])
        .output()
        .expect("spawn argo worker actions execute");
    assert_success("step 3: execute", &execute);

    let state = current_state(home.path(), work.path(), &verticals_dir, &action_id);
    assert_eq!(state, "completed");
    let _ = server.kill();
}

fn current_state(home: &Path, work: &Path, verticals_dir: &Path, action_id: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_argo"))
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals_dir)
        .current_dir(work)
        .args(["worker", "actions", "show", action_id, "--json"])
        .output()
        .expect("spawn argo worker actions show");
    assert_success("step 4: show", &out);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("show stdout JSON");
    v.get("state")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| panic!("show output missing /state: {v}"))
}
