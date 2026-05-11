//! Integration smoke test for per-tenant rate limiting
//! (`mission-per-tenant-rate-limiting`, MISSION_BACKLOG Section 1
//! #21).
//!
//! Spawns the gateway with a tiny per-tenant budget
//! (`ARGO_RATE_LIMIT_TENANT_PER_MIN=2`), pins a bearer token so the
//! `/sessions` route auth-gate clears (and `ARGO_GATEWAY_AUTH_BYPASS`
//! is *not* set, which would short-circuit the limiter), then hammers
//! `GET /providers?tenant=<uuid>` from a single client and asserts:
//!
//!  - first 2 calls clear the limiter,
//!  - the 3rd is rejected with `429 Too Many Requests` + the JSON
//!    body the brief calls for (`error=rate_limited`, `scope=tenant`,
//!    `retry_after`),
//!  - a request tagged with a *different* tenant in the same window
//!    is not affected (cross-tenant isolation).
//!
//! The brief asks for an HTTP `Retry-After` header on the 429; the
//! gateway frames responses in `boot.rs` (outside this brief's
//! may-touch list) with a fixed header set, so the retry interval is
//! surfaced inside the JSON body as `retry_after` / `retry_after_secs`
//! only — same "Sensible deviation" the per-IP and per-route briefs
//! recorded. See the module-level doc comment in
//! `crates/argo-cli/src/handlers/gateway/rate_limit_tenant.rs`.
mod common;
use common::{argo_bin, await_listening, body_of, gateway_lock_guard, pick_free_port};
use std::io::{BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};

const TENANT_A: &str = "00000000-0000-0000-0000-0000000000a1";
const TENANT_B: &str = "00000000-0000-0000-0000-0000000000b2";
const BEARER: &str = "test-only-bearer-not-real";

/// GET request with a fixed `Authorization: Bearer` header. The
/// shared [`common::http_get`] helper does not set custom headers,
/// and this brief's may-touch list doesn't include `common/mod.rs`,
/// so we inline the request here.
fn get_with_bearer(port: u16, path: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let req = format!(
        "GET {path} HTTP/1.1\r\n\
         Host: localhost\r\n\
         Authorization: Bearer {BEARER}\r\n\
         Connection: close\r\n\r\n"
    );
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

#[test]
fn gateway_per_tenant_rate_limit_returns_429_with_retry_after_json() {
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();

    // Per-tenant budget = 1 rpm with the default 2.0 burst
    // multiplier ⇒ bucket capacity = 2. After two requests for the
    // same tenant the bucket is empty until the next refill (~60s
    // at rpm=1), so the 3rd within the test window is denied. The
    // env knob only overrides `requests_per_minute`; the burst
    // multiplier stays at the
    // `DEFAULT_GATEWAY_RATE_LIMIT_TENANT_BURST = 2.0` config
    // default, so rpm=1 with burst=2.0 gives a 2-token bucket.
    //
    // --max-requests is sized for: 2 allowed tenant_a calls + 1
    // denied tenant_a call + 1 allowed tenant_b call + headroom for
    // startup pings.
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_RATE_LIMIT_TENANT_PER_MIN", "1")
        // The per-IP limiter would also fire from loopback under
        // its tiny defaults; disable it so the *tenant* layer is
        // what we actually observe here.
        .env("ARGO_IP_RATE_LIMIT", "0")
        // Pin a fixed bearer + supply a matching Authorization
        // header on every request. AUTH_BYPASS=1 would also pass
        // the auth gate but the tenant limiter short-circuits when
        // it's set (so the limiter wouldn't fire at all).
        .env("ARGO_GATEWAY_BEARER_TOKEN", BEARER)
        // Startup-check expects a provider key; supply a dummy so
        // the gateway boots in a worker-loop env where the host
        // does NOT set ANTHROPIC_API_KEY (CLAUDE.md forbids it for
        // OAuth-billed sessions). Pair with `--skip-startup-check`
        // for a clean dev/CI environment.
        .env("ANTHROPIC_API_KEY", "sk-ant-test-only-not-real")
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "8",
            "--skip-startup-check",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");

    let mut stderr = BufReader::new(child.stderr.take().expect("stderr"));
    await_listening(&mut stderr);

    let path_a = format!("/providers?tenant={TENANT_A}");
    let path_b = format!("/providers?tenant={TENANT_B}");

    // First two `?tenant=<TENANT_A>` calls drain that tenant's
    // bucket. The route returns 200 with a list body (empty by
    // default for a fresh ARGO_HOME).
    let r1 = get_with_bearer(port, &path_a);
    assert!(r1.contains("HTTP/1.1 200 OK"), "first tenant_a call: {r1}");
    let r2 = get_with_bearer(port, &path_a);
    assert!(r2.contains("HTTP/1.1 200 OK"), "second tenant_a call: {r2}");

    // Third call from tenant_a: empty bucket → 429 with the JSON
    // body shape the mission brief defines.
    let r3 = get_with_bearer(port, &path_a);
    assert!(
        r3.contains("HTTP/1.1 429 Too Many Requests"),
        "third tenant_a call should be 429, got: {r3}"
    );
    let body = body_of(&r3);
    assert!(
        body.contains("\"error\":\"rate_limited\""),
        "missing rate_limited error in body: {body}"
    );
    assert!(
        body.contains("\"scope\":\"tenant\""),
        "missing scope=tenant in body: {body}"
    );
    assert!(
        body.contains("\"retry_after\":"),
        "missing retry_after in body: {body}"
    );

    // Cross-tenant isolation: a different `?tenant=` still has its
    // own bucket. This is what makes the per-tenant layer useful —
    // a noisy tenant shouldn't throttle a quiet neighbour sharing
    // the same source IP.
    let r4 = get_with_bearer(port, &path_b);
    assert!(
        r4.contains("HTTP/1.1 200 OK"),
        "tenant_b should not inherit tenant_a's drained bucket, got: {r4}"
    );

    let _ = child.kill();
    let _ = child.wait();
}
