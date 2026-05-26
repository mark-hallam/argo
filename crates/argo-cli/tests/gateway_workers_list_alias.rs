//! Smoke coverage for `GET /api/v1/workers` as the canonical worker
//! collection endpoint and `/api/v1/team` as its deprecated alias.

mod common;

use common::listener_signal::await_listener_signal;
use common::{argo_bin, body_of, gateway_lock_guard, http_get, pick_free_port};
use std::process::{Command, Stdio};

const SMOKE_DB_KEY_B64: &str = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";

#[test]
fn workers_collection_matches_team_alias_and_team_is_deprecated() {
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

    let workers_resp = http_get(port, "/api/v1/workers");
    assert!(
        workers_resp.contains("HTTP/1.1 200 OK"),
        "workers_resp={workers_resp}"
    );
    assert!(
        !workers_resp.contains("\r\nDeprecation: true\r\n"),
        "canonical workers endpoint must not be deprecated: {workers_resp}"
    );
    let workers_body: serde_json::Value =
        serde_json::from_str(body_of(&workers_resp)).expect("workers json");
    assert!(
        workers_body["workers"].is_array(),
        "workers body={workers_body}"
    );
    assert!(
        workers_body["total_count"].as_u64().is_some(),
        "workers body={workers_body}"
    );

    let team_resp = http_get(port, "/api/v1/team");
    assert!(
        team_resp.contains("HTTP/1.1 200 OK"),
        "team_resp={team_resp}"
    );
    assert!(
        team_resp.contains("\r\nDeprecation: true\r\n"),
        "team endpoint should advertise deprecation: {team_resp}"
    );
    assert!(
        team_resp.contains("\r\nSunset: Wed, 01 Jan 2027 00:00:00 GMT\r\n"),
        "team endpoint should advertise sunset: {team_resp}"
    );
    let team_body: serde_json::Value =
        serde_json::from_str(body_of(&team_resp)).expect("team json");
    assert_eq!(team_body["workers"], workers_body["workers"]);
    assert_eq!(team_body["total_count"], workers_body["total_count"]);

    let status = child.wait().expect("wait");
    assert!(status.success(), "gateway exit: {status}");
}
