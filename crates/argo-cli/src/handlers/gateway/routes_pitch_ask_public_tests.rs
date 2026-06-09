//! Inline tests for the anonymous public pitch route
//! (`POST /api/v1/pitch/ask/public`).
//!
//! These exercise the four load-bearing guarantees of the public surface:
//! it reaches the truth-bounded answerer WITHOUT a bearer (acceptance 1),
//! the authenticated route's bearer gate is still intact (acceptance 2),
//! the per-IP cap returns `429` (acceptance 3), an oversized question is
//! rejected with `400` before any work (acceptance 4), and an off-corpus
//! question with the LLM path off returns the deterministic `MISS_FALLBACK`
//! line — never a fabricated answer (acceptance 5).
//!
//! Tests serialize `ARGO_GATEWAY_*` env mutation on `ENV_LOCK`. Each test
//! uses a DISTINCT `X-Forwarded-For` IP so they never drain each other's
//! shared process-global rate-limit bucket.

use super::*;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Build a public-route request from a given source IP + body.
fn public_req(ip: &str, body: &serde_json::Value) -> String {
    format!(
        "POST /api/v1/pitch/ask/public HTTP/1.1\r\nHost: localhost\r\n\
         Content-Type: application/json\r\nX-Forwarded-For: {ip}\r\n\r\n{body}"
    )
}

#[tokio::test]
async fn anonymous_in_corpus_question_returns_grounded_answer_not_401() {
    // No Authorization header — proves the public route bypasses the
    // bearer gate and reaches the truth-bounded answerer.
    let body = serde_json::json!({ "question": "What's the tech stack?" });
    let req = public_req("203.0.113.1", &body);
    let (status, body) = ask_public_response(&req, None).await;
    assert_eq!(status, "200 OK", "status={status} body={body}");
    assert_ne!(status, "401 Unauthorized");
    assert_ne!(status, "403 Forbidden");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("body is JSON");
    assert_eq!(parsed["matched"], true, "expected a canonical hit: {body}");
    assert_eq!(parsed["source"], "canonical");
    // The strict public channel was forced server-side.
    assert_eq!(parsed["channel"], "WebEmbed");
}

// Acceptance 2: the authenticated `/api/v1/pitch/ask` route is unchanged —
// the bearer gate (`require_identity` → `check_bearer_auth`) still rejects an
// un-authenticated request with 401. The public route is purely ADDITIVE: it
// reaches the handler WITHOUT a bearer, proven by
// `anonymous_in_corpus_question_returns_grounded_answer_not_401` above
// (status 200, never 401/403).
//
// Synchronous (`#[test]`) on purpose: it holds the `std::sync::Mutex` env lock
// only across non-async config reads, never across an `.await` — so it does
// not trip `clippy::await_holding_lock`.
#[test]
fn authenticated_pitch_route_still_requires_a_bearer() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    // Perimeter closed: a bearer token IS configured + bypass off, so a
    // request with no Authorization header is rejected.
    std::env::remove_var("ARGO_GATEWAY_AUTH_BYPASS");
    std::env::set_var("ARGO_GATEWAY_BEARER_TOKEN", "test-secret-public-pitch");
    let auth_req = "POST /api/v1/pitch/ask HTTP/1.1\r\nHost: localhost\r\n\
         Content-Type: application/json\r\n\r\n{}";
    let resp = super::super::routes_auth::require_identity(auth_req);
    assert_eq!(
        resp.map(|(s, _)| s),
        Some("401 Unauthorized"),
        "authenticated /pitch/ask without a bearer must 401"
    );
    std::env::remove_var("ARGO_GATEWAY_BEARER_TOKEN");
}

#[tokio::test]
async fn over_the_per_ip_rate_limit_returns_429() {
    let ip = "198.51.100.7";
    let parsed_ip = ip.parse().expect("valid ip");
    // Drain this IP's bucket via the shared limiter directly (capacity + a
    // margin) so the subsequent handler call is guaranteed over budget.
    for _ in 0..(PUBLIC_PITCH_BUCKET_CAPACITY + 2) {
        let _ = public_pitch_limiter().check(parsed_ip);
    }
    let body = serde_json::json!({ "question": "What's the tech stack?" });
    let req = public_req(ip, &body);
    let (status, body) = ask_public_response(&req, None).await;
    assert_eq!(status, "429 Too Many Requests", "status={status} body={body}");
    assert!(body.contains("rate_limited"), "expected rate_limited body, got {body}");
    assert!(body.contains("retry_after"), "expected retry_after in body, got {body}");
}

#[tokio::test]
async fn oversized_question_is_rejected_with_400() {
    // > MAX_PUBLIC_QUESTION_LEN characters → 400 before any corpus/LLM work.
    let huge = "a".repeat(MAX_PUBLIC_QUESTION_LEN + 1);
    let body = serde_json::json!({ "question": huge });
    let req = public_req("198.51.100.9", &body);
    let (status, body) = ask_public_response(&req, None).await;
    assert_eq!(status, "400 Bad Request", "status={status} body={body}");
    assert!(body.contains("bad_request"), "expected bad_request envelope, got {body}");
}

#[tokio::test]
async fn empty_question_is_rejected_with_400() {
    let body = serde_json::json!({ "question": "   " });
    let req = public_req("198.51.100.11", &body);
    let (status, body) = ask_public_response(&req, None).await;
    assert_eq!(status, "400 Bad Request", "status={status} body={body}");
}

#[tokio::test]
async fn off_corpus_question_with_llm_off_returns_miss_fallback_not_a_fabrication() {
    // Defense-in-depth: explicitly disable the LLM-fallback path. The path
    // is ALSO auto-disabled with no provider key present (the worker
    // environment never sets ANTHROPIC_API_KEY), so the deterministic miss
    // is robust either way.
    std::env::set_var(super::super::routes_pitch_ask::LLM_FALLBACK_ENV, "0");
    let body = serde_json::json!({
        "question": "Can you recite a haiku about arctic terns?",
    });
    let req = public_req("198.51.100.13", &body);
    let (status, body) = ask_public_response(&req, None).await;
    assert_eq!(status, "200 OK", "status={status} body={body}");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("body is JSON");
    assert_eq!(parsed["matched"], false);
    assert_eq!(parsed["source"], "canonical");
    assert_eq!(
        parsed["answer"],
        serde_json::Value::String(super::super::routes_pitch_ask::MISS_FALLBACK.to_string()),
        "off-corpus miss must return the deterministic fallback, never a fabricated claim"
    );
    assert!(
        parsed.get("model").is_none(),
        "no LLM model field on the deterministic miss path"
    );
}
