//! Regression smoke for `mission-audit-gateway-dashboard-auth-gate`.
//!
//! P0 audit (codex 2026-05-19, finding P0-2) showed that the legacy
//! server-rendered dashboards (`/dashboard/*`, `/dashboard/fleet`)
//! were mounted ahead of the `check_bearer_auth` gate. With
//! `ARGO_GATEWAY_BEARER_TOKEN` configured, spend ledger, audit
//! records, pending approvals, sessions, and worker IDs were
//! readable without an `Authorization: Bearer <token>` header.
//!
//! This test boots the gateway with a bearer token configured and
//! asserts:
//!   1. `GET /dashboard/workers` without an `Authorization` header
//!      returns `401` (gate now runs first).
//!   2. The same request with the matching `Authorization: Bearer`
//!      header returns `200`.
//!   3. `GET /dashboard/fleet` without an `Authorization` header
//!      also returns `401` (fleet view is gated by the same move).
//!
//! Spawn pattern (`Stdio::piped` for stdout/stderr +
//! `common::await_listening` on stderr) mirrors
//! `gateway_tenant_rate_limit_smoke.rs`. The bearer-token env knob,
//! dummy `ANTHROPIC_API_KEY`, and `--skip-startup-check` flag match
//! the same canonical CI shape so this test does not require a real
//! provider key or a live `ANTHROPIC_API_KEY` on the host.

mod common;

use common::{argo_bin, await_listening, gateway_lock_guard, pick_free_port};
use std::io::{BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, ChildStderr, Command, Stdio};

const BEARER_TOKEN: &str = "test-bearer-token-dashboard-auth-gate";

fn spawn_gateway(home: &Path, port: u16, max_requests: u32) -> (Child, BufReader<ChildStderr>) {
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env("ARGO_GATEWAY_BEARER_TOKEN", BEARER_TOKEN)
        // Startup check requires a provider key; supply a dummy and
        // pair with `--skip-startup-check` so the gateway boots in
        // worker-loop / CI envs that do not configure
        // `ANTHROPIC_API_KEY` (CLAUDE.md forbids it for OAuth-billed
        // sessions). Matches `gateway_tenant_rate_limit_smoke.rs`.
        .env("ANTHROPIC_API_KEY", "sk-ant-test-only-not-real")
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            &max_requests.to_string(),
            "--skip-startup-check",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway");
    let stderr = BufReader::new(child.stderr.take().expect("stderr piped"));
    (child, stderr)
}

fn http_get_with_headers(port: u16, path: &str, extra_headers: &[(&str, &str)]) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let mut req = format!(
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n",
    );
    for (name, value) in extra_headers {
        req.push_str(&format!("{name}: {value}\r\n"));
    }
    req.push_str("\r\n");
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

#[test]
fn dashboard_routes_require_bearer_when_token_configured() {
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().expect("home");
    // 3 GETs: unauthenticated /dashboard/workers, authenticated
    // /dashboard/workers, unauthenticated /dashboard/fleet.
    let max_requests = 3;
    let (mut child, mut stderr) = spawn_gateway(home.path(), port, max_requests);
    await_listening(&mut stderr);

    let bearer = format!("Bearer {BEARER_TOKEN}");
    let auth_header = ("Authorization", bearer.as_str());

    // (1) Unauthenticated dashboard request must be rejected by the
    // bearer-auth gate now that it runs before dashboard dispatch.
    let resp = http_get_with_headers(port, "/dashboard/workers", &[]);
    assert!(
        resp.contains("HTTP/1.1 401 Unauthorized"),
        "expected 401 without bearer for /dashboard/workers, got:\n{resp}"
    );

    // (2) Same request with the matching bearer passes the gate and
    // renders the workers tab.
    let resp = http_get_with_headers(port, "/dashboard/workers", &[auth_header]);
    assert!(
        resp.contains("HTTP/1.1 200 OK"),
        "expected 200 with bearer for /dashboard/workers, got:\n{resp}"
    );

    // (3) Fleet view is gated by the same reorder.
    let resp = http_get_with_headers(port, "/dashboard/fleet", &[]);
    assert!(
        resp.contains("HTTP/1.1 401 Unauthorized"),
        "expected 401 without bearer for /dashboard/fleet, got:\n{resp}"
    );

    let status = child.wait().expect("wait gateway");
    assert!(status.success(), "gateway exited with {status}");
}
