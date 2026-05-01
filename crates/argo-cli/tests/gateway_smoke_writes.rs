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

#[test]
fn forge_optimize_post_validates_body_and_returns_202_on_kick_off() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 3);

    // Missing skill → 400
    let resp = http_post(port, "/forge/optimize", r#"{"examples":"foo.jsonl"}"#);
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    // Missing examples → 400
    let resp = http_post(port, "/forge/optimize", r#"{"skill":"x"}"#);
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    // Valid body → 202 Accepted (background task may fail later
    // because the skill / file don't exist, but the kick-off is
    // synchronous and replies immediately).
    let resp = http_post(
        port,
        "/forge/optimize",
        r#"{"skill":"missing-skill","examples":"missing.jsonl","iterations":1,"budget_usd":0.5}"#,
    );
    assert!(resp.contains("HTTP/1.1 202 Accepted"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.contains(r#""started":true"#), "body={body}");
    assert!(body.contains(r#""kind":"optimize""#), "body={body}");

    assert!(child.wait().expect("wait").success());
}

#[test]
fn forge_bench_post_returns_202_with_provider_defaults() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 2);

    let resp = http_post(port, "/forge/bench", r#"{}"#);
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    let resp = http_post(
        port,
        "/forge/bench",
        r#"{"skill":"missing-skill","examples":"missing.jsonl"}"#,
    );
    assert!(resp.contains("HTTP/1.1 202 Accepted"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.contains(r#""kind":"bench""#), "body={body}");
    assert!(body.contains(r#""provider":"anthropic""#), "body={body}");

    assert!(child.wait().expect("wait").success());
}

#[test]
fn forge_abort_returns_400_on_short_id_and_404_on_missing_uuid() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 2);

    // Short id (8 hex) — rejected at parse step.
    let resp = http_post(port, "/forge/runs/abcdef12/abort", "");
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    // Valid uuid that isn't in the DB — 404.
    let resp = http_post(
        port,
        "/forge/runs/00000000-0000-0000-0000-000000000000/abort",
        "",
    );
    assert!(resp.contains("HTTP/1.1 404 Not Found"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.contains("no forge run"), "body={body}");

    assert!(child.wait().expect("wait").success());
}

#[test]
fn skill_promote_returns_400_on_missing_version_and_unknown_skill() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 2);

    // Missing version → 400.
    let resp = http_post(port, "/skills/anything/promote", r#"{}"#);
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.contains("missing or empty `version`"), "body={body}");

    // Unknown skill → 400 from SkillStore::promote.
    let resp = http_post(
        port,
        "/skills/never-installed/promote",
        r#"{"version":"0.1"}"#,
    );
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.contains("promote"), "body={body}");

    assert!(child.wait().expect("wait").success());
}
