//! Integration smoke for `fix-onboarding-provision-idempotent-flag-and-toast`.
//!
//! Spawns `argo gateway start`, POSTs `/api/v1/onboarding/provision`
//! twice from the same principal, and asserts:
//!
//! - The first call lands `201 Created` with `"created": true`.
//! - The second call lands `200 OK` with `"created": false` and the
//!   long-standing `"idempotent": true` field is preserved.
//!
//! The wizard's workspace step keys off `created` to decide whether to
//! show the celebration toast; this test pins the wire contract so a
//! later refactor can't silently drop the field.

mod common;

use common::listener_signal::{await_listener_signal, StderrDrain};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};

const BEARER_TOKEN: &str = "test-bearer-onboarding-created-flag";
const PRINCIPAL_HEADER: &str = "human:alice@acme.com";
const SMOKE_DB_KEY_B64: &str = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn pick_free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

fn http_post(port: u16, path: &str, body: &str, headers: &[(&str, &str)]) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let mut req = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    for (name, value) in headers {
        req.push_str(&format!("{name}: {value}\r\n"));
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

fn spawn_gateway(home: &Path, port: u16, max_requests: u32) -> (Child, StderrDrain) {
    if std::env::var_os("ARGO_TEST_LISTENER_DEADLINE_MS").is_none() {
        std::env::set_var("ARGO_TEST_LISTENER_DEADLINE_MS", "60000");
    }
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env("SKYCOT_DB_KEY_BASE64", SMOKE_DB_KEY_B64)
        .env("ARGO_GATEWAY_BEARER_TOKEN", BEARER_TOKEN)
        // The `gtm-engineer` lookup pre-flights the on-disk catalog;
        // point at the workspace's bundled vertical pack so a clean
        // ARGO_HOME doesn't 500 on a missing catalog.
        .env("ARGO_VERTICALS_DIR", workspace_verticals_dir())
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
        .expect("spawn");
    let stderr = child.stderr.take().expect("stderr piped");
    let drain = await_listener_signal(stderr).expect("gateway listener signal");
    (child, drain)
}

fn workspace_verticals_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .join("verticals")
}

#[test]
fn provision_returns_created_true_then_false_on_idempotent_recall() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let (mut child, _drain) = spawn_gateway(home.path(), port, 2);

    let bearer = format!("Bearer {BEARER_TOKEN}");
    let auth = ("Authorization", bearer.as_str());
    let principal = ("X-Argo-Principal", PRINCIPAL_HEADER);
    let body = r#"{"workspace_name":"Acme","company_name":"Acme Inc."}"#;

    let first = http_post(
        port,
        "/api/v1/onboarding/provision",
        body,
        &[auth, principal],
    );
    assert!(
        first.contains("HTTP/1.1 201 Created"),
        "first call should land 201 Created: {first}"
    );
    let first_body = body_of(&first);
    assert!(
        first_body.contains("\"created\":true"),
        "fresh provision must surface created:true — body: {first_body}"
    );
    assert!(
        first_body.contains("\"idempotent\":false"),
        "fresh provision must keep idempotent:false — body: {first_body}"
    );

    let second = http_post(
        port,
        "/api/v1/onboarding/provision",
        body,
        &[auth, principal],
    );
    assert!(
        second.contains("HTTP/1.1 200 OK"),
        "second call should land 200 OK: {second}"
    );
    let second_body = body_of(&second);
    assert!(
        second_body.contains("\"created\":false"),
        "idempotent re-call must surface created:false — body: {second_body}"
    );
    assert!(
        second_body.contains("\"idempotent\":true"),
        "idempotent re-call must keep idempotent:true — body: {second_body}"
    );

    let status = child.wait().expect("wait");
    assert!(status.success(), "gateway exited non-zero: {status:?}");
}
