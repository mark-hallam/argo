use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::Duration;

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn run(workspace: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .current_dir(workspace)
        .env("ARGO_HOME", home)
        .env_remove("ARGO_DEFAULT_TENANT")
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

fn pick_free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

fn http_get(port: u16, path: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let req = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

fn http_post(port: u16, path: &str, body: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

fn body_of(resp: &str) -> &str {
    resp.split_once("\r\n\r\n").map(|(_h, b)| b).unwrap_or("")
}

fn await_listening(stderr: &mut BufReader<impl Read>) {
    let mut line = String::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        if std::time::Instant::now() > deadline {
            panic!("listener did not announce in 10s");
        }
        line.clear();
        stderr.read_line(&mut line).expect("read line");
        if line.contains("listening on") {
            break;
        }
    }
}

#[cfg(unix)]
#[test]
fn session_start_worker_flag_persists_worker_id_for_show() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let tenant = uuid::Uuid::now_v7().to_string();
    let worker = uuid::Uuid::now_v7().to_string();
    let out = run(
        workspace.path(),
        home.path(),
        &[
            "session", "start", "--tenant", &tenant, "--title", "worker-smoke",
            "--worker", &worker, "--json",
        ],
    );
    assert!(out.status.success(), "stderr={}", String::from_utf8_lossy(&out.stderr));
    let created: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let session_id = created["id"].as_str().unwrap();
    assert_eq!(created["participants"][0].as_str(), Some(worker.as_str()));

    let shown = run(workspace.path(), home.path(), &["session", "show", session_id, "--json"]);
    assert!(shown.status.success(), "stderr={}", String::from_utf8_lossy(&shown.stderr));
    let shown: serde_json::Value = serde_json::from_slice(&shown.stdout).unwrap();
    assert_eq!(shown["worker_id"].as_str(), Some(worker.as_str()));

    let no_worker = run(
        workspace.path(),
        home.path(),
        &["session", "start", "--tenant", &tenant, "--title", "no-worker", "--json"],
    );
    assert!(no_worker.status.success(), "stderr={}", String::from_utf8_lossy(&no_worker.stderr));
    let no_worker: serde_json::Value = serde_json::from_slice(&no_worker.stdout).unwrap();
    let shown = run(
        workspace.path(),
        home.path(),
        &["session", "show", no_worker["id"].as_str().unwrap(), "--json"],
    );
    assert!(shown.status.success(), "stderr={}", String::from_utf8_lossy(&shown.stderr));
    let shown: serde_json::Value = serde_json::from_slice(&shown.stdout).unwrap();
    assert!(shown["worker_id"].is_null());
}

#[cfg(unix)]
#[test]
fn gateway_post_and_get_return_worker_id() {
    let port = pick_free_port();
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let worker = uuid::Uuid::now_v7().to_string();
    let mut child = Command::new(argo_bin())
        .current_dir(workspace.path())
        .env("ARGO_HOME", home.path())
        .args(["gateway", "start", "--port", &port.to_string(), "--max-requests", "2"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);

    let create_body = serde_json::json!({
        "model": "m",
        "provider": "p",
        "title": "worker-gateway",
        "worker_id": worker,
    })
    .to_string();
    let created = http_post(port, "/sessions", &create_body);
    assert!(created.contains("HTTP/1.1 201 Created"), "created={created}");
    let created: serde_json::Value = serde_json::from_str(body_of(&created)).unwrap();
    let session_id = created["id"].as_str().unwrap();
    assert_eq!(created["worker_id"].as_str(), Some(worker.as_str()));

    let shown = http_get(port, &format!("/sessions/{session_id}"));
    assert!(shown.contains("HTTP/1.1 200 OK"), "shown={shown}");
    let shown: serde_json::Value = serde_json::from_str(body_of(&shown)).unwrap();
    assert_eq!(shown["worker_id"].as_str(), Some(worker.as_str()));
    assert!(child.wait().expect("wait").success());
}
