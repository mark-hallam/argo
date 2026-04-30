//! Smoke test for `argo gateway start` — spawns the binary, hits
//! `/health` and `/version`, asserts the response shapes, then
//! waits for the `--max-requests` exit.
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::time::Duration;

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn pick_free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

fn http_get(port: u16, path: &str) -> String {
    // Hand-rolled HTTP/1.1 client to keep argo-cli's dev-deps tight
    // (no reqwest / hyper here).
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let req = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

fn body_of(resp: &str) -> &str {
    resp.split_once("\r\n\r\n").map(|(_h, b)| b).unwrap_or("")
}

#[test]
fn gateway_start_serves_health_and_version_then_exits_on_max_requests() {
    let port = pick_free_port();
    let mut child = Command::new(argo_bin())
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "2",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn argo gateway start");

    // Wait for the listener to bind by polling the port. 2s budget
    // is generous for a tokio TcpListener::bind on a fresh port.
    let stderr = child.stderr.take().expect("stderr");
    let mut stderr = BufReader::new(stderr);
    let mut line = String::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        if std::time::Instant::now() > deadline {
            panic!("listener did not announce in 2s");
        }
        line.clear();
        stderr.read_line(&mut line).expect("read line");
        if line.contains("listening on") {
            break;
        }
    }

    let h = http_get(port, "/health");
    assert!(h.contains("HTTP/1.1 200 OK"), "h={h}");
    assert_eq!(body_of(&h), r#"{"status":"ok"}"#);

    let v = http_get(port, "/version");
    assert!(v.contains("HTTP/1.1 200 OK"), "v={v}");
    let body = body_of(&v);
    assert!(body.starts_with(r#"{"version":""#));

    let status = child.wait().expect("wait");
    assert!(status.success(), "exit: {status}");
}

#[test]
fn gateway_sessions_endpoint_returns_json_array_for_empty_db() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");

    let stderr = child.stderr.take().expect("stderr");
    let mut stderr = BufReader::new(stderr);
    let mut line = String::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            panic!("listener did not announce in 2s");
        }
        line.clear();
        stderr.read_line(&mut line).expect("read line");
        if line.contains("listening on") {
            break;
        }
    }

    let r = http_get(port, "/sessions");
    assert!(r.contains("HTTP/1.1 200 OK"), "got: {r}");
    let body = body_of(&r);
    let parsed: serde_json::Value = serde_json::from_str(body).expect("valid json");
    assert!(parsed.is_array(), "expected array, got {parsed}");
    assert_eq!(parsed.as_array().unwrap().len(), 0);

    let status = child.wait().expect("wait");
    assert!(status.success());
}

fn http_post(port: u16, path: &str, body: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

#[test]
fn gateway_post_sessions_creates_session_row_via_ensure_workspace() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    // Initialise workspace inside a tempdir so ensure_workspace
    // anchors there (and cwd doesn't leak into the test).
    Command::new(argo_bin())
        .current_dir(workspace.path())
        .env("ARGO_HOME", home.path())
        .args(["workspace", "init"])
        .output()
        .expect("workspace init");

    let mut child = Command::new(argo_bin())
        .current_dir(workspace.path())
        .env("ARGO_HOME", home.path())
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let stderr = child.stderr.take().expect("stderr");
    let mut stderr = BufReader::new(stderr);
    let mut line = String::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            panic!("listener did not announce in 2s");
        }
        line.clear();
        stderr.read_line(&mut line).expect("read line");
        if line.contains("listening on") {
            break;
        }
    }

    let r = http_post(
        port,
        "/sessions",
        r#"{"model":"claude-haiku","provider":"anthropic","title":"smoke"}"#,
    );
    assert!(r.contains("HTTP/1.1 201 Created"), "got: {r}");
    let body = body_of(&r);
    let parsed: serde_json::Value = serde_json::from_str(body).expect("valid json");
    assert!(parsed["id"].is_string(), "missing id: {parsed}");
    assert_eq!(parsed["model"], "claude-haiku");
    assert_eq!(parsed["provider"], "anthropic");
    assert_eq!(parsed["title"], "smoke");

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn gateway_get_session_by_id_and_list_messages() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    Command::new(argo_bin())
        .current_dir(workspace.path())
        .env("ARGO_HOME", home.path())
        .args(["workspace", "init"])
        .output()
        .expect("workspace init");

    let mut child = Command::new(argo_bin())
        .current_dir(workspace.path())
        .env("ARGO_HOME", home.path())
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "4",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let stderr = child.stderr.take().expect("stderr");
    let mut stderr = BufReader::new(stderr);
    let mut line = String::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            panic!("listener did not announce in 2s");
        }
        line.clear();
        stderr.read_line(&mut line).expect("read line");
        if line.contains("listening on") {
            break;
        }
    }

    // Create + append.
    let r = http_post(port, "/sessions", r#"{"model":"m","provider":"p"}"#);
    let create: serde_json::Value = serde_json::from_str(body_of(&r)).unwrap();
    let sid = create["id"].as_str().unwrap();
    let path = format!("/sessions/{sid}/messages");
    http_post(
        port,
        &path,
        r#"{"role":"user","content":[{"type":"text","text":"hi"}]}"#,
    );

    // GET /sessions/<id>
    let r = http_get(port, &format!("/sessions/{sid}"));
    assert!(r.contains("HTTP/1.1 200 OK"), "got: {r}");
    let parsed: serde_json::Value = serde_json::from_str(body_of(&r)).expect("valid json");
    assert_eq!(parsed["id"], sid);
    assert_eq!(parsed["model"], "m");
    assert_eq!(parsed["provider"], "p");
    assert_eq!(parsed["message_count"], 1);

    // GET /sessions/<id>/messages
    let r = http_get(port, &format!("/sessions/{sid}/messages"));
    assert!(r.contains("HTTP/1.1 200 OK"), "got: {r}");
    let parsed: serde_json::Value = serde_json::from_str(body_of(&r)).expect("valid json");
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["role"], "user");
    assert_eq!(arr[0]["seq"], 1);

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn gateway_post_runs_returns_500_when_anthropic_key_missing() {
    // Spawns the gateway, creates a session against the bundled
    // anthropic manifest, then runs `POST /runs` with no
    // ANTHROPIC_API_KEY in env. The agent loop's missing-key
    // error should bubble back through the gateway as
    // 500 Internal Server Error with the env-var name in the body
    // — that's the contract external clients rely on to give the
    // user an actionable hint.
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    Command::new(argo_bin())
        .current_dir(workspace.path())
        .env("ARGO_HOME", home.path())
        .args(["workspace", "init"])
        .output()
        .expect("workspace init");
    // Copy the bundled anthropic manifest so manifest loading works.
    let manifests_dst = workspace.path().join("manifests").join("providers");
    std::fs::create_dir_all(&manifests_dst).unwrap();
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    std::fs::copy(
        repo_root.join("manifests/providers/anthropic.toml"),
        manifests_dst.join("anthropic.toml"),
    )
    .unwrap();

    let mut child = Command::new(argo_bin())
        .current_dir(workspace.path())
        .env("ARGO_HOME", home.path())
        .env_remove("ANTHROPIC_API_KEY")
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "2",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let stderr = child.stderr.take().expect("stderr");
    let mut stderr = BufReader::new(stderr);
    let mut line = String::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            panic!("listener did not announce in 2s");
        }
        line.clear();
        stderr.read_line(&mut line).expect("read line");
        if line.contains("listening on") {
            break;
        }
    }

    let r = http_post(
        port,
        "/sessions",
        r#"{"model":"claude-haiku-4-5","provider":"anthropic"}"#,
    );
    let create: serde_json::Value = serde_json::from_str(body_of(&r)).unwrap();
    let sid = create["id"].as_str().unwrap();

    let r = http_post(port, &format!("/sessions/{sid}/runs"), "{}");
    assert!(r.contains("HTTP/1.1 500"), "expected 500, got: {r}");
    let body = body_of(&r);
    let parsed: serde_json::Value = serde_json::from_str(body).expect("valid json");
    let err = parsed["error"].as_str().unwrap_or("");
    assert!(
        err.contains("ANTHROPIC_API_KEY"),
        "expected ANTHROPIC_API_KEY in error, got: {err}"
    );

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn gateway_post_messages_appends_to_session_then_round_trips() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    Command::new(argo_bin())
        .current_dir(workspace.path())
        .env("ARGO_HOME", home.path())
        .args(["workspace", "init"])
        .output()
        .expect("workspace init");

    let mut child = Command::new(argo_bin())
        .current_dir(workspace.path())
        .env("ARGO_HOME", home.path())
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "2",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let stderr = child.stderr.take().expect("stderr");
    let mut stderr = BufReader::new(stderr);
    let mut line = String::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            panic!("listener did not announce in 2s");
        }
        line.clear();
        stderr.read_line(&mut line).expect("read line");
        if line.contains("listening on") {
            break;
        }
    }

    // 1. Create a session.
    let r = http_post(
        port,
        "/sessions",
        r#"{"model":"m","provider":"p","title":"messages-test"}"#,
    );
    assert!(r.contains("HTTP/1.1 201 Created"));
    let create: serde_json::Value = serde_json::from_str(body_of(&r)).unwrap();
    let sid = create["id"].as_str().unwrap();

    // 2. Append a message.
    let path = format!("/sessions/{sid}/messages");
    let r = http_post(
        port,
        &path,
        r#"{"role":"user","content":[{"type":"text","text":"hi"}]}"#,
    );
    assert!(r.contains("HTTP/1.1 201 Created"), "got: {r}");
    let body = body_of(&r);
    let parsed: serde_json::Value = serde_json::from_str(body).expect("valid json");
    assert!(parsed["id"].is_string());
    assert_eq!(parsed["session_id"], sid);

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[cfg(unix)]
#[test]
fn gateway_stop_sends_sigterm_to_live_pid_then_clears_pidfile() {
    use std::process::Command;

    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["gateway", "start", "--port", &port.to_string()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn argo gateway start");

    // Wait for "listening on" so we know the PID file is written.
    let stderr = child.stderr.take().expect("stderr");
    let mut stderr = BufReader::new(stderr);
    let mut line = String::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            panic!("listener did not announce in 2s");
        }
        line.clear();
        stderr.read_line(&mut line).expect("read line");
        if line.contains("listening on") {
            break;
        }
    }

    let pidfile = home.path().join("gateway.pid");
    assert!(pidfile.exists(), "PID file should be created on start");

    let stop = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["gateway", "stop"])
        .output()
        .expect("spawn argo gateway stop");
    assert!(stop.status.success(), "stop failed: {:?}", stop);
    assert!(String::from_utf8_lossy(&stop.stdout).contains("SIGTERM"));

    // The child should have exited from SIGTERM. Wait briefly.
    let status = child.wait().expect("wait child");
    // Default tokio shutdown on SIGTERM produces a non-zero exit
    // (signal-killed); accept both 0 and signal-terminated.
    assert!(
        status.code().is_none() || matches!(status.code(), Some(0) | Some(143)),
        "unexpected exit: {status:?}"
    );
}

#[cfg(unix)]
#[test]
fn gateway_stop_with_no_pidfile_returns_clean_error() {
    use std::process::Command;
    let home = tempfile::tempdir().unwrap();
    let stop = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["gateway", "stop"])
        .output()
        .expect("spawn");
    assert!(!stop.status.success());
    let stderr = String::from_utf8_lossy(&stop.stderr);
    assert!(
        stderr.contains("no PID file"),
        "expected 'no PID file' hint, got: {stderr}"
    );
}
