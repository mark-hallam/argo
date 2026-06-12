//! Smoke test for `X-Argo-Principal` audit-payload stamping.
//!
//! Boots the gateway, exercises `POST /sessions/<id>/approve` and
//! `POST /sessions/<id>/revoke-approval` with and without the
//! principal header, then asserts the audit rows (re-opened from
//! the on-disk SQLite store) carry the expected `principal` field.
//!
//! Uses `common::listener_signal::await_listener_signal` (background
//! drain thread + env-var deadline override) so the test survives
//! loaded shared-target runs where the freshly-built binary is slow
//! to first-emit on stderr.

mod common;

use common::listener_signal::{await_listener_signal, StderrDrain};
use common::{body_of, pick_free_port};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use uuid::Uuid;

const SMOKE_DB_KEY_B64: &str = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";

fn boot_gateway(home: &Path, port: u16) -> (Child, StderrDrain) {
    if std::env::var_os("ARGO_TEST_LISTENER_DEADLINE_MS").is_none() {
        std::env::set_var("ARGO_TEST_LISTENER_DEADLINE_MS", "60000");
    }
    let mut child = Command::new(common::argo_bin())
        .env("ARGO_HOME", home)
        .env("SKYCOT_DB_KEY_BASE64", SMOKE_DB_KEY_B64)
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "4",
            "--skip-startup-check",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway");
    let stderr = child.stderr.take().expect("stderr piped");
    let drain = await_listener_signal(stderr).expect("gateway listener signal");
    (child, drain)
}

fn http_post_with_principal(port: u16, path: &str, body: &str, principal: Option<&str>) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let mut req = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(principal) = principal {
        req.push_str(&format!("X-Argo-Principal: {principal}\r\n"));
    }
    req.push_str("\r\n");
    req.push_str(body);
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

#[tokio::test]
async fn gateway_auth_principal_stamps_approval_audit_payloads() {
    let _gateway_lock = common::gateway_lock_guard();
    let home = tempfile::tempdir().unwrap();
    let port = pick_free_port();
    let (mut child, _drain) = boot_gateway(home.path(), port);

    let resp = http_post_with_principal(
        port,
        "/sessions",
        r#"{"model":"claude-haiku-4-5","provider":"anthropic","title":"principal"}"#,
        None,
    );
    assert!(
        resp.contains("HTTP/1.1 201 Created"),
        "create session: {resp}"
    );
    let created: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    let sid = created["id"].as_str().unwrap();

    let worker = Uuid::parse_str("018f55b7-89ab-7cde-8123-456789abcdef").unwrap();
    let resp = http_post_with_principal(
        port,
        &format!("/sessions/{sid}/approve"),
        r#"{"tool":"delegate_task"}"#,
        Some(&format!("worker:{worker}")),
    );
    assert!(resp.contains("HTTP/1.1 200 OK"), "approve: {resp}");

    let resp = http_post_with_principal(
        port,
        &format!("/sessions/{sid}/revoke-approval"),
        r#"{"tool":"delegate_task"}"#,
        None,
    );
    assert!(resp.contains("HTTP/1.1 200 OK"), "revoke: {resp}");

    let resp = http_post_with_principal(
        port,
        &format!("/sessions/{sid}/approve"),
        r#"{"tool":"delegate_task"}"#,
        Some("worker:not-a-uuid"),
    );
    assert!(
        resp.contains("HTTP/1.1 400 Bad Request"),
        "bad principal: {resp}"
    );
    // Structured error envelope: `{"error":{"code","message"}}` so the SPA's
    // typed-error path (`error.code`) resolves rather than falling through to
    // the generic "Something went wrong".
    let err: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    assert_eq!(err["error"]["code"], "bad_request", "body: {}", body_of(&resp));
    assert_eq!(err["error"]["message"], "bad principal");

    let status = child.wait().expect("wait gateway");
    assert!(status.success(), "gateway exited with {status}");

    let store = argo_store::SessionStore::open(&home.path().join("sessions.db"))
        .await
        .unwrap();
    let granted: String = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record WHERE kind = 'approval.granted' \
         ORDER BY seq DESC LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    let granted: serde_json::Value = serde_json::from_str(&granted).unwrap();
    assert_eq!(granted["principal"], format!("worker:{worker}"));

    let revoked: String = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record WHERE kind = 'approval.revoked' \
         ORDER BY seq DESC LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    let revoked: serde_json::Value = serde_json::from_str(&revoked).unwrap();
    assert_eq!(revoked["principal"], "anonymous");
}
