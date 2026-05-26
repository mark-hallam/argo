//! Smoke coverage for route-level audit 404s versus missing audit rows.

mod common;

use common::listener_signal::await_listener_signal;
use common::{argo_bin, body_of, gateway_lock_guard, http_get, pick_free_port};
use std::process::{Command, Stdio};

const SMOKE_DB_KEY_B64: &str = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";

#[test]
fn audit_subroute_typo_returns_endpoint_not_found_without_hiding_missing_rows() {
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().expect("tempdir");
    if std::env::var_os("ARGO_TEST_LISTENER_DEADLINE_MS").is_none() {
        std::env::set_var("ARGO_TEST_LISTENER_DEADLINE_MS", "60000");
    }
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("SKYCOT_DB_KEY_BASE64", SMOKE_DB_KEY_B64)
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "2",
            "--skip-startup-check",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let stderr = child.stderr.take().expect("stderr piped");
    let _drain = await_listener_signal(stderr).expect("gateway listener signal");

    let typo = http_get(port, "/api/v1/audit/feed?limit=3");
    assert!(typo.contains("HTTP/1.1 404 Not Found"), "typo={typo}");
    let typo_body = body_of(&typo);
    assert!(
        typo_body.contains(r#""endpoint_not_found""#),
        "typo_body={typo_body}"
    );
    assert!(
        !typo_body.contains("row"),
        "route-level 404 should not mention rows: {typo_body}"
    );

    let missing = http_get(port, "/api/v1/audit/00112233445566778899aabbccddeeff");
    assert!(
        missing.contains("HTTP/1.1 404 Not Found"),
        "missing={missing}"
    );
    assert!(
        body_of(&missing).contains("audit row not found"),
        "missing body={}",
        body_of(&missing)
    );

    let status = child.wait().expect("wait");
    assert!(status.success(), "gateway exit: {status}");
}
