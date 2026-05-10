//! Smoke tests for the audit + costs surface —
//! `GET /audit/sessions/<id>/verify`, `GET /audit/breaks`,
//! `GET /audit/count`, and `GET /costs/total`. Pulled out of
//! `gateway_smoke_streaming.rs` so each integration-test binary
//! stays under the 500-LOC ceiling.
mod common;
use common::{argo_bin, await_listening, body_of, http_get, pick_free_port};
use std::io::BufReader;
use std::process::{Command, Stdio};

#[test]
fn audit_session_verify_returns_ok_true_for_empty_session() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "2",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);

    // Bad uuid → 400.
    let resp = http_get(port, "/audit/sessions/not-a-uuid/verify");
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    // Valid uuid with no rows → ok=true, rows=0.
    let resp = http_get(
        port,
        "/audit/sessions/00000000-0000-0000-0000-000000000000/verify",
    );
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.contains(r#""ok":true"#), "body={body}");
    assert!(body.contains(r#""rows":0"#), "body={body}");

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn audit_breaks_returns_empty_array_for_clean_chain() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);

    let resp = http_get(port, "/audit/breaks");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    assert_eq!(body_of(&resp), "[]");

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn costs_total_returns_zero_rollup_for_empty_store() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);

    let resp = http_get(port, "/costs/total");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let body = body_of(&resp);
    let parsed: serde_json::Value = serde_json::from_str(body).expect("valid json");
    assert_eq!(parsed["today"], 0);
    assert_eq!(parsed["this_month"], 0);
    assert_eq!(parsed["all_time"], 0);

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn audit_count_returns_zero_for_clean_chain() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "3",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);

    let resp = http_get(port, "/audit/count");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let parsed: serde_json::Value = serde_json::from_str(body_of(&resp)).expect("json");
    assert_eq!(parsed["count"], 0);

    let resp = http_get(port, "/audit/count?kind=tool_call");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let parsed: serde_json::Value = serde_json::from_str(body_of(&resp)).expect("json");
    assert_eq!(parsed["count"], 0);
    assert_eq!(parsed["kind"], "tool_call");

    let resp = http_get(port, "/audit/count?session=not-a-uuid");
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    let status = child.wait().expect("wait");
    assert!(status.success());
}
