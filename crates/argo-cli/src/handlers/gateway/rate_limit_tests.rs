//! Unit tests for [`super::rate_limit`]
//! (claude-rate-limit-per-route).
//!
//! Six unit tests + one integration test covering:
//!   1. bucket fills + drains (capacity = rpm; drain to empty)
//!   2. refill rate is correct (`rpm/60.0` tokens/sec)
//!   3. eviction removes idle buckets after [`EVICT_AGE`]
//!   4. env override wires through `ARGO_RATE_LIMIT_<ROUTE>_RPM`
//!   5. unknown route falls back to default
//!   6. concurrent acquires are thread-safe (DashMap entry semantics)
//!   7. integration: 11 requests to a 10/min route → exactly 1 reject

use super::{
    classify_request, classify_route, env_var_for_route, extract_tenant, RateLimiter, RetryAfter,
    EVICT_AGE, FORGE_RUN_RPM, ROUTE_FORGE_RUN, ROUTE_SKILL_EXECUTE,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;

fn limiter(default_rpm: u32) -> RateLimiter {
    RateLimiter::new(default_rpm, HashMap::new())
}

#[test]
fn bucket_fills_to_capacity_then_drains() {
    let l = limiter(5);
    let now = Instant::now();
    // Cold-start bucket holds full rpm tokens (5).
    for i in 0..5 {
        assert!(
            l.acquire_at("r/test", "tenant-a", now).is_ok(),
            "request {i} within capacity should pass"
        );
    }
    // Sixth at the same instant — bucket empty, deny.
    let err = l
        .acquire_at("r/test", "tenant-a", now)
        .expect_err("6th call must reject");
    assert!(err.retry_after_ms >= 1, "retry_after_ms must be >= 1");
}

#[test]
fn refill_rate_matches_rpm_over_60s() {
    // 60 rpm = exactly 1 token/sec. After 1s, the empty bucket should
    // hold 1 token; after 2s, 2 tokens.
    let l = limiter(60);
    let t0 = Instant::now();
    for _ in 0..60 {
        assert!(l.acquire_at("r/refill", "t", t0).is_ok());
    }
    let RetryAfter { retry_after_ms } = l
        .acquire_at("r/refill", "t", t0)
        .expect_err("bucket drained");
    // 1 token / (60/60.0 tokens/sec) = 1.0s = 1000ms.
    assert_eq!(retry_after_ms, 1000, "wait for one token at 1 tok/sec");

    // Wait 2s — should fund 2 successful acquires.
    let t1 = t0 + Duration::from_secs(2);
    assert!(l.acquire_at("r/refill", "t", t1).is_ok());
    assert!(l.acquire_at("r/refill", "t", t1).is_ok());
    assert!(
        l.acquire_at("r/refill", "t", t1).is_err(),
        "third should fail — only 2s of refill banked"
    );
}

#[test]
fn evict_idle_drops_stale_buckets() {
    let l = limiter(2);
    let t0 = Instant::now();
    let _ = l.acquire_at("r/a", "tenant-a", t0);
    let _ = l.acquire_at("r/b", "tenant-b", t0);
    let _ = l.acquire_at("r/c", "tenant-c", t0);
    assert_eq!(l.bucket_count(), 3);
    // Within the eviction window — keep all.
    l.evict_idle(t0 + EVICT_AGE - Duration::from_secs(1));
    assert_eq!(l.bucket_count(), 3, "idle but still inside window");
    // Past the window — drop everything.
    l.evict_idle(t0 + EVICT_AGE + Duration::from_secs(30));
    assert_eq!(l.bucket_count(), 0, "stale entries should be evicted");
}

#[test]
fn env_override_for_known_route_is_honoured() {
    // `from_env` reads `ARGO_RATE_LIMIT_FORGE_RUN_RPM` for the
    // `forge.run` alias. Use `set_var` for the duration of this test;
    // tests share a process so a unique sentinel value is safe.
    std::env::set_var("ARGO_RATE_LIMIT_FORGE_RUN_RPM", "7");
    let l = RateLimiter::from_env();
    assert_eq!(l.rpm_for(ROUTE_FORGE_RUN), 7, "env override for forge.run");
    assert_eq!(
        l.rpm_for(ROUTE_SKILL_EXECUTE),
        30,
        "skill.execute keeps documented default"
    );
    std::env::remove_var("ARGO_RATE_LIMIT_FORGE_RUN_RPM");

    // env_var_for_route maps as documented.
    assert_eq!(env_var_for_route("forge.run"), "ARGO_RATE_LIMIT_FORGE_RUN_RPM");
    assert_eq!(
        env_var_for_route("skill.execute"),
        "ARGO_RATE_LIMIT_SKILL_EXECUTE_RPM"
    );
    assert_eq!(
        env_var_for_route("POST /sessions"),
        "ARGO_RATE_LIMIT_POST_SESSIONS_RPM"
    );
}

#[test]
fn unknown_route_falls_back_to_default_rpm() {
    let mut overrides = HashMap::new();
    overrides.insert(ROUTE_FORGE_RUN.to_string(), 10);
    let l = RateLimiter::new(100, overrides);
    assert_eq!(l.rpm_for("totally-unknown-route"), 100);
    assert_eq!(l.rpm_for("GET /skills/foo"), 100);
    assert_eq!(l.rpm_for(ROUTE_FORGE_RUN), 10);
    let now = Instant::now();
    // 100 rpm default → 100 successful acquires before drain.
    for i in 0..100 {
        assert!(
            l.acquire_at("GET /unknown", "t", now).is_ok(),
            "request {i} within default capacity"
        );
    }
    assert!(l.acquire_at("GET /unknown", "t", now).is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_acquires_are_thread_safe() {
    // 100 concurrent acquires against a bucket sized for 50: exactly 50
    // succeed, 50 are rejected, no panics, no double-spends.
    let l = Arc::new(limiter(50));
    let now = Instant::now();
    let mut tasks = Vec::new();
    for _ in 0..100 {
        let l = Arc::clone(&l);
        tasks.push(tokio::spawn(async move {
            l.acquire_at("r/race", "tenant-x", now).is_ok()
        }));
    }
    let mut allowed = 0usize;
    for h in tasks {
        if h.await.unwrap() {
            allowed += 1;
        }
    }
    assert_eq!(
        allowed, 50,
        "exactly capacity-many should succeed under contention"
    );
}

#[test]
fn integration_eleven_requests_to_ten_per_min_route_rejects_one() {
    // Brief acceptance: issue 11 requests back-to-back to a 10/min
    // route, observe exactly 1 reject. Use `forge.run` so the test
    // exercises the alias path end-to-end.
    let mut overrides = HashMap::new();
    overrides.insert(ROUTE_FORGE_RUN.to_string(), FORGE_RUN_RPM);
    let l = RateLimiter::new(100, overrides);
    let now = Instant::now();
    let mut allowed = 0;
    let mut rejected = 0;
    for _ in 0..11 {
        match l.acquire_at(ROUTE_FORGE_RUN, "tenant-z", now) {
            Ok(()) => allowed += 1,
            Err(_) => rejected += 1,
        }
    }
    assert_eq!(allowed, 10, "10/min capacity = 10 allowed");
    assert_eq!(rejected, 1, "exactly one over-limit reject");
}

// ---- ancillary: route classification + tenant extraction ----

#[test]
fn classify_runs_post_as_forge_run_alias() {
    let key = classify_route(
        "POST",
        "/sessions/3df8d50c-ddc1-4a3a-9f0d-1e2c5a2b9c10/runs",
    );
    assert_eq!(key.as_ref(), ROUTE_FORGE_RUN);
}

#[test]
fn classify_skill_refine_post_as_skill_execute_alias() {
    let key = classify_route("POST", "/skills/my-skill/refine");
    assert_eq!(key.as_ref(), ROUTE_SKILL_EXECUTE);
}

#[test]
fn classify_other_route_uses_method_path_template_with_uuid_masked() {
    let key = classify_route(
        "GET",
        "/sessions/3df8d50c-ddc1-4a3a-9f0d-1e2c5a2b9c10/messages?limit=10",
    );
    assert_eq!(key.as_ref(), "GET /sessions/:id/messages");
}

#[test]
fn classify_request_parses_request_line() {
    let req = "POST /sessions/3df8d50c-ddc1-4a3a-9f0d-1e2c5a2b9c10/runs HTTP/1.1\r\n\r\n";
    assert_eq!(classify_request(req).as_ref(), ROUTE_FORGE_RUN);
}

#[test]
fn extract_tenant_prefers_query_string_then_header_then_default() {
    let req_qs = "GET /sessions?tenant=tx HTTP/1.1\r\nHost: x\r\n\r\n";
    assert_eq!(extract_tenant(req_qs), "tx");

    let req_hdr = "GET /sessions HTTP/1.1\r\nX-Tenant-Id: tH\r\n\r\n";
    assert_eq!(extract_tenant(req_hdr), "tH");

    // No query/header → "anonymous" (env unset in test env).
    let req_none = "GET / HTTP/1.1\r\n\r\n";
    let extracted = extract_tenant(req_none);
    assert!(
        extracted == "anonymous"
            || extracted == std::env::var("ARGO_DEFAULT_TENANT").unwrap_or_default()
            || extracted == std::env::var("ARGO_WEBINBOX_TENANT_ID").unwrap_or_default(),
        "fallback path returned {extracted:?}"
    );
}

#[test]
fn separate_tenants_have_separate_buckets_for_same_route() {
    let l = limiter(2);
    let now = Instant::now();
    assert!(l.acquire_at("r/shared", "tenant-1", now).is_ok());
    assert!(l.acquire_at("r/shared", "tenant-1", now).is_ok());
    assert!(
        l.acquire_at("r/shared", "tenant-1", now).is_err(),
        "tenant-1 drained"
    );
    // tenant-2 is independent — full bucket still available.
    assert!(l.acquire_at("r/shared", "tenant-2", now).is_ok());
    assert!(l.acquire_at("r/shared", "tenant-2", now).is_ok());
    assert!(
        l.acquire_at("r/shared", "tenant-2", now).is_err(),
        "tenant-2 drained"
    );
}
