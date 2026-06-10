//! Verify-route XFF-spoof throttle tests (fix-verify-route-xff-spoof-reland).
//!
//! Lives in its own file (rather than the shared
//! `auth_magic_link_tests.rs`) so that test module stays under the
//! 500-LOC per-file ceiling. Asserts that the verify route's 60/IP/min
//! token-brute-force throttle keys on the trusted `src_ip` the accept
//! loop extracts (never a client-supplied `X-Forwarded-For`), so a
//! rotating forged header cannot mint a fresh bucket per request, and
//! that two distinct trusted peers get independent buckets.

use super::verify_outcome;
use super::VERIFY_RATE_LIMIT_PER_MIN;
use argo_store::SessionStore;
use std::sync::Arc;

async fn fresh_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:")
        .await
        .expect("in-memory store");
    // Migrate so the substrate user_tenant + audit_record tables exist
    // when verify_outcome's emit-audit hooks fire from a verify path.
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn req(method_path: &str, ip: Option<&str>, body: &str) -> String {
    let mut headers = String::new();
    if let Some(ip) = ip {
        headers.push_str(&format!("X-Forwarded-For: {ip}\r\n"));
    }
    format!(
        "{method_path} HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n{headers}\r\n{body}",
        body.len()
    )
}

// fix-verify-route-xff-spoof-reland — the verify route's 60/IP/min
// token-brute-force throttle keys on the trusted `src_ip` the accept loop
// extracts, never on a client-supplied `X-Forwarded-For`. An attacker who
// rotates the forged header on every request must NOT mint a fresh verify
// bucket per request. Same socket peer (one trusted src_ip), a different
// forged XFF on each call → the shared bucket fills and the 61st throttles.
#[tokio::test]
async fn verify_throttle_keys_on_trusted_src_ip_not_spoofable_xff() {
    let store = fresh_store().await;
    // Process-unique trusted peer so the static bucket map doesn't collide
    // with other tests' counters.
    let trusted: std::net::IpAddr = std::net::Ipv4Addr::new(198, 51, 100, 7).into();
    for i in 0..VERIFY_RATE_LIMIT_PER_MIN {
        // A different forged X-Forwarded-For on every single request.
        let forged = format!("203.0.113.{}", (i % 200) + 1);
        let r = req(
            "POST /api/v1/auth/verify",
            Some(&forged),
            r#"{"token":"never-issued"}"#,
        );
        let outcome = verify_outcome(&r, trusted, Some(&store)).await;
        assert!(
            outcome.status.starts_with("401"),
            "attempt {i} must pass the throttle (invalid token), got {} {}",
            outcome.status,
            outcome.body
        );
    }
    // 61st request, yet another forged XFF. Under the old spoofable helper
    // this minted a fresh per-XFF bucket (would be 401); keyed on the
    // trusted src_ip it is throttled.
    let r = req(
        "POST /api/v1/auth/verify",
        Some("203.0.113.250"),
        r#"{"token":"never-issued"}"#,
    );
    let outcome = verify_outcome(&r, trusted, Some(&store)).await;
    assert!(
        outcome.status.starts_with("429"),
        "rotating XFF must not defeat the trusted-src_ip throttle, got {} {}",
        outcome.status,
        outcome.body
    );
}

// fix-verify-route-xff-spoof-reland — two DISTINCT trusted src_ips get
// independent verify buckets: exhausting one peer's 60/min budget must not
// throttle a different peer.
#[tokio::test]
async fn verify_distinct_trusted_src_ips_get_separate_buckets() {
    let store = fresh_store().await;
    let ip_a: std::net::IpAddr = std::net::Ipv4Addr::new(198, 51, 100, 21).into();
    let ip_b: std::net::IpAddr = std::net::Ipv4Addr::new(198, 51, 100, 22).into();
    // Burn A's full budget (no XFF on the request — the throttle ignores it).
    for _ in 0..VERIFY_RATE_LIMIT_PER_MIN {
        let r = req("POST /api/v1/auth/verify", None, r#"{"token":"x"}"#);
        let _ = verify_outcome(&r, ip_a, Some(&store)).await;
    }
    let a_over = verify_outcome(
        &req("POST /api/v1/auth/verify", None, r#"{"token":"x"}"#),
        ip_a,
        Some(&store),
    )
    .await;
    assert!(
        a_over.status.starts_with("429"),
        "A's bucket must be exhausted, got {}",
        a_over.status
    );
    // B is a different trusted peer — its bucket is independent.
    let b_first = verify_outcome(
        &req("POST /api/v1/auth/verify", None, r#"{"token":"x"}"#),
        ip_b,
        Some(&store),
    )
    .await;
    assert!(
        b_first.status.starts_with("401"),
        "B's independent bucket must still allow, got {}",
        b_first.status
    );
}
