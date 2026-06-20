//! Pure-function unit tests for `routes_genesis_stream` (no TCP harness).
//!
//! Split from `routes_genesis_stream_tests.rs` to keep that file under the
//! per-file LOC ceiling.

use super::*;

#[test]
fn is_streaming_request_matches_post_only() {
    assert!(is_genesis_streaming_request(
        "POST /api/v1/genesis HTTP/1.1\r\n\r\n"
    ));
    assert!(is_genesis_streaming_request(
        "POST /api/v1/genesis?dry=1 HTTP/1.1\r\n\r\n"
    ));
    // The legacy sub-routes (`classify`, `draft-worker`) keep their
    // own handlers and must not be swallowed by the streaming check.
    assert!(!is_genesis_streaming_request(
        "POST /api/v1/genesis/classify HTTP/1.1\r\n\r\n"
    ));
    assert!(!is_genesis_streaming_request(
        "POST /api/v1/genesis/draft-worker HTTP/1.1\r\n\r\n"
    ));
    assert!(!is_genesis_streaming_request(
        "GET /api/v1/genesis HTTP/1.1\r\n\r\n"
    ));
}

#[test]
fn sanitize_tenant_normalises_special_chars() {
    assert_eq!(sanitize_tenant("AAAA-BBBB"), "aaaa-bbbb");
    assert_eq!(sanitize_tenant("..::weird::.."), "weird");
    assert_eq!(sanitize_tenant(""), "anonymous");
    assert_eq!(sanitize_tenant("///"), "anonymous");
}

#[test]
fn derive_persona_falls_back_to_operator_on_noise() {
    assert_eq!(derive_persona("!!! ??? ###"), "operator");
    assert_eq!(derive_persona("a b c"), "operator"); // tokens too short
    let derived = derive_persona("Marketing intern needed asap");
    assert_eq!(derived, "Marketing");
}

#[test]
fn genesis_stream_limiter_denies_after_capacity_burst() {
    // Anonymous Genesis SSE wallet/cost-DoS guard: a fresh limiter built with
    // the genesis constants must allow exactly the burst capacity then deny,
    // BEFORE any LLM authoring work runs. Build a fresh limiter (not the
    // process-global one) so the test is isolated from other suites.
    let limiter = IpRateLimiter::new(
        GENESIS_STREAM_BUCKET_CAPACITY,
        GENESIS_STREAM_BUCKET_REFILL_PER_SEC,
        true,
        false,
    );
    let ip: IpAddr = std::net::Ipv4Addr::new(203, 0, 113, 7).into();
    for i in 0..GENESIS_STREAM_BUCKET_CAPACITY {
        assert!(
            matches!(limiter.check(ip), RateLimitIpCheckResult::Allow),
            "request {i} within burst capacity should be allowed"
        );
    }
    match limiter.check(ip) {
        RateLimitIpCheckResult::Deny { retry_after_secs } => {
            assert!(retry_after_secs >= 1, "deny must carry a Retry-After hint");
        }
        RateLimitIpCheckResult::Allow => {
            panic!("the request past burst capacity must be denied (cost guardrail)")
        }
    }
}

#[test]
fn persona_for_honours_the_persona_slot_in_the_role_text() {
    // Day-1 fix 4 live repro: the SPA posted no `persona` body field but
    // the composed role text carried the intake's `Persona: Ava` slot.
    // The slug must be "ava" — not the charter's first word "read".
    let role = "Read invoice files from the inbox and draft payment \
                reminders.\n\nPersona: Ava\nCadence: every weekday morning";
    assert_eq!(persona_for(None, role), "ava");
    // An explicit body field still wins over the embedded slot.
    assert_eq!(persona_for(Some("Maya"), role), "maya");
    // No slot anywhere → the historical first-word fallback (slugged).
    assert_eq!(
        persona_for(None, "Chase overdue invoices politely."),
        "chase"
    );
}
