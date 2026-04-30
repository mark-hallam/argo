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
