//! Smoke tests for the streaming + observability surface —
//! `POST /forge/optimize/stream`, `GET /metrics`, and
//! `GET /audit/sessions/<id>/verify`. Pulled out of
//! `gateway_smoke_v2.rs` so each integration-test binary
//! stays under the 500-LOC ceiling.
mod common;
use common::{argo_bin, await_listening, body_of, http_get, http_post, pick_free_port};
use std::io::BufReader;
use std::process::{Command, Stdio};

#[test]
fn forge_optimize_stream_returns_chunked_ndjson_with_error_chunk() {
    // The skill doesn't exist, so the spawned task errors out
    // — but we still get a 200 + chunked transfer header + a
    // chunk-terminator. That's enough to prove the routing +
    // chunking framework is wired correctly.
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

    let resp = http_post(
        port,
        "/forge/optimize/stream",
        r#"{"skill":"missing-skill","examples":"missing.jsonl","iterations":1,"budget_usd":0.01}"#,
    );
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    assert!(
        resp.contains("Transfer-Encoding: chunked"),
        "missing chunked header: {resp}"
    );
    let body = body_of(&resp);
    assert!(body.contains(r#""event":"error""#), "body={body}");
    assert!(body.ends_with("0\r\n\r\n"), "no chunk terminator: {body}");

    let status = child.wait().expect("wait");
    assert!(status.success());
}

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
fn session_judge_returns_empty_array_for_clean_session() {
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
    let resp = http_get(port, "/sessions/not-a-uuid/judge");
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    // Valid uuid with no rows → empty array.
    let resp = http_get(
        port,
        "/sessions/00000000-0000-0000-0000-000000000000/judge",
    );
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    assert_eq!(body_of(&resp), "[]");

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn last_message_returns_404_on_session_with_no_messages() {
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

    let resp = http_get(port, "/sessions/not-a-uuid/messages/last");
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    let resp = http_get(
        port,
        "/sessions/00000000-0000-0000-0000-000000000000/messages/last",
    );
    assert!(resp.contains("HTTP/1.1 404 Not Found"), "resp={resp}");

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn workspace_info_auto_bootstraps_on_first_request() {
    // #718: GET /workspace/info now uses cwd_workspace (matches
    // every writer's path). On a fresh DB the gateway's process cwd
    // is materialised into a workspace row by ensure_workspace, so
    // the endpoint returns 200 + the cwd-derived metadata. Pre-#718
    // it returned 404 until `argo workspace init` had run separately.
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

    let resp = http_get(port, "/workspace/info");
    assert!(
        resp.contains("HTTP/1.1 200 OK"),
        "fresh DB should auto-bootstrap; resp={resp}"
    );
    let body = body_of(&resp);
    assert!(
        body.contains("\"workspace_id\"") && body.contains("\"name\""),
        "response should carry workspace_id + name; body={body}"
    );

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn db_stats_returns_zero_counts_for_clean_workspace() {
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

    let resp = http_get(port, "/db/stats");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let body = body_of(&resp);
    let parsed: serde_json::Value = serde_json::from_str(body).expect("valid json");
    assert_eq!(parsed["sessions"], 0);
    assert_eq!(parsed["messages"], 0);
    assert_eq!(parsed["audit_rows"], 0);
    assert_eq!(parsed["forge_runs"], 0);

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn metrics_reset_zeroes_scrape_counter() {
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
            "4",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);

    // Two scrapes → counter at 2.
    let _ = http_get(port, "/metrics");
    let resp = http_get(port, "/metrics");
    assert!(
        body_of(&resp).contains("argo_metrics_scrapes_total 2"),
        "expected scrape=2: {resp}"
    );

    // Reset → returns previous=2, current=0.
    let resp = http_post(port, "/metrics/reset", "");
    let body = body_of(&resp);
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    assert!(body.contains(r#""previous":2"#), "body={body}");
    assert!(body.contains(r#""current":0"#), "body={body}");

    // Next scrape lands at 1, not 3.
    let resp = http_get(port, "/metrics");
    assert!(
        body_of(&resp).contains("argo_metrics_scrapes_total 1"),
        "post-reset scrape should be 1: {resp}"
    );

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

#[test]
fn count_endpoints_return_zero_for_clean_workspace() {
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
            "4",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);

    // Sessions count → 0 on a fresh tempdir.
    let resp = http_get(port, "/sessions/count");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let body = body_of(&resp);
    let parsed: serde_json::Value = serde_json::from_str(body).expect("json");
    assert_eq!(parsed["count"], 0);
    assert_eq!(parsed["status"], "all");

    // Forge runs count → 0.
    let resp = http_get(port, "/forge/runs/count");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let parsed: serde_json::Value = serde_json::from_str(body_of(&resp)).expect("json");
    assert_eq!(parsed["count"], 0);

    // Skills count → 0.
    let resp = http_get(port, "/skills/count");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let parsed: serde_json::Value = serde_json::from_str(body_of(&resp)).expect("json");
    assert_eq!(parsed["count"], 0);
    assert_eq!(parsed["include_superseded"], false);

    // Memory count → 0 (no workspace registered).
    let resp = http_get(port, "/memory/count");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let parsed: serde_json::Value = serde_json::from_str(body_of(&resp)).expect("json");
    assert_eq!(parsed["count"], 0);

    let status = child.wait().expect("wait");
    assert!(status.success());
}

#[test]
fn metrics_endpoint_serves_text_plain_with_prometheus_lines() {
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

    let resp = http_get(port, "/metrics");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    assert!(
        resp.contains("Content-Type: text/plain; version=0.0.4"),
        "missing prometheus content-type: {resp}"
    );
    let body = body_of(&resp);
    assert!(body.contains("# TYPE argo_build_info gauge"), "body={body}");
    assert!(body.contains("argo_build_info{version="), "body={body}");
    assert!(body.contains("argo_sessions_total 0"), "body={body}");
    assert!(body.contains("argo_skills_active_total 0"), "body={body}");
    assert!(body.contains("argo_audit_chain_ok"), "body={body}");

    let status = child.wait().expect("wait");
    assert!(status.success());
}
