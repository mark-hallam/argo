//! Unit tests for the gateway security-headers module. Loaded via
//! `#[cfg(test)] #[path = "security_headers_tests.rs"] mod tests;`.
//!
//! Env vars are process-global. Tests that touch `ARGO_CSP_REPORT_ONLY`
//! serialize on `ENV_LOCK` and reset the var at start AND end so the
//! order-of-execution does not affect outcomes — same pattern as
//! `routes_auth_tests.rs`.

use super::{header_lines, request_is_https};
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());
const REPORT_ONLY_KEY: &str = "ARGO_CSP_REPORT_ONLY";

fn reset_env() {
    std::env::remove_var(REPORT_ONLY_KEY);
}

#[test]
fn html_response_gets_all_headers_including_csp() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    reset_env();
    let block = header_lines("text/html; charset=utf-8", false);
    assert!(block.contains("X-Content-Type-Options: nosniff"));
    assert!(block.contains("X-Frame-Options: DENY"));
    assert!(block.contains("Referrer-Policy: strict-origin-when-cross-origin"));
    assert!(block.contains("Permissions-Policy: geolocation=(), camera=(), microphone=(), payment=()"));
    assert!(block.contains("Content-Security-Policy: default-src 'self'"));
    assert!(block.contains("frame-ancestors 'none'"));
    reset_env();
}

#[test]
fn json_response_gets_all_headers_except_csp() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    reset_env();
    let block = header_lines("application/json", false);
    assert!(block.contains("X-Content-Type-Options: nosniff"));
    assert!(block.contains("X-Frame-Options: DENY"));
    assert!(block.contains("Referrer-Policy: strict-origin-when-cross-origin"));
    assert!(block.contains("Permissions-Policy: "));
    assert!(
        !block.contains("Content-Security-Policy"),
        "JSON response must NOT carry CSP: block={block}"
    );
    reset_env();
}

#[test]
fn https_request_adds_hsts() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    reset_env();
    let block = header_lines("application/json", true);
    assert!(block.contains("Strict-Transport-Security: max-age=31536000; includeSubDomains"));
    let req = "GET / HTTP/1.1\r\nHost: x\r\nX-Forwarded-Proto: https\r\n\r\n";
    assert!(request_is_https(req));
    // Case-insensitive on both header name and value.
    let req_mixed = "GET / HTTP/1.1\r\nx-forwarded-proto: HTTPS\r\n\r\n";
    assert!(request_is_https(req_mixed));
    reset_env();
}

#[test]
fn http_request_omits_hsts() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    reset_env();
    let block = header_lines("application/json", false);
    assert!(
        !block.contains("Strict-Transport-Security"),
        "HTTP-derived request must NOT carry HSTS"
    );
    // No header, or a non-https proto, must read as not-HTTPS.
    assert!(!request_is_https("GET / HTTP/1.1\r\nHost: x\r\n\r\n"));
    assert!(!request_is_https(
        "GET / HTTP/1.1\r\nX-Forwarded-Proto: http\r\n\r\n"
    ));
    reset_env();
}

#[test]
fn report_only_env_switches_header_name() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    reset_env();
    std::env::set_var(REPORT_ONLY_KEY, "1");
    let block = header_lines("text/html", false);
    assert!(
        block.contains("Content-Security-Policy-Report-Only: "),
        "expected report-only variant: block={block}"
    );
    assert!(
        !block.contains("\r\nContent-Security-Policy: "),
        "must not also emit the enforcing variant: block={block}"
    );
    reset_env();
    // Without the env var, the enforcing header name returns.
    let block_off = header_lines("text/html", false);
    assert!(block_off.contains("\r\nContent-Security-Policy: "));
    assert!(!block_off.contains("Content-Security-Policy-Report-Only"));
    reset_env();
}

#[test]
fn header_block_terminates_each_line_for_splicing() {
    // The block is spliced between an existing `Content-Length: N\r\n`
    // line and the body separator `\r\n` — so every header MUST end
    // with `\r\n`, otherwise a downstream parser will glue two headers
    // together and a handler-set header could be effectively "replaced".
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    reset_env();
    let block = header_lines("text/html", true);
    // Block must end with \r\n so `block + "\r\n"` produces the
    // canonical `\r\n\r\n` end-of-headers separator.
    assert!(block.ends_with("\r\n"), "block does not end \\r\\n: {block:?}");
    // Every non-empty line must end with \r — counted by splitting on
    // \r\n and confirming we get the same number of lines as headers.
    let line_count = block.matches("\r\n").count();
    let header_count = block
        .split("\r\n")
        .filter(|s| !s.is_empty() && s.contains(':'))
        .count();
    assert_eq!(line_count, header_count, "every header must end \\r\\n");
    // For HTML+HTTPS, expect: nosniff + frame + referrer + perms + HSTS + CSP = 6
    assert_eq!(header_count, 6, "block={block}");
    reset_env();
}
