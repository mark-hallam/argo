//! Integration smoke for `mission-gateway-cors-preflight`: boots a
//! real gateway, sends an OPTIONS preflight and a follow-up request
//! against `/api/v1/*`, and verifies:
//!
//!   - `OPTIONS /api/v1/auth/signup` with an allowlisted Origin
//!     returns `204 No Content` with the full set of
//!     `Access-Control-Allow-*` headers + `Vary: Origin`.
//!   - The same preflight with a non-allowlisted Origin still
//!     returns 204 but carries no `Access-Control-Allow-Origin`
//!     header — the browser-side block path.
//!   - A regular `POST /api/v1/auth/signup` from an allowlisted
//!     Origin carries `Access-Control-Allow-Origin: <Origin>` on
//!     its response so the browser will consume the body once the
//!     preflight has cleared.
//!
//! The gateway is spawned with `--skip-startup-check` so it boots
//! without a real provider key (workers run without
//! `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` set).

mod common;
use common::listener_signal::await_listener_signal;
use common::{argo_bin, gateway_lock_guard, pick_free_port};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};

const ALLOWED_ORIGIN: &str = "http://localhost:3000";
const FORBIDDEN_ORIGIN: &str = "https://evil.example.com";
const SMOKE_DB_KEY_B64: &str = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";

fn http_options(port: u16, path: &str, origin: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let req = format!(
        "OPTIONS {path} HTTP/1.1\r\n\
         Host: localhost\r\n\
         Origin: {origin}\r\n\
         Access-Control-Request-Method: POST\r\n\
         Access-Control-Request-Headers: Content-Type, Authorization\r\n\
         Connection: close\r\n\r\n"
    );
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

fn http_post_with_origin(port: u16, path: &str, body: &str, origin: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let req = format!(
        "POST {path} HTTP/1.1\r\n\
         Host: localhost\r\n\
         Origin: {origin}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

#[test]
fn cors_preflight_round_trip() {
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().expect("tempdir");
    // Extend the listener-signal deadline (default 10s). Gateway
    // boot on a loaded build host (concurrent cargo runs in the
    // shared target dir) routinely brushes against the default;
    // 30s is comfortably above the observed worst case and is
    // forgiving without masking a real boot regression. The env
    // var is read inside `await_listener_signal` so setting it
    // here, before the spawn, is sufficient.
    std::env::set_var("ARGO_TEST_LISTENER_DEADLINE_MS", "30000");
    // 3 requests: allowed preflight, forbidden preflight, real POST.
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("SKYCOT_DB_KEY_BASE64", SMOKE_DB_KEY_B64)
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "3",
            "--skip-startup-check",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let stderr = child.stderr.take().expect("stderr piped");
    let _drain = await_listener_signal(stderr).expect("gateway listener signal");

    // 1) Allowlisted Origin preflight — full CORS header set.
    let resp = http_options(port, "/api/v1/auth/signup", ALLOWED_ORIGIN);
    assert!(
        resp.contains("HTTP/1.1 204 No Content"),
        "preflight should be 204; resp={resp}"
    );
    assert!(
        resp.contains(&format!("Access-Control-Allow-Origin: {ALLOWED_ORIGIN}")),
        "preflight echoes Origin; resp={resp}"
    );
    assert!(
        resp.contains("Access-Control-Allow-Credentials: true"),
        "preflight asserts credentials; resp={resp}"
    );
    assert!(
        resp.contains("Access-Control-Allow-Methods"),
        "preflight advertises methods; resp={resp}"
    );
    assert!(
        resp.contains("Access-Control-Allow-Headers"),
        "preflight advertises headers; resp={resp}"
    );
    assert!(
        resp.contains("Vary: Origin"),
        "preflight sets Vary: Origin for caches; resp={resp}"
    );

    // 2) Non-allowlisted Origin preflight — 204, but no
    //    Access-Control-Allow-Origin (browser blocks).
    let resp = http_options(port, "/api/v1/auth/signup", FORBIDDEN_ORIGIN);
    assert!(
        resp.contains("HTTP/1.1 204 No Content"),
        "forbidden preflight still 204; resp={resp}"
    );
    assert!(
        !resp.contains("Access-Control-Allow-Origin:"),
        "forbidden preflight must NOT echo Origin; resp={resp}"
    );

    // 3) Real POST from the allowed origin — CORS headers stamped
    //    on the actual response (whatever its status). We send an
    //    empty body to keep this independent of magic-link config;
    //    the server replies with a 400/422-shaped error and the
    //    CORS headers we care about are present on that response.
    let resp = http_post_with_origin(port, "/api/v1/auth/signup", "{}", ALLOWED_ORIGIN);
    assert!(
        resp.contains(&format!("Access-Control-Allow-Origin: {ALLOWED_ORIGIN}")),
        "regular POST response carries ACAO; resp={resp}"
    );
    assert!(
        resp.contains("Vary: Origin"),
        "regular POST response carries Vary: Origin; resp={resp}"
    );

    let status = child.wait().expect("wait");
    assert!(status.success(), "gateway exit: {status}");
}
