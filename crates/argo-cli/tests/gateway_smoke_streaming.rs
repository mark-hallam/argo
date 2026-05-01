//! Smoke tests for the streaming + observability surface —
//! `POST /forge/optimize/stream`, `GET /metrics`, and
//! `GET /audit/sessions/<id>/verify`. Pulled out of
//! `gateway_smoke_v2.rs` so each integration-test binary
//! stays under the 500-LOC ceiling.
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

#[test]
fn forge_optimize_stream_returns_chunked_ndjson_with_error_chunk() {
    // The skill doesn't exist, so the spawned task errors out
    // — but we still get a 200 + chunked transfer header + a
    // chunk-terminator. That's enough to prove the routing +
    // chunking framework is wired correctly.
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
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);

    let resp = http_post(
        port,
        "/forge/optimize/stream",
        r#"{"skill":"missing-skill","examples":"missing.jsonl","iterations":1,"budget_usd":0.01}"#,
    );
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    assert!(
        resp.contains("Transfer-Encoding: chunked"),
        "missing chunked header: {resp}"
    );
    let body = body_of(&resp);
    assert!(body.contains(r#""event":"error""#), "body={body}");
    assert!(body.ends_with("0\r\n\r\n"), "no chunk terminator: {body}");

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn audit_session_verify_returns_ok_true_for_empty_session() {
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
            "2",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);

    // Bad uuid → 400.
    let resp = http_get(port, "/audit/sessions/not-a-uuid/verify");
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    // Valid uuid with no rows → ok=true, rows=0.
    let resp = http_get(
        port,
        "/audit/sessions/00000000-0000-0000-0000-000000000000/verify",
    );
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.contains(r#""ok":true"#), "body={body}");
    assert!(body.contains(r#""rows":0"#), "body={body}");

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn session_judge_returns_empty_array_for_clean_session() {
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
            "2",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);

    // Bad uuid → 400.
    let resp = http_get(port, "/sessions/not-a-uuid/judge");
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    // Valid uuid with no rows → empty array.
    let resp = http_get(
        port,
        "/sessions/00000000-0000-0000-0000-000000000000/judge",
    );
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    assert_eq!(body_of(&resp), "[]");

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn db_stats_returns_zero_counts_for_clean_workspace() {
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
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);

    let resp = http_get(port, "/db/stats");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let body = body_of(&resp);
    let parsed: serde_json::Value = serde_json::from_str(body).expect("valid json");
    assert_eq!(parsed["sessions"], 0);
    assert_eq!(parsed["messages"], 0);
    assert_eq!(parsed["audit_rows"], 0);
    assert_eq!(parsed["forge_runs"], 0);

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn metrics_endpoint_serves_text_plain_with_prometheus_lines() {
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
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);

    let resp = http_get(port, "/metrics");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    assert!(
        resp.contains("Content-Type: text/plain; version=0.0.4"),
        "missing prometheus content-type: {resp}"
    );
    let body = body_of(&resp);
    assert!(body.contains("# TYPE argo_build_info gauge"), "body={body}");
    assert!(body.contains("argo_build_info{version="), "body={body}");
    assert!(body.contains("argo_sessions_total 0"), "body={body}");
    assert!(body.contains("argo_skills_active_total 0"), "body={body}");
    assert!(body.contains("argo_audit_chain_ok"), "body={body}");

    let status = child.wait().expect("wait");
    assert!(status.success());
}
