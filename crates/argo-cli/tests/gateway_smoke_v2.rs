//! Smoke tests for the gateway endpoints added in PRs #170 / #175 /
//! #177 — `GET /audit/sessions/<id>`, `GET /costs/by-model`,
//! `GET /costs/by-session`, `POST /sessions/<id>/steer`. Each test
//! spawns `argo gateway start --max-requests N`, hits the
//! endpoint, and asserts a smoke-level shape.
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
fn costs_by_model_and_by_session_return_empty_arrays_on_clean_db() {
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

    let m = http_get(port, "/costs/by-model");
    assert!(m.contains("HTTP/1.1 200 OK"), "m={m}");
    let mb = body_of(&m);
    assert!(mb.contains("\"by_model\""), "body={mb}");

    let s = http_get(port, "/costs/by-session");
    assert!(s.contains("HTTP/1.1 200 OK"), "s={s}");
    let sb = body_of(&s);
    assert!(sb.contains("\"by_session\""), "body={sb}");

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn audit_session_returns_empty_array_for_unknown_session() {
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

    // A random valid UUID with no audit rows yet — endpoint should
    // 200 with `[]`, not 404 (the chain just has nothing for it).
    let r = http_get(port, "/audit/sessions/00000000-0000-0000-0000-000000000000");
    assert!(r.contains("HTTP/1.1 200 OK"), "r={r}");
    assert_eq!(body_of(&r), "[]");

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn steer_endpoint_appends_system_message_and_404s_unknown_session() {
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
            "3",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);

    // Create a session first.
    let create = http_post(
        port,
        "/sessions",
        r#"{"model":"m","provider":"p","title":"t"}"#,
    );
    assert!(create.contains("HTTP/1.1 201 Created"), "create={create}");
    let id = body_of(&create)
        .split("\"id\":\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("id in body")
        .to_string();

    let s = http_post(port, &format!("/sessions/{id}/steer"), r#"{"text":"hi"}"#);
    assert!(s.contains("HTTP/1.1 201 Created"), "s={s}");

    let unknown = http_post(
        port,
        "/sessions/00000000-0000-0000-0000-000000000000/steer",
        r#"{"text":"x"}"#,
    );
    assert!(unknown.contains("HTTP/1.1 404"), "unknown={unknown}");

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn replay_stream_returns_chunked_ndjson_for_unknown_session() {
    // The replay-stream endpoint returns 200 + chunked NDJSON;
    // an unknown session id surfaces as an error event chunk
    // rather than an HTTP error, mirroring the runs-stream
    // contract.
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
        "/sessions/00000000-0000-0000-0000-000000000000/replay/stream",
        r#"{}"#,
    );
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    assert!(
        resp.contains("Transfer-Encoding: chunked"),
        "no chunked header: {resp}"
    );
    let body = body_of(&resp);
    assert!(body.contains("\"event\":\"error\""), "body={body}");
    assert!(body.ends_with("0\r\n\r\n"), "no chunk terminator: {body}");

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn forge_runs_returns_empty_array_on_clean_db() {
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

    let resp = http_get(port, "/forge/runs");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.starts_with('['), "body={body}");

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn routes_endpoint_lists_known_paths() {
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

    let resp = http_get(port, "/routes");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.starts_with('['), "body={body}");
    // Spot-check a handful of expected paths.
    for path in ["/health", "/sessions", "/tools", "/routes"] {
        assert!(body.contains(path), "body missing {path}: {body}");
    }

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn workspace_artifacts_returns_empty_array_on_clean_db() {
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

    let resp = http_get(port, "/workspace/artifacts");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.starts_with('['), "body={body}");

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn search_endpoint_returns_400_on_missing_q_and_array_on_clean_db() {
    // /search returns 400 when `q` is absent (the smoke test
    // hits this path because there's no message in the DB).
    // With a query, the response is a JSON array (empty on a
    // clean store).
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

    let missing_q = http_get(port, "/search");
    assert!(
        missing_q.contains("HTTP/1.1 400 Bad Request"),
        "resp={missing_q}"
    );

    let with_q = http_get(port, "/search?q=hello");
    assert!(with_q.contains("HTTP/1.1 200 OK"), "resp={with_q}");
    let body = body_of(&with_q);
    assert!(body.starts_with('['), "body={body}");

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn providers_and_tools_return_arrays() {
    // /providers reads `<cwd>/manifests/providers/*.toml` (or
    // returns `[]` on a clean workspace). /tools is built from
    // `argo_tool::builtin_tools(cwd)` and always has the eight
    // in-core tools.
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

    let providers = http_get(port, "/providers");
    assert!(providers.contains("HTTP/1.1 200 OK"), "resp={providers}");
    let body = body_of(&providers);
    assert!(body.starts_with('['), "body={body}");

    let tools = http_get(port, "/tools");
    assert!(tools.contains("HTTP/1.1 200 OK"), "resp={tools}");
    let body = body_of(&tools);
    assert!(body.starts_with('['), "body={body}");
    assert!(body.contains("\"file_read\""), "body={body}");
    assert!(body.contains("\"file_write\""), "body={body}");

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn runs_stream_emits_chunked_ndjson_with_terminator() {
    // The gateway's streaming endpoint always returns 200 +
    // chunked-encoded NDJSON. Run against an unknown session so
    // run_turn fails fast and we get a single error event + the
    // chunk terminator without needing a live provider.
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
        "/sessions/00000000-0000-0000-0000-000000000000/runs/stream",
        r#"{"user":"hi"}"#,
    );
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    assert!(
        resp.contains("Transfer-Encoding: chunked"),
        "no chunked header: {resp}"
    );
    let body = body_of(&resp);
    // RFC 7230 chunked terminator + at least one error event chunk.
    assert!(body.contains("\"event\":\"error\""), "body={body}");
    assert!(body.ends_with("0\r\n\r\n"), "no chunk terminator: {body}");

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
    // Build info constant + several store-derived counters land
    // in the body.
    assert!(body.contains("# TYPE argo_build_info gauge"), "body={body}");
    assert!(
        body.contains("argo_build_info{version="),
        "body={body}"
    );
    assert!(body.contains("argo_sessions_total 0"), "body={body}");
    assert!(body.contains("argo_skills_active_total 0"), "body={body}");
    assert!(body.contains("argo_audit_chain_ok"), "body={body}");

    let status = child.wait().expect("wait");
    assert!(status.success());
}
