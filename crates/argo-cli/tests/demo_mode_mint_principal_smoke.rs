//! Integration smoke for the env-var-gated demo-mode magic-link
//! bypass (`demo-ops-add-magic-link-bypass-for-recording`).
//!
//! Three scenarios:
//!
//! 1. **Off.** Without `ARGO_DEMO_MODE=1`, `POST /api/v1/demo/mint-principal`
//!    falls through to the default 404 — the dispatcher never mounts
//!    the demo route.
//! 2. **On + bypass.** With `ARGO_DEMO_MODE=1` + `ARGO_GATEWAY_AUTH_BYPASS=1`,
//!    the route mints a JSON envelope including
//!    `principal: "human:<email>"` and `session_token` (a real JWT),
//!    and the gateway writes an `auth.succeeded` audit row whose
//!    payload contains `demo_mode_bypass: true`.
//! 3. **On without bypass.** With `ARGO_DEMO_MODE=1` but no
//!    `ARGO_GATEWAY_AUTH_BYPASS`, the startup-check refuses to bind
//!    and exits with the documented message.
mod common;

use common::{listener_signal::await_listener_signal, pick_free_port};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
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

fn status_line(resp: &str) -> &str {
    resp.lines().next().unwrap_or("")
}

fn body_of(resp: &str) -> &str {
    resp.split_once("\r\n\r\n").map(|(_h, b)| b).unwrap_or("")
}

fn spawn_gateway(home: &Path, port: u16, demo_mode: bool) -> Child {
    let mut cmd = Command::new(argo_bin());
    cmd.env("ARGO_HOME", home)
        .env("OPENAI_API_KEY", "test-key")
        .env("ARGO_GATEWAY_AUTH_BYPASS", "1")
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if demo_mode {
        cmd.env("ARGO_DEMO_MODE", "1");
    } else {
        cmd.env_remove("ARGO_DEMO_MODE");
    }
    cmd.spawn().expect("spawn")
}

async fn read_demo_audit_payloads(home: &Path) -> Vec<String> {
    let store = argo_store::SessionStore::open(home.join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let rows = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT kind, payload_json FROM audit_record WHERE kind = 'auth.succeeded' ORDER BY seq ASC",
    )
    .fetch_all(store.pool())
    .await
    .unwrap();
    rows.into_iter().filter_map(|(_, p)| p).collect()
}

#[test]
fn demo_route_returns_404_when_flag_unset() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let mut child = spawn_gateway(home.path(), port, false);
    let stderr = child.stderr.take().expect("stderr piped");
    let _drain = await_listener_signal(stderr).expect("gateway listener signal");

    let resp = http_post(
        port,
        "/api/v1/demo/mint-principal",
        r#"{"email":"demo@throwaway.cc"}"#,
    );
    assert!(
        resp.contains("HTTP/1.1 404 Not Found"),
        "expected 404 with demo mode off, got: {}",
        status_line(&resp),
    );

    let _ = child.wait();
}

#[test]
fn demo_route_mints_principal_and_writes_audit_when_flag_set() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let mut child = spawn_gateway(home.path(), port, true);
    let stderr = child.stderr.take().expect("stderr piped");
    let _drain = await_listener_signal(stderr).expect("gateway listener signal");

    let resp = http_post(
        port,
        "/api/v1/demo/mint-principal",
        r#"{"email":"demo@throwaway.cc"}"#,
    );
    assert!(
        resp.contains("HTTP/1.1 200 OK"),
        "mint failed: {}",
        status_line(&resp),
    );
    let body = body_of(&resp);
    assert!(
        body.contains(r#""ok":true"#),
        "mint body missing ok=true: {body}",
    );
    assert!(
        body.contains(r#""principal":"human:demo@throwaway.cc""#),
        "mint body missing principal: {body}",
    );
    assert!(
        body.contains(r#""session_token":""#),
        "mint body missing session_token: {body}",
    );

    let _ = child.wait();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let payloads = rt.block_on(read_demo_audit_payloads(home.path()));
    assert!(
        payloads
            .iter()
            .any(|p| p.contains("demo_mode_bypass") && p.contains("true")),
        "no auth.succeeded row with demo_mode_bypass=true: {payloads:?}",
    );
    assert!(
        payloads.iter().any(|p| p.contains("demo@throwaway.cc")),
        "no auth.succeeded row stamped with email: {payloads:?}",
    );
}

#[test]
fn gateway_refuses_to_bind_when_demo_mode_set_without_bypass() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("OPENAI_API_KEY", "test-key")
        .env("ARGO_DEMO_MODE", "1")
        .env_remove("ARGO_GATEWAY_AUTH_BYPASS")
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
        .output()
        .expect("spawn");
    assert!(
        !child.status.success(),
        "gateway should have refused to bind",
    );
    let stderr = String::from_utf8_lossy(&child.stderr);
    assert!(
        stderr.contains("ARGO_DEMO_MODE=1 cannot be used in production"),
        "stderr missing demo-mode refusal: {stderr}",
    );
    assert!(
        stderr.contains("ARGO_GATEWAY_AUTH_BYPASS=1"),
        "stderr missing remediation hint: {stderr}",
    );
}
