//! Integration smoke for `mission-pm-workflow-edit-api`.
//!
//! Spawns `argo gateway start`, then exercises the seven workflow-
//! definition routes through TCP. The bearer token is intentionally
//! unset so the per-handler `X-Argo-Principal` + `X-Argo-Role`
//! capability gate is the only line of defense — that gate is the
//! contract this brief ships.
//!
//! Covers (per the brief acceptance criteria):
//!
//! * anonymous (no `X-Argo-Principal`) → 401 on GET and POST
//! * authenticated reader (`X-Argo-Role: user`) → 200 on GET,
//!   403 on POST
//! * authenticated editor (`X-Argo-Role: owner`) → 201 on POST,
//!   200 on the subsequent GET
//! * `PUT /head` flips the head pointer; `GET /{name}` reflects it
//! * `DELETE /versions/{id}` on the current head clears the head
//!   pointer (subsequent `GET /{name}` is 404)
mod common;
use common::listener_signal::{await_listener_signal, StderrDrain};
use common::{argo_bin, http_get, pick_free_port};

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};

const VERTICAL: &str = "pr_checker";
const NAME: &str = "smoke-flow";

const ANON_OK: &str = "human:smoke-anon@example.com";
const READER: &str = "human:smoke-reader@example.com";
const EDITOR: &str = "human:smoke-editor@example.com";

/// One workflow TOML the smoke uses for every POST. Parses cleanly
/// through `argo_vertical::workflow::parse_workflow`.
const SAMPLE_TOML: &str = r#"schema_version = "1"
[workflow]
name = "smoke-flow"
description = "smoke"

[[step]]
name = "first"
kind = "prompt"
prompt = "hi"
"#;

/// Local boot helper. The shared `common::boot` does not stamp the
/// `ANTHROPIC_API_KEY` env var on the spawned `argo gateway start`
/// child, which the post-#1252 startup health check requires — see
/// `crates/argo-cli/src/handlers/gateway/startup_check.rs`. Setting
/// it inline (a test-only stub value) keeps the boundary contained
/// to this brief; the shared helper stays untouched.
/// Fixed 32-byte SQLCipher key in base64 — set via `SKYCOT_DB_KEY_BASE64`
/// so the gateway bypasses the macOS keychain entirely. Keychain access
/// from a headless test process can prompt or block, surfacing as
/// `DeadlineExceeded` on `await_listener_signal`. The value is the
/// base64 of 32 fixed bytes (0x42 repeated) and never leaves the test
/// boundary — every test invocation creates a fresh tempdir DB.
const SMOKE_DB_KEY_B64: &str = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";

fn boot_with_api_key(port: u16, max_requests: u32) -> (Child, StderrDrain) {
    let home = tempfile::tempdir().expect("tempdir").keep();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", &home as &Path)
        .env("ANTHROPIC_API_KEY", "sk-ant-workflow-edit-api-smoke")
        .env("SKYCOT_DB_KEY_BASE64", SMOKE_DB_KEY_B64)
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            &(max_requests + 1).to_string(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn argo gateway");
    let stderr = child.stderr.take().expect("stderr piped");
    // Cold-cache compile loads the test machine; the default 10s
    // listener deadline races the startup health check on shared
    // targets, so bump to 60s if the operator hasn't already.
    if std::env::var_os("ARGO_TEST_LISTENER_DEADLINE_MS").is_none() {
        std::env::set_var("ARGO_TEST_LISTENER_DEADLINE_MS", "60000");
    }
    let drain = await_listener_signal(stderr).expect("gateway listener signal");
    let resp = http_get(port, "/sessions");
    assert!(
        resp.contains("HTTP/1.1 200 OK"),
        "gateway store not ready: {resp}"
    );
    (child, drain)
}

fn http(port: u16, line: &str, body: &str, extras: &[(&str, &str)]) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let mut req = format!(
        "{line}\r\nHost: localhost\r\nContent-Type: text/plain\r\n\
         Content-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    for (k, v) in extras {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str("\r\n");
    req.push_str(body);
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

fn body_of(resp: &str) -> &str {
    resp.split_once("\r\n\r\n").map(|(_h, b)| b).unwrap_or("")
}

fn status_line(resp: &str) -> &str {
    resp.lines().next().unwrap_or("")
}

fn principal_and_role<'a>(p: &'a str, role: &'a str) -> [(&'a str, &'a str); 2] {
    [("X-Argo-Principal", p), ("X-Argo-Role", role)]
}

fn extract_version_id(body: &str) -> String {
    body.split("\"version_id\":\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("version_id in body")
        .to_string()
}

#[test]
fn workflow_edit_api_lifecycle() {
    let port = pick_free_port();
    // 13 in-test HTTP requests below; `boot_with_api_key` issues one
    // `GET /sessions` health probe and adds `+1` internally, so the
    // gateway's `--max-requests` lands at exactly 14 and the child
    // exits cleanly after the final assertion.
    let (mut child, _drain) = boot_with_api_key(port, 13);

    // ── (a) anonymous request → 401 on GET and POST ──
    let resp = http(port, &format!("GET /api/v1/verticals/{VERTICAL}/workflows HTTP/1.1"), "", &[]);
    assert!(
        status_line(&resp).contains("401"),
        "anonymous GET should 401; got: {}",
        status_line(&resp)
    );
    let resp = http(
        port,
        &format!("POST /api/v1/verticals/{VERTICAL}/workflows/{NAME} HTTP/1.1"),
        SAMPLE_TOML,
        &[],
    );
    assert!(
        status_line(&resp).contains("401"),
        "anonymous POST should 401; got: {}",
        status_line(&resp)
    );

    // ── (b) reader → 200 on GET (empty list), 403 on POST ──
    let reader = principal_and_role(READER, "user");
    let resp = http(
        port,
        &format!("GET /api/v1/verticals/{VERTICAL}/workflows HTTP/1.1"),
        "",
        &reader,
    );
    assert!(
        status_line(&resp).contains("200"),
        "reader GET list should 200; got: {} body={}",
        status_line(&resp),
        body_of(&resp)
    );
    let body = body_of(&resp);
    assert!(body.contains("\"count\":0"), "list before any create should be empty; body={body}");

    let resp = http(
        port,
        &format!("POST /api/v1/verticals/{VERTICAL}/workflows/{NAME} HTTP/1.1"),
        SAMPLE_TOML,
        &reader,
    );
    assert!(
        status_line(&resp).contains("403"),
        "reader POST should 403; got: {}",
        status_line(&resp)
    );

    // ── (c) editor → 201 on POST, 200 on subsequent GET ──
    let editor = principal_and_role(EDITOR, "owner");
    let resp = http(
        port,
        &format!("POST /api/v1/verticals/{VERTICAL}/workflows/{NAME} HTTP/1.1"),
        SAMPLE_TOML,
        &editor,
    );
    assert!(
        status_line(&resp).contains("201"),
        "editor POST should 201; got: {} body={}",
        status_line(&resp),
        body_of(&resp)
    );
    let v1 = extract_version_id(body_of(&resp));

    // GET head before set_head should 404 — the create lands `draft`,
    // not `active`. The smoke confirms this isolation before flipping.
    let resp = http(
        port,
        &format!("GET /api/v1/verticals/{VERTICAL}/workflows/{NAME} HTTP/1.1"),
        "",
        &editor,
    );
    assert!(
        status_line(&resp).contains("404"),
        "head GET before set_head should 404; got: {} body={}",
        status_line(&resp),
        body_of(&resp)
    );

    // ── (d) PUT /head; subsequent GET head visible ──
    let put_body = format!("{{\"version_id\":\"{v1}\"}}");
    let resp = http(
        port,
        &format!("PUT /api/v1/verticals/{VERTICAL}/workflows/{NAME}/head HTTP/1.1"),
        &put_body,
        &editor,
    );
    assert!(
        status_line(&resp).contains("204"),
        "PUT head should 204; got: {} body={}",
        status_line(&resp),
        body_of(&resp)
    );
    let resp = http(
        port,
        &format!("GET /api/v1/verticals/{VERTICAL}/workflows/{NAME} HTTP/1.1"),
        "",
        &editor,
    );
    assert!(
        status_line(&resp).contains("200"),
        "GET head after PUT should 200; got: {}",
        status_line(&resp)
    );
    let body = body_of(&resp);
    assert!(body.contains(&format!("\"id\":\"{v1}\"")), "head body should carry the v1 id; body={body}");
    assert!(
        body.contains("\"lifecycle_state\":\"active\""),
        "head row should be active after set_head; body={body}"
    );

    // ── (e) archive on the current head clears the head pointer ──
    let resp = http(
        port,
        &format!("DELETE /api/v1/verticals/{VERTICAL}/workflows/{NAME}/versions/{v1} HTTP/1.1"),
        "",
        &editor,
    );
    assert!(
        status_line(&resp).contains("204"),
        "DELETE should 204; got: {} body={}",
        status_line(&resp),
        body_of(&resp)
    );
    let resp = http(
        port,
        &format!("GET /api/v1/verticals/{VERTICAL}/workflows/{NAME} HTTP/1.1"),
        "",
        &editor,
    );
    assert!(
        status_line(&resp).contains("404"),
        "GET head after archive should 404 (head cleared); got: {} body={}",
        status_line(&resp),
        body_of(&resp)
    );

    // Sanity: list-versions still returns the archived row.
    let resp = http(
        port,
        &format!("GET /api/v1/verticals/{VERTICAL}/workflows/{NAME}/versions HTTP/1.1"),
        "",
        &editor,
    );
    assert!(
        status_line(&resp).contains("200"),
        "GET versions after archive should 200; got: {} body={}",
        status_line(&resp),
        body_of(&resp)
    );
    let body = body_of(&resp);
    assert!(
        body.contains("\"lifecycle_state\":\"archived\""),
        "archived version should still surface in /versions; body={body}"
    );

    // Reader can read versions but anon cannot — defense-in-depth on
    // the second GET path.
    let resp = http(
        port,
        &format!("GET /api/v1/verticals/{VERTICAL}/workflows/{NAME}/versions HTTP/1.1"),
        "",
        &principal_and_role(ANON_OK, "user"),
    );
    assert!(
        status_line(&resp).contains("200"),
        "user-role GET versions should 200; got: {}",
        status_line(&resp)
    );
    let resp = http(
        port,
        &format!(
            "GET /api/v1/verticals/{VERTICAL}/workflows/{NAME}/versions HTTP/1.1"
        ),
        "",
        &[],
    );
    assert!(
        status_line(&resp).contains("401"),
        "anonymous GET versions should 401; got: {}",
        status_line(&resp)
    );

    assert!(child.wait().expect("wait").success());
}
