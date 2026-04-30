//! Smoke tests for the write-side gateway endpoints —
//! `POST /skills`, `POST /memory`. Pulled out of
//! `gateway_smoke_v2.rs` so each integration-test binary stays
//! under the 500-LOC ceiling.
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

/// Start the gateway and return both the child handle + the
/// stderr BufReader. The reader must stay alive for the whole
/// test — dropping it before the child exits closes the pipe
/// and breaks the gateway's stderr writes (manifest as
/// `ConnectionReset` on the next HTTP request).
fn boot(
    port: u16,
    max_requests: u32,
) -> (
    std::process::Child,
    BufReader<std::process::ChildStderr>,
) {
    let home = tempfile::tempdir().unwrap().keep();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home)
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
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);
    (child, stderr)
}

#[test]
fn skill_install_returns_400_on_missing_raw_and_201_on_valid_doc() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 2);

    let resp = http_post(port, "/skills", "{}");
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    let raw = "---\\nname: zeta\\nversion: \\\"0.1\\\"\\ndescription: D\\nlock: none\\n\
               provenance:\\n  source: smoke\\n  hash: sha256-x\\n---\\nbody-of-zeta";
    let body = format!("{{\"raw\": \"{raw}\"}}");
    let resp = http_post(port, "/skills", &body);
    assert!(resp.contains("HTTP/1.1 201 Created"), "resp={resp}");

    assert!(child.wait().expect("wait").success());
}

#[test]
fn memory_append_returns_400_on_unknown_file_and_when_no_workspace() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 2);

    let resp = http_post(port, "/memory", r#"{"file":"bogus","body":"x"}"#);
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    let resp = http_post(port, "/memory", r#"{"file":"memory","body":"hello"}"#);
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.contains("workspace"), "body={body}");

    assert!(child.wait().expect("wait").success());
}
