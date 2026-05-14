//! Integration smoke for `mission-prometheus-exporter-openmetrics-route`:
//! boots a real gateway, scrapes BOTH `/metrics` (legacy
//! store-derived 18-metric handler) and `/openmetrics` (new
//! prometheus-registry-backed exposition), and asserts:
//!
//!   - Both return 200 + `text/plain; version=0.0.4`.
//!   - `/metrics` still carries its legacy series (`argo_build_info`,
//!     `argo_store_available`) — the brief is non-breaking.
//!   - `/openmetrics` carries every brief-specified metric family by
//!     name (HELP / TYPE lines).
//!   - `/openmetrics` is reachable without a bearer token (public-GET
//!     allowlist).
//!
//! Spawned with `--skip-startup-check` so no provider key is needed.

mod common;
use common::listener_signal::await_listener_signal;
use common::{argo_bin, body_of, gateway_lock_guard, http_get, pick_free_port};
use std::process::{Command, Stdio};

#[test]
fn gateway_serves_metrics_and_openmetrics_routes() {
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().expect("tempdir");
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        // Hits two GETs (/metrics + /openmetrics); cap at 2 so the
        // gateway exits cleanly after the smoke.
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

    // Legacy /metrics — unchanged store-derived handler. Asserts the
    // brief's non-breaking promise holds.
    let resp = http_get(port, "/metrics");
    assert!(resp.contains("HTTP/1.1 200 OK"), "/metrics resp={resp}");
    assert!(
        resp.contains("Content-Type: text/plain; version=0.0.4"),
        "/metrics content-type: {resp}"
    );
    let body = body_of(&resp);
    assert!(
        body.contains("argo_build_info"),
        "/metrics should carry the legacy build_info series: {body}"
    );

    // New /openmetrics — prometheus-registry-backed, carries the 8
    // brief-specified families.
    let resp = http_get(port, "/openmetrics");
    assert!(
        resp.contains("HTTP/1.1 200 OK"),
        "/openmetrics resp={resp}"
    );
    assert!(
        resp.contains("Content-Type: text/plain; version=0.0.4"),
        "/openmetrics content-type: {resp}"
    );
    let body = body_of(&resp);
    for required in [
        "gateway_request_total",
        "gateway_request_duration_seconds",
        "gateway_rate_limit_throttle_total",
        "provider_call_total",
        "provider_breaker_state",
        "provider_cost_micros_total",
        "audit_chain_tail_seq",
        "store_migration_chain_version",
    ] {
        assert!(
            body.contains(required),
            "/openmetrics missing brief-specified metric {required}:\n{body}"
        );
    }

    let status = child.wait().expect("wait");
    assert!(status.success(), "gateway exit: {status}");
}
