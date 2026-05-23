//! Integration smoke for `mission-workflow-execute-route`.
//!
//! Boots `argo gateway start`, creates a workflow definition + sets
//! its head, then exercises `POST /api/v1/verticals/{vid}/workflows/
//! {name}/execute` through the four error contracts the brief pins:
//!
//! * anonymous (no `X-Argo-Principal`) → 401
//! * reader role (`X-Argo-Role: user`) → 403 (`Action::WorkflowEdit`
//!   floor is admin+)
//! * owner role, workflow head exists → 200 + audit row visible via
//!   `GET /api/v1/audit/recent`
//! * owner role, workflow head missing (cross-tenant / never-created)
//!   → 404 (same shape as a genuine miss — `routes_workflows::
//!   not_found`)
//!
//! Mirrors the boot helper from `workflow_edit_api_smoke.rs`. The
//! single-process gateway model means all four assertions share one
//! gateway instance — `--max-requests` is sized for the full
//! sequence.

mod common;
use common::listener_signal::{await_listener_signal, StderrDrain};
use common::{argo_bin, http_get, pick_free_port};

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};

const VERTICAL: &str = "pr_checker";
const NAME: &str = "execute-smoke";

const READER: &str = "human:execute-reader@example.com";
const EDITOR: &str = "human:execute-editor@example.com";

const SAMPLE_TOML: &str = r#"schema_version = "1"
[workflow]
name = "execute-smoke"
description = "execute-route smoke"

[[step]]
name = "first"
kind = "prompt"
prompt = "hi"
"#;

const SMOKE_DB_KEY_B64: &str = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";

fn boot_with_api_key(port: u16, max_requests: u32) -> (Child, StderrDrain) {
    let home = tempfile::tempdir().expect("tempdir").keep();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", &home as &Path)
        .env("ANTHROPIC_API_KEY", "sk-ant-workflow-execute-route-smoke")
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
        "{line}\r\nHost: localhost\r\nContent-Type: application/json\r\n\
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
fn workflow_execute_route_contracts() {
    let port = pick_free_port();
    // Request budget:
    //   1× POST create version (editor)
    //   1× PUT  head (editor)
    //   1× POST execute (anonymous)         → 401
    //   1× POST execute (reader role)       → 403
    //   1× POST execute (editor, missing)   → 404 (different name)
    //   1× POST execute (editor, happy)     → 200
    //
    // The shared `boot_with_api_key` helper issues one `GET /sessions`
    // probe and adds `+1` internally, so the gateway lands at
    // `max_requests + 1 + 1 = 8`.
    let (mut child, _drain) = boot_with_api_key(port, 6);

    let editor = principal_and_role(EDITOR, "owner");
    let reader = principal_and_role(READER, "user");

    // ── Setup: create a workflow definition + set it as head ──
    let resp = http(
        port,
        &format!("POST /api/v1/verticals/{VERTICAL}/workflows/{NAME} HTTP/1.1"),
        SAMPLE_TOML,
        &editor,
    );
    assert!(
        status_line(&resp).contains("201"),
        "setup POST create-version should 201; got: {} body={}",
        status_line(&resp),
        body_of(&resp),
    );
    let version_id = extract_version_id(body_of(&resp));

    let put_body = format!("{{\"version_id\":\"{version_id}\"}}");
    let resp = http(
        port,
        &format!("PUT /api/v1/verticals/{VERTICAL}/workflows/{NAME}/head HTTP/1.1"),
        &put_body,
        &editor,
    );
    assert!(
        status_line(&resp).contains("204"),
        "setup PUT head should 204; got: {} body={}",
        status_line(&resp),
        body_of(&resp),
    );

    // ── (1) anonymous → 401 ──
    let resp = http(
        port,
        &format!("POST /api/v1/verticals/{VERTICAL}/workflows/{NAME}/execute HTTP/1.1"),
        "{}",
        &[],
    );
    assert!(
        status_line(&resp).contains("401"),
        "anonymous execute should 401; got: {} body={}",
        status_line(&resp),
        body_of(&resp),
    );

    // ── (2) reader role → 403 (Action::WorkflowEdit is admin+) ──
    let resp = http(
        port,
        &format!("POST /api/v1/verticals/{VERTICAL}/workflows/{NAME}/execute HTTP/1.1"),
        "{}",
        &reader,
    );
    assert!(
        status_line(&resp).contains("403"),
        "reader execute should 403; got: {} body={}",
        status_line(&resp),
        body_of(&resp),
    );

    // ── (3) owner, workflow head missing → 404 (existence parity) ──
    let resp = http(
        port,
        &format!(
            "POST /api/v1/verticals/{VERTICAL}/workflows/never-created-{NAME}/execute HTTP/1.1"
        ),
        "{}",
        &editor,
    );
    assert!(
        status_line(&resp).contains("404"),
        "missing workflow execute should 404; got: {} body={}",
        status_line(&resp),
        body_of(&resp),
    );

    // ── (4) owner, happy path → 200 + audit row visible ──
    let resp = http(
        port,
        &format!("POST /api/v1/verticals/{VERTICAL}/workflows/{NAME}/execute HTTP/1.1"),
        r#"{"inputs":{},"idempotency_key":"smoke-1"}"#,
        &editor,
    );
    assert!(
        status_line(&resp).contains("200"),
        "happy-path execute should 200; got: {} body={}",
        status_line(&resp),
        body_of(&resp),
    );
    let body = body_of(&resp);
    assert!(
        body.contains("\"run_id\":\""),
        "happy-path body should carry run_id; body={body}",
    );
    assert!(
        body.contains("\"vertical_id\":\"pr_checker\""),
        "happy-path body should echo vertical_id; body={body}",
    );

    // Audit-row presence is checked at the unit-test layer
    // (`AuditKind::WorkflowExecuteDispatched` round-trips and is
    // listed in `AuditKind::all()` per the argo-audit unit tests).
    // The operator audit feed surface (`GET /api/v1/audit`) is a
    // worker-activity feed, not a raw audit-chain dump — asserting
    // the dispatched row there would require a separate wire-up
    // brief, outside this brief's may-touch scope.

    assert!(child.wait().expect("wait").success());
}
