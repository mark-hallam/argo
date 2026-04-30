//! Smoke tests for the per-id show endpoints added across
//! #263 / #264 / #265 — `GET /forge/runs/<id>`,
//! `GET /skills/<name>`, `GET /memory/<id>`. Pulled out of
//! `gateway_smoke_v2.rs` so each test file stays under the
//! 500-LOC ceiling.
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
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let req = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

fn await_listening(stderr: &mut BufReader<impl Read>) {
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
}

fn boot(port: u16, max_requests: u32) -> (std::process::Child, BufReader<std::process::ChildStderr>) {
    let home = tempfile::tempdir().unwrap();
    // Persist the tempdir for the child's lifetime by leaking
    // its path into ARGO_HOME — the test process tears it down
    // when it exits.
    let home_path = home.into_path();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home_path)
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            &max_requests.to_string(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let stderr = BufReader::new(child.stderr.take().unwrap());
    (child, stderr)
}

#[test]
fn forge_run_show_returns_404_on_unknown_id_and_400_on_garbage() {
    let port = pick_free_port();
    let (mut child, mut stderr) = boot(port, 2);
    await_listening(&mut stderr);

    let resp = http_get(port, "/forge/runs/00000000-0000-0000-0000-000000000000");
    assert!(resp.contains("HTTP/1.1 404 Not Found"), "resp={resp}");

    let resp = http_get(port, "/forge/runs/not-a-uuid");
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    assert!(child.wait().expect("wait").success());
}

#[test]
fn skill_show_returns_404_on_unknown_name() {
    let port = pick_free_port();
    let (mut child, mut stderr) = boot(port, 1);
    await_listening(&mut stderr);

    let resp = http_get(port, "/skills/nonexistent-skill");
    assert!(resp.contains("HTTP/1.1 404 Not Found"), "resp={resp}");

    assert!(child.wait().expect("wait").success());
}

#[test]
fn memory_show_returns_404_on_unknown_id_and_400_on_garbage() {
    let port = pick_free_port();
    let (mut child, mut stderr) = boot(port, 2);
    await_listening(&mut stderr);

    let resp = http_get(port, "/memory/00000000-0000-0000-0000-000000000000");
    assert!(resp.contains("HTTP/1.1 404 Not Found"), "resp={resp}");

    let resp = http_get(port, "/memory/not-a-uuid");
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    assert!(child.wait().expect("wait").success());
}
