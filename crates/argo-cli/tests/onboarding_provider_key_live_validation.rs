//! Integration guard for `sprint-ws2-3-live-provider-key-validation`,
//! kept **fully offline / deterministic**.
//!
//! The new live one-token validation is inserted into
//! `provider_key_response` AFTER the WS4-3 tier gate and AFTER the
//! shape pre-check. This test pins that ordering for a fresh tenant:
//! with the live gate turned ON and a *mapped* provider (`anthropic`,
//! which would otherwise dial `api.anthropic.com`), a fresh Free-tier
//! tenant is rejected by the tier gate (`403 tier_forbidden`) and so
//! NEVER reaches the network call — the gateway answers promptly, no
//! hang, no `500`. That is the structural guarantee that a Free tenant
//! can never trigger a provider dial-out from the save path.
//!
//! The live-validation decision logic itself (gate on/off,
//! provider→manifest mapping, `401`/`403` → reject vs. transient →
//! proceed, network-skip) is exercised offline in the unit tests at
//! `crates/argo-cli/src/handlers/gateway/routes_onboarding_live_key_tests.rs`.
//! A true "valid key stores" / "bad key rejected" E2E would need a
//! Pro/Enterprise tenant (to clear the tier gate) plus a live or
//! mock-redirected provider endpoint — neither is CI-safe here, so the
//! unit tests own that coverage.

mod common;

use common::listener_signal::{await_listener_signal, StderrDrain};
use common::{argo_bin, body_of, http_get, pick_free_port};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

const PRINCIPAL: &str = "human:live-key@example.com";
// 32 bytes base64 — reused for both the DB key and the workspace root
// key; in this offline test their concrete values are irrelevant beyond
// being valid 32-byte secrets so credential wrap could succeed.
const SMOKE_KEY_B64: &str = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

/// Boot the gateway with a valid workspace root key and the live
/// validation gate turned ON.
fn boot_gate_on(port: u16, max_requests: u32) -> (Child, StderrDrain) {
    let root = workspace_root();
    let home = tempfile::tempdir().expect("tempdir").keep();
    let mut child = Command::new(argo_bin())
        .current_dir(&root)
        .env("ARGO_HOME", &home)
        .env("HOME", &home)
        .env("ARGO_VERTICALS_DIR", root.join("verticals"))
        .env("OPENAI_API_KEY", "sk-test-only-not-real")
        .env("SKYCOT_DB_KEY_BASE64", SMOKE_KEY_B64)
        .env("SKYCOT_WORKSPACE_ROOT_KEY_BASE64", SMOKE_KEY_B64)
        // Gate ON — if the live call were misplaced BEFORE the tier
        // gate, a fresh Free tenant + mapped provider would dial out and
        // this test would hang / flake. It must not.
        .env("ARGO_LIVE_KEY_VALIDATION", "1")
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
        .expect("spawn");
    let stderr = child.stderr.take().expect("stderr piped");
    let drain = await_listener_signal(stderr).expect("gateway listener signal");
    let resp = http_get(port, "/sessions");
    assert!(
        resp.contains("HTTP/1.1 200 OK"),
        "gateway store not ready: {resp}"
    );
    (child, drain)
}

fn http_post_json(port: u16, path: &str, body: &str, headers: &[(&str, &str)]) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let mut req =
        format!("POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n");
    for (name, value) in headers {
        req.push_str(name);
        req.push_str(": ");
        req.push_str(value);
        req.push_str("\r\n");
    }
    req.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    ));
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

#[test]
fn free_tenant_never_dials_out_even_with_gate_on() {
    let port = pick_free_port();
    let (mut child, _drain) = boot_gate_on(port, 2);

    let provision = http_post_json(
        port,
        "/api/v1/onboarding/provision",
        r#"{"workspace_name":"Acme","company_name":"Acme Co"}"#,
        &[("X-Argo-Principal", PRINCIPAL)],
    );
    assert!(
        provision.contains("HTTP/1.1 201 Created") || provision.contains("HTTP/1.1 200 OK"),
        "provision response: {provision}"
    );

    // Mapped provider (`anthropic`) + gate ON, but the fresh tenant is
    // Free → the tier gate rejects with 403 BEFORE the live call. The
    // gateway answers immediately (no network hang), proving the live
    // check sits after the tier gate. The reject body must be the tier
    // upsell, not a provider error / 500.
    let resp = http_post_json(
        port,
        "/api/v1/onboarding/provider-key",
        r#"{"provider":"anthropic","api_key":"sk-ant-valid-shape-test-key-000000"}"#,
        &[("X-Argo-Principal", PRINCIPAL)],
    );
    assert!(
        resp.contains("HTTP/1.1 403 Forbidden"),
        "expected tier 403 before any live call; got: {resp}"
    );
    let parsed: Value = serde_json::from_str(body_of(&resp)).expect("json body");
    assert_eq!(parsed["error"], "tier_forbidden", "body: {resp}");
    assert_eq!(parsed["feature"], "provider_real_keys", "body: {resp}");

    let status = child.wait().expect("wait");
    assert!(status.success(), "gateway exit: {status}");
}
