//! Integration smoke for the gateway MCP health-check task.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

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

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral")
        .local_addr()
        .expect("local addr")
        .port()
}

fn write_slow_server(path: &Path) {
    std::fs::write(
        path,
        r#"#!/usr/bin/env python3
import json, sys, time
PROTOCOL_VERSION = "2024-11-05"
while True:
    line = sys.stdin.readline()
    if not line:
        break
    req = json.loads(line)
    mid = req.get("id")
    if mid is None:
        continue
    method = req.get("method")
    if method == "initialize":
        result = {"protocolVersion": PROTOCOL_VERSION, "capabilities": {"tools": {}}, "serverInfo": {"name": "slow", "version": "0.1.0"}}
    elif method == "tools/list":
        time.sleep(0.05)
        result = {"tools": [{"name": "slow", "description": "slow list", "inputSchema": {"type": "object"}}]}
    else:
        result = {}
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": mid, "result": result}) + "\n")
    sys.stdout.flush()
"#,
    )
    .expect("write slow mcp fixture");
}

fn write_vertical_manifest(root: &Path, python: &str, slow: &Path) {
    let dir = root.join("mcp-health-smoke");
    std::fs::create_dir_all(&dir).expect("vertical dir");
    let manifest = format!(
        r#"schema_version = "1"

[vertical]
id = "mcp-health-smoke"
name = "MCP health smoke"
version = "0.1.0"

[mcp_servers]
fast = {{ transport = "stdio", command = "{python}", args = ["{fast}"] }}
slow = {{ transport = "stdio", command = "{python}", args = ["{slow}"] }}
dead = {{ transport = "stdio", command = "/tmp/skycot-missing-mcp-server", args = [] }}
"#,
        python = python,
        fast = fixture_script_path().display(),
        slow = slow.display(),
    );
    std::fs::write(dir.join("vertical.toml"), manifest).expect("write vertical.toml");
}

fn hit_health(port: u16) -> bool {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) {
            let _ = stream.write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n");
            let mut body = String::new();
            let _ = stream.read_to_string(&mut body);
            return body.contains("200 OK");
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    false
}

#[test]
fn gateway_health_task_starts_with_fast_slow_and_dead_servers() {
    let Some(python) = python3() else {
        eprintln!("WARN: skipping mcp health smoke - python3 not in PATH");
        return;
    };
    let home = tempfile::tempdir().expect("ARGO_HOME");
    let work = tempfile::tempdir().expect("cwd");
    let verticals = tempfile::tempdir().expect("verticals");
    let slow = work.path().join("slow_mcp.py");
    write_slow_server(&slow);
    write_vertical_manifest(verticals.path(), &python, &slow);

    let port = free_port();
    let mut child = Command::new(env!("CARGO_BIN_EXE_argo"))
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .env("ARGO_MCP_HEALTH_CHECK", "1")
        .env("ARGO_MCP_HEALTH_INTERVAL_MS", "50")
        .env("ARGO_MCP_HEALTH_TIMEOUT_MS", "500")
        .env("ARGO_MCP_HEALTH_MAX_CONCURRENT", "2")
        .env("RUST_LOG", "info")
        .current_dir(work.path())
        .args([
            "gateway",
            "start",
            "--bind",
            "127.0.0.1",
            "--port",
            &port.to_string(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway");

    if !hit_health(port) {
        let _ = child.kill();
        panic!("gateway did not answer /health");
    }
    std::thread::sleep(Duration::from_millis(1200));
    let _ = child.kill();
    let output = child.wait_with_output().expect("wait gateway");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("mcp health check enabled"), "{stderr}");
    assert!(!stderr.contains("mcp.health task stopped"), "{stderr}");
}
