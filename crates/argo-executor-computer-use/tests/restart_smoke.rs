//! Integration smoke for the restart layer. Spins up a single TCP
//! listener that mimics both E2B and Anthropic by URL path, and
//! drives the full `ComputerUseExecutor::run` loop with a flaky
//! mock that fails twice then succeeds.
//!
//! Asserts the retry path: with `RestartStrategy::RetryWithContext`,
//! a transient 502 from Anthropic does *not* terminate the action
//! prematurely — the executor re-prompts and the third attempt
//! produces a `stop_success`.

use argo_executor_computer_use::{
    ComputerUseExecutor, CuConfig, Outcome, ProfileRegistry, RunRequest,
};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn retry_with_context_recovers_after_two_anthropic_failures() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let base = format!("http://{addr}");

    let anthropic_calls = Arc::new(AtomicU32::new(0));
    let server_calls = Arc::clone(&anthropic_calls);
    let server = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                return;
            };
            let calls = Arc::clone(&server_calls);
            tokio::spawn(async move {
                let mut buf = vec![0u8; 16_384];
                let Ok(n) = stream.read(&mut buf).await else {
                    return;
                };
                if n == 0 {
                    return;
                }
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let response = route_response(&req, &calls);
                let _ = stream.write_all(&response).await;
                let _ = stream.shutdown().await;
            });
        }
    });

    // Profile config for the test: written as TOML so the
    // production loader exercises the same parse path.
    let dir = tempfile::tempdir().expect("tempdir");
    let profiles_path = dir.path().join("computer_use.toml");
    std::fs::write(
        &profiles_path,
        r#"
[profiles.flaky_test]
sandbox_template = "skycot/test:flaky"
max_tokens_per_action = 10000
max_wallclock_seconds = 60
max_steps = 5
screen_width = 800
screen_height = 600
model = "claude-test"

[profiles.flaky_test.restart]
kind = "retry_with_context"
max_attempts = 3
backoff_ms = 0
"#,
    )
    .expect("write profiles");
    let registry = ProfileRegistry::load_from_path(&profiles_path).expect("load");

    let config = CuConfig::for_tests(base.clone(), base.clone());
    let exec = ComputerUseExecutor::new(config, registry);

    let bundle_dir = tempfile::tempdir().expect("tempdir");
    let req = RunRequest {
        action_id: "act-flaky-1".into(),
        job_id: "job-flaky".into(),
        tenant_id: "t-flaky".into(),
        persona_id: "p-flaky".into(),
        profile_name: "flaky_test".into(),
        system_prompt: "Visit example.com and stop_success.".into(),
        budget_usd: None,
        bundle_dir: Some(bundle_dir.path().join("act-flaky-1")),
        session_id: None,
    };

    let outcome = exec.run(req).await.expect("run completes");
    server.abort();

    // The executor should have completed, not handed off, because
    // the third Anthropic call returned stop_success.
    match outcome {
        Outcome::Completed { .. } => {}
        Outcome::HumanHandoff { reason, .. } => {
            panic!("expected Completed but got HumanHandoff: {reason}");
        }
    }
    let calls = anthropic_calls.load(Ordering::SeqCst);
    assert!(
        calls >= 3,
        "expected ≥3 anthropic calls (2 failures + 1 success), got {calls}"
    );
}

/// Route the incoming raw HTTP request to a canned response.
/// Matches by URL prefix only — the mock is intentionally minimal.
fn route_response(req: &str, anthropic_calls: &AtomicU32) -> Vec<u8> {
    let first_line = req.lines().next().unwrap_or("");
    if first_line.contains(" /v1/messages") {
        let n = anthropic_calls.fetch_add(1, Ordering::SeqCst);
        if n < 2 {
            return http_response(502, "application/json", b"{\"error\":\"bad gateway\"}");
        }
        let body = br#"{"content":[{"type":"tool_use","name":"computer","input":{"action":"stop_success","text":"all done"}}],"usage":{"input_tokens":10,"output_tokens":5}}"#;
        return http_response(200, "application/json", body);
    }
    if first_line.starts_with("POST ") && first_line.contains(" /sandboxes/") {
        if first_line.contains("/screenshot") {
            return http_response(200, "image/png", b"PNG_BYTES");
        }
        return http_response(200, "application/json", b"{\"ok\":true}");
    }
    if first_line.starts_with("POST /sandboxes ") {
        return http_response(200, "application/json", b"{\"sandboxId\":\"sb-test\"}");
    }
    if first_line.starts_with("DELETE ") && first_line.contains("/sandboxes/") {
        return http_response(200, "application/json", b"{}");
    }
    http_response(404, "text/plain", b"not found")
}

fn http_response(status: u16, content_type: &str, body: &[u8]) -> Vec<u8> {
    let reason = match status {
        200 => "OK",
        404 => "Not Found",
        502 => "Bad Gateway",
        _ => "Status",
    };
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n",
        len = body.len()
    );
    let mut buf = header.into_bytes();
    buf.extend_from_slice(body);
    buf
}
