//! Integration smoke for `mvp-gateway-request-audit`: every
//! non-skip-list HTTP request to a running gateway appends one
//! `http.request` row to the audit chain, including failure
//! responses, while `/health*` and `/metrics*` produce nothing.
mod common;
use common::{argo_bin, await_listening, http_get, pick_free_port};
use std::io::BufReader;
use std::process::{Command, Stdio};

#[tokio::test]
async fn gateway_request_audit_appends_one_row_per_request_skipping_probes() {
    let _gateway_lock = common::gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        // Bypass the provider-key startup check so the test does
        // not depend on `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` in
        // the test runner's environment — we never make a model
        // call from inside this test.
        .env("OPENAI_API_KEY", "sk-test-only-not-real")
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

    // Two audited requests: one 200 (audited), one 400 (still audited).
    let r = http_get(port, "/sessions");
    assert!(r.contains("HTTP/1.1 200 OK"), "sessions: {r}");
    let r = http_get(port, "/audit/sessions/not-a-uuid/verify");
    assert!(r.contains("HTTP/1.1 400 Bad Request"), "bad verify: {r}");

    // Two skip-list requests: liveness probe + metrics scrape.
    let r = http_get(port, "/health");
    assert!(r.contains("HTTP/1.1 200 OK"), "health: {r}");
    let r = http_get(port, "/metrics");
    assert!(r.contains("HTTP/1.1 200 OK"), "metrics: {r}");

    let status = child.wait().expect("wait");
    assert!(status.success(), "gateway exit: {status}");

    let store = argo_store::SessionStore::open(&home.path().join("sessions.db"))
        .await
        .unwrap();
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT kind, payload_json FROM audit_record WHERE kind = 'http.request' ORDER BY seq",
    )
    .fetch_all(store.pool())
    .await
    .unwrap();
    assert_eq!(
        rows.len(),
        2,
        "expected exactly one http.request per audited request, got {rows:?}"
    );
    for (_, payload_json) in &rows {
        let v: serde_json::Value = serde_json::from_str(payload_json).unwrap();
        assert_eq!(v["method"], "GET", "method should be GET in {payload_json}");
    }
    let first: serde_json::Value = serde_json::from_str(&rows[0].1).unwrap();
    assert_eq!(first["path"], "/sessions");
    assert_eq!(first["status"], 200);
    assert!(first["duration_ms"].as_u64().is_some());

    let second: serde_json::Value = serde_json::from_str(&rows[1].1).unwrap();
    assert_eq!(second["path"], "/audit/sessions/not-a-uuid/verify");
    assert_eq!(second["status"], 400);

    // The chain should still verify after the request burst — no
    // chain breaks introduced by the new wrapper.
    let report = argo_audit::verify_all(&store).await.unwrap();
    assert!(
        report.breaks.is_empty(),
        "audit chain broken after request burst: {:?}",
        report.breaks
    );
}
