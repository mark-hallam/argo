//! Integration smoke test for per-IP token-bucket rate limiting
//! (claude-security-rate-limit-by-ip).
//!
//! Spawns the gateway with a tiny capacity + slow refill, hammers
//! `/health` from a single IP (loopback), and asserts that excess
//! requests get a `429 Too Many Requests` response with a
//! `Retry-After` header and a JSON `{"error":"rate_limited",...}`
//! body.
mod common;
use common::{argo_bin, await_listening, body_of, gateway_lock_guard, http_get, pick_free_port};
use std::io::BufReader;
use std::process::{Command, Stdio};

#[test]
fn gateway_per_ip_rate_limit_returns_429_with_retry_after() {
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();

    // Capacity 2 + glacial refill (0.01/sec) → after the first 2
    // requests the bucket is empty for the duration of the test.
    // --max-requests of 6 is generous: 2 allowed health checks,
    // several denials, and headroom for any startup pings.
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_IP_RATE_LIMIT", "1")
        .env("ARGO_IP_RATE_CAPACITY", "2")
        .env("ARGO_IP_RATE_REFILL", "0.01")
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "6",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");

    let mut stderr = BufReader::new(child.stderr.take().expect("stderr"));
    await_listening(&mut stderr);

    // First two calls drain the bucket.
    let r1 = http_get(port, "/health");
    assert!(r1.contains("HTTP/1.1 200 OK"), "first call: {r1}");
    let r2 = http_get(port, "/health");
    assert!(r2.contains("HTTP/1.1 200 OK"), "second call: {r2}");

    // Third call should be rate-limited with 429 + Retry-After + JSON body.
    let r3 = http_get(port, "/health");
    assert!(
        r3.contains("HTTP/1.1 429 Too Many Requests"),
        "third call should be 429, got: {r3}"
    );
    assert!(r3.contains("Retry-After:"), "missing Retry-After: {r3}");
    let body = body_of(&r3);
    assert!(body.contains("\"error\":\"rate_limited\""), "body={body}");
    assert!(body.contains("\"scope\":\"ip\""), "body={body}");
    assert!(body.contains("\"retry_after\":"), "body={body}");

    let _ = child.kill();
    let _ = child.wait();
}
