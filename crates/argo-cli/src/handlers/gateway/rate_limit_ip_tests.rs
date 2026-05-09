//! Unit tests for [`super::rate_limit_ip`]
//! (claude-security-rate-limit-by-ip).
//!
//! Seven tests covering:
//!   1. first request from a new IP is allowed
//!   2. burst up to capacity is allowed
//!   3. exceeding capacity → deny with `Retry-After`
//!   4. waiting long enough refills the bucket
//!   5. distinct IPs hold independent buckets
//!   6. eviction culls stale entries when the map is over cap
//!   7. `enabled = false` short-circuits to Allow even at capacity 0

use super::{
    extract_ip, parse_first_xff_hop, rate_limited_response_bytes, Bucket, CheckResult,
    IpRateLimiter, EVICT_AGE_SECS, MAX_ENTRIES,
};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::{Duration, Instant};

fn ip(a: u8, b: u8, c: u8, d: u8) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(a, b, c, d))
}

#[test]
fn first_request_from_new_ip_is_allowed() {
    let limiter = IpRateLimiter::new(5, 1.0, true, false);
    assert_eq!(limiter.check(ip(10, 0, 0, 1)), CheckResult::Allow);
}

#[test]
fn burst_up_to_capacity_is_allowed() {
    let limiter = IpRateLimiter::new(3, 0.001, true, false);
    let now = Instant::now();
    let src = ip(10, 0, 0, 2);
    for i in 0..3 {
        assert_eq!(
            limiter.check_with_now(src, now),
            CheckResult::Allow,
            "request {i} should be allowed within capacity"
        );
    }
}

#[test]
fn exceeding_capacity_returns_deny_with_retry_after() {
    let limiter = IpRateLimiter::new(2, 0.5, true, false);
    let now = Instant::now();
    let src = ip(10, 0, 0, 3);
    assert_eq!(limiter.check_with_now(src, now), CheckResult::Allow);
    assert_eq!(limiter.check_with_now(src, now), CheckResult::Allow);
    match limiter.check_with_now(src, now) {
        CheckResult::Deny { retry_after_secs } => {
            // 1 / 0.5 = 2s exactly.
            assert_eq!(retry_after_secs, 2, "retry_after should round up to 2s");
        }
        CheckResult::Allow => panic!("third request should be denied"),
    }

    // Body shape sanity-check.
    let bytes = rate_limited_response_bytes(2);
    let resp = std::str::from_utf8(&bytes).unwrap();
    assert!(resp.starts_with("HTTP/1.1 429 Too Many Requests\r\n"));
    assert!(resp.contains("Retry-After: 2\r\n"));
    assert!(resp.contains("\"scope\":\"ip\""));
    assert!(resp.contains("\"retry_after\":2"));
}

#[test]
fn refill_after_wait_allows_again() {
    let limiter = IpRateLimiter::new(1, 1.0, true, false);
    let t0 = Instant::now();
    let src = ip(10, 0, 0, 4);
    assert_eq!(limiter.check_with_now(src, t0), CheckResult::Allow);
    // Bucket empty.
    matches_deny(limiter.check_with_now(src, t0));
    // Two seconds later — at 1 token/sec, we should be back over the
    // 1-token threshold.
    let t1 = t0 + Duration::from_secs(2);
    assert_eq!(limiter.check_with_now(src, t1), CheckResult::Allow);
}

#[test]
fn different_ips_have_separate_buckets() {
    let limiter = IpRateLimiter::new(1, 0.001, true, false);
    let now = Instant::now();
    let a = ip(10, 0, 0, 5);
    let b = ip(10, 0, 0, 6);
    // First request on each IP succeeds despite capacity 1.
    assert_eq!(limiter.check_with_now(a, now), CheckResult::Allow);
    assert_eq!(limiter.check_with_now(b, now), CheckResult::Allow);
    // Second request on either IP is denied (bucket empty).
    matches_deny(limiter.check_with_now(a, now));
    matches_deny(limiter.check_with_now(b, now));
}

#[test]
fn evict_old_culls_stale_entries() {
    let limiter = IpRateLimiter::new(1, 0.001, true, false);
    let t0 = Instant::now();
    // Touch three IPs at t0.
    let _ = limiter.check_with_now(ip(10, 0, 0, 7), t0);
    let _ = limiter.check_with_now(ip(10, 0, 0, 8), t0);
    let _ = limiter.check_with_now(ip(10, 0, 0, 9), t0);
    assert_eq!(limiter.bucket_count(), 3);
    // Jump well past EVICT_AGE_SECS and trigger an eviction sweep.
    let t1 = t0 + Duration::from_secs(EVICT_AGE_SECS + 30);
    limiter.evict_old(t1);
    assert_eq!(
        limiter.bucket_count(),
        0,
        "stale entries should be evicted"
    );
    // MAX_ENTRIES is the soft cap that triggers eviction in `check`.
    // Sanity: it's a positive integer the production path actually uses.
    const _: () = assert!(MAX_ENTRIES >= 1_000);
}

#[test]
fn disabled_limiter_short_circuits_to_allow() {
    // Capacity 0 + disabled would normally always deny; disabled wins.
    let limiter = IpRateLimiter::new(1, 1.0, false, false);
    let now = Instant::now();
    let src = ip(10, 0, 0, 10);
    for _ in 0..50 {
        assert_eq!(limiter.check_with_now(src, now), CheckResult::Allow);
    }
}

// ---- ancillary tests for IP extraction + bucket math ----

#[test]
fn extract_ip_uses_peer_when_xff_disabled() {
    let req = "GET / HTTP/1.1\r\nX-Forwarded-For: 1.2.3.4\r\n\r\n";
    let peer = SocketAddr::new(ip(127, 0, 0, 1), 9000);
    assert_eq!(extract_ip(req, peer, false), ip(127, 0, 0, 1));
}

#[test]
fn extract_ip_honours_xff_first_hop_when_trusted() {
    let req = "GET / HTTP/1.1\r\nX-Forwarded-For: 1.2.3.4, 5.6.7.8\r\nHost: x\r\n\r\n";
    let peer = SocketAddr::new(ip(127, 0, 0, 1), 9000);
    assert_eq!(extract_ip(req, peer, true), ip(1, 2, 3, 4));
}

#[test]
fn extract_ip_falls_back_to_peer_on_invalid_xff() {
    let req = "GET / HTTP/1.1\r\nX-Forwarded-For: nonsense\r\n\r\n";
    let peer = SocketAddr::new(ip(127, 0, 0, 1), 9000);
    assert_eq!(extract_ip(req, peer, true), ip(127, 0, 0, 1));
}

#[test]
fn parse_first_xff_hop_is_case_insensitive_and_handles_ipv6() {
    let req = "GET / HTTP/1.1\r\nx-forwarded-for: ::1, 5.6.7.8\r\n\r\n";
    assert_eq!(parse_first_xff_hop(req), Some(IpAddr::V6(Ipv6Addr::LOCALHOST)));
}

#[test]
fn bucket_caps_tokens_at_capacity() {
    let mut bucket = Bucket::new(2, Instant::now());
    let later = Instant::now() + Duration::from_secs(60);
    // Even after a long idle, take() shouldn't accumulate above cap.
    assert!(bucket.take(later, 2, 5.0));
    assert!(bucket.take(later, 2, 5.0));
    // 3rd take in same instant must be denied — bucket is empty even
    // though the refill credit was generous.
    assert!(!bucket.take(later, 2, 5.0));
}

fn matches_deny(r: CheckResult) {
    match r {
        CheckResult::Deny { retry_after_secs } => assert!(retry_after_secs >= 1),
        CheckResult::Allow => panic!("expected Deny, got Allow"),
    }
}
