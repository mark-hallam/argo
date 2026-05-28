//! CSRF double-submit pin for `POST /api/v1/verticals/{vid}/workflows/
//! {name}/execute` — the "Run now" surface narrated in Scene 5 of the
//! v1.0 demo MP4 runbook.
//!
//! Brief: `fix-workflow-execute-csrf-token-wiring-verify-before-demo`.
//! UX-AUDIT-T0519 F9 [P1]: a direct preview-eval curl to this route
//! returned `403 csrf_missing`, but the auditor could not confirm the
//! in-browser path (Preview MCP wedged). This test pins the gateway
//! side: cookie-authenticated callers MUST present a matching
//! `X-CSRF-Token` header, and the workflow-execute route MUST honor
//! the gate. The SPA-side counterpart pin lives at
//! `web/lib/api/client.test.ts` (vitest).
//!
//! The CSRF middleware itself is exercised in `middleware_csrf::tests`
//! (path-agnostic unit tests). What this file adds is the route-level
//! integration: that `POST .../execute` actually flows through the
//! middleware on the live gateway boot.
//!
//! Test strategy:
//!   1. Boot gateway with `SKYCOT_AUTH_JWT_SECRET` set to a known
//!      string so the test can mint cookie JWTs the verify gate
//!      accepts.
//!   2. Set up a workflow head via bearer-auth (no cookies — CSRF
//!      skips when no `__Host-session` is present), so the execute
//!      request has a real target to dispatch.
//!   3. POST execute with `__Host-session` + `__Host-csrf` cookies
//!      but no `X-CSRF-Token` header → expect `403 csrf_missing`.
//!   4. POST execute with cookies + matching `X-CSRF-Token` header
//!      → expect `200` with a `run_id` in the body. The audit-row
//!      assertion is the same one the sibling
//!      `routes_workflow_execute_test::workflow_execute_route_contracts`
//!      test already pins at the unit-test layer (see comment block in
//!      that file); this test asserts the dispatch path is reachable
//!      through the CSRF gate, not the audit-row shape.

mod common;
use common::listener_signal::{await_listener_signal, StderrDrain};
use common::{argo_bin, http_get, pick_free_port};

use base64::Engine;
use jsonwebtoken::{EncodingKey, Header};
use serde::Serialize;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};

const VERTICAL: &str = "pr_checker";
const NAME: &str = "csrf-pin";
const EDITOR: &str = "human:csrf-pin@example.com";
const JWT_SECRET: &str = "csrf-pin-secret-do-not-use-in-prod";
const SMOKE_DB_KEY_B64: &str = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";

const SAMPLE_TOML: &str = r#"schema_version = "1"
[workflow]
name = "csrf-pin"
description = "csrf-route smoke"

[[step]]
name = "first"
kind = "prompt"
prompt = "hi"
"#;

#[derive(Serialize)]
struct Claims<'a> {
    sub: &'a str,
    email: &'a str,
    tnt: Option<&'a str>,
    role: Option<&'a str>,
    jti: &'a str,
    iat: i64,
    exp: i64,
}

fn mint_jwt(role: &str) -> (String, String) {
    // Random-ish jti so the two test passes don't share a token value
    // (defensive — the CSRF gate compares header to cookie verbatim,
    // not against any registry, but ensures we're not relying on a
    // coincidence of identical strings elsewhere).
    let jti = uuid::Uuid::now_v7().to_string();
    let now = chrono::Utc::now().timestamp();
    let key = EncodingKey::from_secret(JWT_SECRET.as_bytes());
    let token = jsonwebtoken::encode(
        &Header::new(jsonwebtoken::Algorithm::HS256),
        &Claims {
            sub: "csrf-pin-user",
            email: "csrf-pin@example.com",
            tnt: None,
            role: Some(role),
            jti: &jti,
            iat: now,
            exp: now + 600,
        },
        &key,
    )
    .expect("mint jwt");
    (token, jti)
}

fn boot_with_jwt_secret(port: u16, max_requests: u32) -> (Child, StderrDrain) {
    let home = tempfile::tempdir().expect("tempdir").keep();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", &home as &Path)
        .env("ANTHROPIC_API_KEY", "sk-ant-csrf-pin")
        .env("SKYCOT_DB_KEY_BASE64", SMOKE_DB_KEY_B64)
        .env("SKYCOT_AUTH_JWT_SECRET", JWT_SECRET)
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

fn extract_version_id(body: &str) -> String {
    body.split("\"version_id\":\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("version_id in body")
        .to_string()
}

#[test]
fn workflow_execute_csrf_gate() {
    // Compile-time sanity-check that the b64 helper matches the encoded
    // length we expect — guards against transcription regressions in
    // the smoke DB key constant.
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(SMOKE_DB_KEY_B64)
            .expect("decode smoke db key")
            .len(),
        32,
    );

    let port = pick_free_port();
    // Request budget:
    //   1× POST create version (bearer-auth, no cookies)
    //   1× PUT  head (bearer-auth, no cookies)
    //   1× POST execute (cookies, no CSRF header)  → 403 csrf_missing
    //   1× POST execute (cookies + matching CSRF header) → 200
    //
    // `boot_with_jwt_secret` issues one `GET /sessions` probe and adds
    // `+1` internally, so the gateway lands at `max_requests+1+1 = 6`.
    let (mut child, _drain) = boot_with_jwt_secret(port, 4);

    // ── Setup: create a workflow definition + set head via bearer ──
    let bearer = [
        ("X-Argo-Principal", EDITOR),
        ("X-Argo-Role", "owner"),
    ];
    let resp = http(
        port,
        &format!("POST /api/v1/verticals/{VERTICAL}/workflows/{NAME} HTTP/1.1"),
        SAMPLE_TOML,
        &bearer,
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
        &bearer,
    );
    assert!(
        status_line(&resp).contains("204"),
        "setup PUT head should 204; got: {} body={}",
        status_line(&resp),
        body_of(&resp),
    );

    // Mint a session JWT once and reuse — the same `__Host-csrf` value
    // (= the JWT's `jti`) is what the SPA echoes back as the header.
    let (jwt, jti) = mint_jwt("owner");
    let cookie_pair = format!("__Host-session={jwt}; __Host-csrf={jti}");

    // ── (1) cookies present, NO `X-CSRF-Token` header → 403 csrf_missing
    let resp = http(
        port,
        &format!("POST /api/v1/verticals/{VERTICAL}/workflows/{NAME}/execute HTTP/1.1"),
        "{}",
        &[
            ("Cookie", cookie_pair.as_str()),
            ("X-Argo-Principal", EDITOR),
            ("X-Argo-Role", "owner"),
        ],
    );
    assert!(
        status_line(&resp).contains("403"),
        "execute without CSRF header should 403; got: {} body={}",
        status_line(&resp),
        body_of(&resp),
    );
    assert!(
        body_of(&resp).contains("csrf_missing"),
        "execute without CSRF header should carry csrf_missing body; got body={}",
        body_of(&resp),
    );

    // ── (2) cookies + matching `X-CSRF-Token` → 200 + run_id
    let resp = http(
        port,
        &format!("POST /api/v1/verticals/{VERTICAL}/workflows/{NAME}/execute HTTP/1.1"),
        r#"{"inputs":{},"idempotency_key":"csrf-pin-1"}"#,
        &[
            ("Cookie", cookie_pair.as_str()),
            ("X-CSRF-Token", jti.as_str()),
            ("X-Argo-Principal", EDITOR),
            ("X-Argo-Role", "owner"),
        ],
    );
    assert!(
        status_line(&resp).contains("200"),
        "execute with matching CSRF header should 200; got: {} body={}",
        status_line(&resp),
        body_of(&resp),
    );
    let body = body_of(&resp);
    assert!(
        !body.contains("csrf_missing") && !body.contains("csrf_mismatch"),
        "happy-path body must not carry a CSRF rejection; body={body}",
    );
    assert!(
        body.contains("\"run_id\":\""),
        "happy-path body should carry run_id; body={body}",
    );

    assert!(child.wait().expect("wait").success());
}
