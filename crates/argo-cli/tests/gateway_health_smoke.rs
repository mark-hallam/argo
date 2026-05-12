//! Integration smoke for `mission-health-check-endpoint`: boots a
//! real gateway, hits `/health`, `/readyz`, and `/livez`, and
//! verifies:
//!
//!   - `/health` returns 200 with the new shape (`status:"ok"`,
//!     numeric `schema_version`, numeric `uptime_seconds`).
//!   - `/readyz` returns the same shape (alias contract).
//!   - `/livez` returns 200 with `status:"alive"` regardless of
//!     readiness checks.
//!   - All three endpoints are reachable without an
//!     `Authorization: Bearer` header (k8s/systemd require this).
//!   - `uptime_seconds` is non-decreasing across two consecutive
//!     `/health` probes with a sleep between them.
//!
//! The gateway is spawned with `--skip-startup-check` so it boots
//! without a real provider key (workers run without
//! `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` set). Readiness uses the
//! channel-based `await_listener_signal` helper for robust
//! detection of a successful TCP bind vs a startup crash.
mod common;
use common::listener_signal::await_listener_signal;
use common::{argo_bin, body_of, gateway_lock_guard, http_get, pick_free_port};
use std::process::{Command, Stdio};

#[test]
fn gateway_health_endpoints_match_brief_contract() {
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().expect("tempdir");
    // We hit four GET endpoints (/health x2, /readyz, /livez);
    // spawn with max-requests=4 so the gateway exits after the 4th.
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
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
        .expect("spawn");
    let stderr = child.stderr.take().expect("stderr piped");
    let _drain = await_listener_signal(stderr).expect("gateway listener signal");

    // /health — new shape on the happy path.
    let resp = http_get(port, "/health");
    assert!(resp.contains("HTTP/1.1 200 OK"), "health resp={resp}");
    let body = body_of(&resp);
    let parsed: serde_json::Value =
        serde_json::from_str(body).expect("/health body should parse");
    assert_eq!(parsed["status"], "ok", "/health status: {body}");
    assert!(
        parsed["schema_version"].as_u64().is_some(),
        "/health missing schema_version: {body}"
    );
    let u1 = parsed["uptime_seconds"]
        .as_u64()
        .expect("/health missing uptime_seconds");

    // /readyz — same shape as /health (alias contract).
    let resp = http_get(port, "/readyz");
    assert!(resp.contains("HTTP/1.1 200 OK"), "readyz resp={resp}");
    let body = body_of(&resp);
    let parsed: serde_json::Value =
        serde_json::from_str(body).expect("/readyz body should parse");
    assert_eq!(parsed["status"], "ok", "/readyz status: {body}");
    assert!(
        parsed["schema_version"].as_u64().is_some()
            && parsed["uptime_seconds"].as_u64().is_some(),
        "/readyz must mirror /health shape: {body}"
    );

    // /livez — always 200, status:"alive", no schema/uptime fields.
    let resp = http_get(port, "/livez");
    assert!(resp.contains("HTTP/1.1 200 OK"), "livez resp={resp}");
    let body = body_of(&resp);
    let parsed: serde_json::Value =
        serde_json::from_str(body).expect("/livez body should parse");
    assert_eq!(parsed["status"], "alive", "/livez status: {body}");

    // uptime monotonicity: two /health probes a second apart should
    // show u2 >= u1.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let resp = http_get(port, "/health");
    let parsed: serde_json::Value =
        serde_json::from_str(body_of(&resp)).expect("/health 2 parse");
    let u2 = parsed["uptime_seconds"]
        .as_u64()
        .expect("/health 2 missing uptime_seconds");
    assert!(u2 >= u1, "uptime regressed: u1={u1} u2={u2}");

    let status = child.wait().expect("wait");
    assert!(status.success(), "gateway exit: {status}");
}
