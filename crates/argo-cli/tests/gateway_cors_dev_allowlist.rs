//! Gateway CORS regression for recorder/dev loopback origins.
//!
//! Dev gateways must accept browser calls from localhost preview
//! proxies without requiring every ephemeral port to be listed in
//! `gateway.cors.allowed_origins`. Production mode remains exact-match
//! only; that strict branch is pinned in the CORS unit tests.

mod common;
use common::listener_signal::await_listener_signal;
use common::{argo_bin, gateway_lock_guard, pick_free_port};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};

const DEV_ORIGINS: &[&str] = &[
    "http://localhost:3000",
    "http://127.0.0.1:3000",
    "http://localhost:53854",
    "http://localhost:3001",
];
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

#[test]
fn dev_gateway_allows_loopback_origin_variants() {
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().expect("tempdir");
    std::env::set_var("ARGO_TEST_LISTENER_DEADLINE_MS", "30000");

    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("SKYCOT_DB_KEY_BASE64", SMOKE_DB_KEY_B64)
        .env_remove("ARGO_GATEWAY_PROFILE")
        .env_remove("ARGO_GATEWAY_BIND")
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            &DEV_ORIGINS.len().to_string(),
            "--skip-startup-check",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let stderr = child.stderr.take().expect("stderr piped");
    let _drain = await_listener_signal(stderr).expect("gateway listener signal");

    for origin in DEV_ORIGINS {
        let resp = http_options(port, "/api/v1/auth/signup", origin);
        assert!(
            resp.contains("HTTP/1.1 204 No Content"),
            "preflight should be 204 for {origin}; resp={resp}"
        );
        assert!(
            resp.contains(&format!("Access-Control-Allow-Origin: {origin}")),
            "preflight should echo dev origin {origin}; resp={resp}"
        );
        assert!(
            resp.contains("Access-Control-Allow-Credentials: true"),
            "preflight should allow credentials for {origin}; resp={resp}"
        );
        assert!(
            resp.contains("Vary: Origin"),
            "preflight should set Vary: Origin for {origin}; resp={resp}"
        );
    }

    let status = child.wait().expect("wait");
    assert!(status.success(), "gateway exit: {status}");
}
