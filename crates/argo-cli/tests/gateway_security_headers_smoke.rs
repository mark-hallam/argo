//! Smoke test: boots the gateway, hits `/health` (a known JSON route),
//! and asserts the always-on security headers ride along on the
//! response. The hand-rolled HTTP/1.1 listener has no HTML routes
//! today, so this exercises the JSON-path side of the brief
//! (always-on headers, no CSP). HTML+CSP are covered by the unit
//! tests under `security_headers_tests.rs`.
mod common;
use common::{body_of, boot, http_get, pick_free_port};

#[test]
fn gateway_json_responses_carry_always_on_security_headers() {
    let port = pick_free_port();
    // boot() internally sends one /sessions request to verify readiness;
    // the test sends one more (/health). max_requests=1 → the gateway
    // serves 2 (boot+1) and exits, so `child.wait()` returns cleanly.
    let (mut child, _stderr) = boot(port, 1);

    let resp = http_get(port, "/health");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    // Always-on headers — present on every response regardless of
    // request scheme or response Content-Type.
    assert!(
        resp.contains("X-Content-Type-Options: nosniff"),
        "missing nosniff: {resp}"
    );
    assert!(
        resp.contains("X-Frame-Options: DENY"),
        "missing X-Frame-Options: {resp}"
    );
    assert!(
        resp.contains("Referrer-Policy: strict-origin-when-cross-origin"),
        "missing Referrer-Policy: {resp}"
    );
    assert!(
        resp.contains("Permissions-Policy: "),
        "missing Permissions-Policy: {resp}"
    );
    // CSP must NOT ride along on JSON — browsers don't render
    // application/json as a document, so CSP would be wasted bytes
    // and confuse audit tooling.
    assert!(
        !resp.contains("Content-Security-Policy"),
        "CSP should not be on JSON responses: {resp}"
    );
    // HSTS must NOT appear without an HTTPS-derived request — the
    // raw http_get helper does not set X-Forwarded-Proto.
    assert!(
        !resp.contains("Strict-Transport-Security"),
        "HSTS should only appear on HTTPS-derived requests: {resp}"
    );
    // Body still parses cleanly — security headers must be additive,
    // never break the response shape that existing routes set.
    let body = body_of(&resp);
    let parsed: serde_json::Value =
        serde_json::from_str(body).expect("/health body should still be valid JSON");
    assert_eq!(parsed["status"], "ok", "body changed: {body}");

    let status = child.wait().expect("wait");
    assert!(status.success(), "exit: {status}");
}
