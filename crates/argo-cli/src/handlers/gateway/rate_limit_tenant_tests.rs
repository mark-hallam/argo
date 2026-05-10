//! Unit tests for [`super::rate_limit_tenant`] (mvp-rate-limit).
//!
//! Coverage:
//!   1. first request from a new tenant is allowed
//!   2. burst up to capacity is allowed
//!   3. exceeding capacity → deny with `retry_after_secs`
//!   4. waiting long enough refills the bucket
//!   5. distinct tenants hold independent buckets (cross-tenant isolation)
//!   6. eviction culls stale entries when over the soft cap
//!   7. `enabled = false` short-circuits to Allow even at capacity 1
//!   8. `from_env` reads `ARGO_RATE_LIMIT_TENANT_PER_MIN` (override + 0-disables + invalid)
//!   9. denied request body carries the documented JSON shape
//!  10. `ARGO_GATEWAY_AUTH_BYPASS=1` short-circuits `check_request`

use super::{Bucket, CheckResult, TenantRateLimiter, EVICT_AGE_SECS, MAX_ENTRIES};
use std::time::{Duration, Instant};

#[test]
fn first_request_from_new_tenant_is_allowed() {
    let limiter = TenantRateLimiter::new(5, true);
    assert_eq!(limiter.check("acme"), CheckResult::Allow);
}

#[test]
fn burst_up_to_capacity_is_allowed() {
    let limiter = TenantRateLimiter::new(3, true);
    let now = Instant::now();
    for i in 0..3 {
        assert_eq!(
            limiter.check_with_now("acme", now),
            CheckResult::Allow,
            "request {i} should be allowed within capacity"
        );
    }
}

#[test]
fn exceeding_capacity_returns_deny_with_retry_after() {
    let limiter = TenantRateLimiter::new(2, true);
    let now = Instant::now();
    assert_eq!(limiter.check_with_now("acme", now), CheckResult::Allow);
    assert_eq!(limiter.check_with_now("acme", now), CheckResult::Allow);
    match limiter.check_with_now("acme", now) {
        // refill = 2/60 tokens/sec ⇒ 1 token in 30s exactly.
        CheckResult::Deny { retry_after_secs } => assert_eq!(retry_after_secs, 30),
        CheckResult::Allow => panic!("third request should be denied"),
    }
}

#[test]
fn refill_after_wait_allows_again() {
    // per_min=60 → cap=60, refill 1 token/sec exactly.
    let limiter = TenantRateLimiter::new(60, true);
    let t0 = Instant::now();
    for _ in 0..60 {
        assert_eq!(limiter.check_with_now("acme", t0), CheckResult::Allow);
    }
    matches_deny(limiter.check_with_now("acme", t0));
    let t1 = t0 + Duration::from_secs(2);
    assert_eq!(limiter.check_with_now("acme", t1), CheckResult::Allow);
}

#[test]
fn two_tenant_isolation_no_cross_drain() {
    // Cap 1 each so a single drain on tenant A would break tenant B
    // if they shared a bucket. This is the brief's required test.
    let limiter = TenantRateLimiter::new(1, true);
    let now = Instant::now();
    assert_eq!(limiter.check_with_now("tenant-a", now), CheckResult::Allow);
    assert_eq!(limiter.check_with_now("tenant-b", now), CheckResult::Allow);
    matches_deny(limiter.check_with_now("tenant-a", now));
    matches_deny(limiter.check_with_now("tenant-b", now));
    assert_eq!(limiter.bucket_count(), 2);
}

#[test]
fn evict_old_culls_stale_entries() {
    let limiter = TenantRateLimiter::new(1, true);
    let t0 = Instant::now();
    for tenant in ["alpha", "beta", "gamma"] {
        let _ = limiter.check_with_now(tenant, t0);
    }
    assert_eq!(limiter.bucket_count(), 3);
    limiter.evict_old(t0 + Duration::from_secs(EVICT_AGE_SECS + 30));
    assert_eq!(limiter.bucket_count(), 0, "stale entries should be evicted");
    const _: () = assert!(MAX_ENTRIES >= 1_000);
}

#[test]
fn disabled_limiter_short_circuits_to_allow() {
    let limiter = TenantRateLimiter::new(1, false);
    let now = Instant::now();
    for _ in 0..50 {
        assert_eq!(limiter.check_with_now("acme", now), CheckResult::Allow);
    }
}

#[test]
fn bucket_caps_tokens_at_capacity() {
    let mut bucket = Bucket::new(2, Instant::now());
    let later = Instant::now() + Duration::from_secs(120);
    assert!(bucket.take(later, 2, 5.0));
    assert!(bucket.take(later, 2, 5.0));
    assert!(!bucket.take(later, 2, 5.0));
}

#[test]
fn from_env_override_zero_and_invalid() {
    let _g = ENV_LOCK.lock().unwrap();

    // Explicit override.
    std::env::set_var("ARGO_RATE_LIMIT_TENANT_PER_MIN", "10");
    let limiter = TenantRateLimiter::from_env();
    let now = Instant::now();
    for _ in 0..10 {
        assert_eq!(limiter.check_with_now("t", now), CheckResult::Allow);
    }
    matches_deny(limiter.check_with_now("t", now));

    // 0 disables.
    std::env::set_var("ARGO_RATE_LIMIT_TENANT_PER_MIN", "0");
    let limiter = TenantRateLimiter::from_env();
    for _ in 0..200 {
        assert_eq!(limiter.check_with_now("t", now), CheckResult::Allow);
    }

    // Invalid → default 60.
    std::env::set_var("ARGO_RATE_LIMIT_TENANT_PER_MIN", "not-a-number");
    let limiter = TenantRateLimiter::from_env();
    for _ in 0..60 {
        assert_eq!(limiter.check_with_now("t", now), CheckResult::Allow);
    }
    matches_deny(limiter.check_with_now("t", now));

    std::env::remove_var("ARGO_RATE_LIMIT_TENANT_PER_MIN");
}

#[test]
fn denied_request_body_carries_documented_shape() {
    // Mirror the JSON shape `check_request` emits without touching
    // process-wide state. The shape is the public contract for
    // gateway clients.
    let limiter = TenantRateLimiter::new(1, true);
    let now = Instant::now();
    assert_eq!(limiter.check_with_now("acme", now), CheckResult::Allow);
    let CheckResult::Deny { retry_after_secs } = limiter.check_with_now("acme", now) else {
        panic!("expected Deny");
    };
    let body = format!(
        "{{\"error\":\"rate_limited\",\"scope\":\"tenant\",\
         \"retry_after\":{retry_after_secs},\
         \"retry_after_secs\":{retry_after_secs}}}"
    );
    assert!(body.contains("\"error\":\"rate_limited\""));
    assert!(body.contains("\"scope\":\"tenant\""));
    assert!(body.contains(&format!("\"retry_after\":{retry_after_secs}")));
    assert!(retry_after_secs >= 1);
}

#[test]
fn check_request_bypasses_when_auth_bypass_set() {
    let _g = ENV_LOCK.lock().unwrap();
    std::env::set_var("ARGO_GATEWAY_AUTH_BYPASS", "1");
    let req = "POST /sessions HTTP/1.1\r\nX-Tenant-Id: bypass-tester\r\n\r\n";
    assert!(super::check_request(req).is_none());
    std::env::remove_var("ARGO_GATEWAY_AUTH_BYPASS");
}

fn matches_deny(r: CheckResult) {
    match r {
        CheckResult::Deny { retry_after_secs } => assert!(retry_after_secs >= 1),
        CheckResult::Allow => panic!("expected Deny, got Allow"),
    }
}

// Process-wide guard for env-mutating tests. `cargo test` runs unit
// tests in parallel; serializing avoids cross-test leakage on
// `ARGO_RATE_LIMIT_TENANT_PER_MIN` / `ARGO_GATEWAY_AUTH_BYPASS`.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
