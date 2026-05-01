//! Shared helpers for gateway integration tests. Cargo's
//! convention: anything under `tests/common/mod.rs` is
//! `mod common;`-imported by each test binary, but doesn't
//! count as a separate test target.

#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{ChildStderr, Command, Stdio};
use std::time::Duration;

pub fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

pub fn pick_free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

pub fn http_get(port: u16, path: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let req = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

pub fn http_post(port: u16, path: &str, body: &str) -> String {
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

pub fn body_of(resp: &str) -> &str {
    resp.split_once("\r\n\r\n").map(|(_h, b)| b).unwrap_or("")
}

pub fn await_listening(stderr: &mut BufReader<impl Read>) {
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
pub fn boot(
    port: u16,
    max_requests: u32,
) -> (std::process::Child, BufReader<ChildStderr>) {
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
