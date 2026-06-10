//! `POST /api/v1/pitch/ask/public` — the ANONYMOUS, rate-limited public
//! front door to the Pitch Agent (Portia), backing the landing-page
//! "Talk to Portia" widget (`web/components/marketing/PortiaEmbed.tsx`).
//!
//! # Why a separate route
//!
//! The operator-facing `/api/v1/pitch/ask`
//! ([`super::routes_pitch_ask::ask_response`]) sits BEHIND
//! `preflight_dispatch`'s `require_identity` bearer gate, so a public web
//! visitor can never reach it. Before this route the embed showed
//! hand-written canned answers — the viral proof of the truth-boundary
//! moat was *fake*. This route is mounted in the SAME pre-bearer lane as
//! the first-run wizard (`super::routes_wizard`) so an unauthenticated
//! visitor reaches the real truth-bounded answerer, making the
//! landing-page proof REAL.
//!
//! # No parallel pitch engine
//!
//! This does NOT fork the pitch engine. It delegates to the same
//! [`super::routes_pitch_ask::ask_response`], pinning the channel to the
//! public web-widget variant ([`PUBLIC_CHANNEL`]) server-side so the
//! STRICT public [`argo_capability::TruthBoundaryPolicy`] always applies
//! — a malicious client cannot ask for the looser rehearsal policy. Only
//! a corpus-grounded answer or the deterministic `MISS_FALLBACK` line is
//! ever returned; the LLM-fallback path stays env-gated and auto-off
//! without a provider key, exactly as on the authenticated path.
//!
//! # Anonymous-safety (the load-bearing part)
//!
//! 1. **Per-IP token-bucket cap FIRST** — reuses the existing
//!    [`super::rate_limit_ip::IpRateLimiter`] type (no new bucket type),
//!    keyed on the visitor's source IP, mirroring the public-signup /
//!    waitlist / enterprise-inquiry routes. Over budget → `429`.
//! 2. **Tight input-length ceiling** ([`MAX_PUBLIC_QUESTION_LEN`]) before
//!    any corpus/LLM work.
//! 3. **Strict public truth-boundary policy** (forced channel, above).
//!
//! # Source IP is the trusted accept-loop value
//!
//! The per-IP limiter keys on `src_ip`, the single authoritative source
//! IP the accept loop extracts once via
//! `super::proxy_guard::extract_real_ip` (honoring `X-Forwarded-For` ONLY
//! from a trusted/loopback direct peer). It is threaded down through the
//! dispatch chain — exactly like `super::routes_enterprise_inquiry` and
//! `super::routes_waitlist`. The earlier design re-derived the IP from a
//! client-supplied `X-Forwarded-For` header in-handler, which let an
//! attacker rotate the header to defeat the cap
//! (fix-public-rate-limit-xff-spoof).

use super::rate_limit_ip::{IpRateLimiter, RateLimitIpCheckResult};
use argo_store::SessionStore;
use serde::Deserialize;
use serde_json::json;
use std::net::IpAddr;
use std::sync::{Arc, OnceLock};

/// Hard upper bound on a public question, in characters. Rejected with
/// `400` before any corpus lookup or LLM call. Keeps the anonymous
/// surface from being used as a bulk text-processing proxy.
pub(super) const MAX_PUBLIC_QUESTION_LEN: usize = 600;

/// Server-forced channel for every public request. Pins the strict
/// public-web truth-boundary policy regardless of what the client sends
/// (`PitchChannel::WebEmbed`, the anonymous-visitor variant).
const PUBLIC_CHANNEL: &str = "WebEmbed";

/// Per-IP bucket capacity for the public pitch route.
const PUBLIC_PITCH_BUCKET_CAPACITY: u32 = 10;
/// Refill: ~1 token every 5s ⇒ ~12 questions/minute sustained per IP,
/// with a burst of [`PUBLIC_PITCH_BUCKET_CAPACITY`]. Tight enough to cap
/// abuse + LLM cost, generous enough for a real visitor's curiosity.
const PUBLIC_PITCH_BUCKET_REFILL_PER_SEC: f64 = 0.2;

/// Wire shape the public widget POSTs. Intentionally a STRICT SUBSET of
/// the authenticated `PitchAskRequest` — the public route does NOT accept
/// `channel`/`audience` from the client (both are forced server-side), so
/// this is not a near-duplicate input struct.
#[derive(Debug, Deserialize)]
struct PublicPitchBody {
    question: String,
}

/// Process-global per-IP limiter dedicated to the public pitch route.
/// Lazily built on first use. `trust_xff = false`: the caller passes the
/// already-trusted `src_ip` (extracted once at the accept loop via
/// `proxy_guard::extract_real_ip`), so this limiter never re-parses
/// `X-Forwarded-For` itself (fix-public-rate-limit-xff-spoof).
pub(super) fn public_pitch_limiter() -> &'static Arc<IpRateLimiter> {
    static INSTANCE: OnceLock<Arc<IpRateLimiter>> = OnceLock::new();
    INSTANCE.get_or_init(|| {
        Arc::new(IpRateLimiter::new(
            PUBLIC_PITCH_BUCKET_CAPACITY,
            PUBLIC_PITCH_BUCKET_REFILL_PER_SEC,
            true,
            false,
        ))
    })
}

/// Handle `POST /api/v1/pitch/ask/public`. Anonymous-safe: per-IP rate
/// limit → input cap → channel-pinned delegation to the truth-bounded
/// answerer. Returns the canonical gateway `(status, body)` tuple.
pub(super) async fn ask_public_response(
    req: &str,
    src_ip: IpAddr,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    // 1. Per-IP rate limit FIRST — a cheap reject before any parsing or
    //    corpus/LLM work. Keyed on the trusted `src_ip` from the accept
    //    loop, never a spoofable `X-Forwarded-For`.
    if let RateLimitIpCheckResult::Deny { retry_after_secs } = public_pitch_limiter().check(src_ip) {
        return rate_limited(retry_after_secs);
    }

    // 2. Parse the minimal public body + enforce the input-length cap.
    let parsed: PublicPitchBody = match serde_json::from_str(super::body_from_request(req)) {
        Ok(b) => b,
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };
    let question = parsed.question.trim();
    if question.is_empty() {
        return bad_request("`question` is required");
    }
    if question.chars().count() > MAX_PUBLIC_QUESTION_LEN {
        return bad_request(&format!(
            "`question` exceeds the {MAX_PUBLIC_QUESTION_LEN}-character public limit"
        ));
    }

    // 3. Force the public channel server-side (never trust the client) and
    //    delegate to the SAME truth-bounded answerer. We rebuild the
    //    request body with the channel pinned while preserving the original
    //    headers, so `ask_response`'s audit tenant/principal resolution is
    //    byte-for-byte unchanged.
    let pinned = pin_public_channel(req, question);
    super::routes_pitch_ask::ask_response(&pinned, store).await
}

/// Rebuild `req` with a normalized, channel-pinned JSON body. Preserves
/// the original header block (audit resolution reads it) and only swaps
/// the body for `{question, channel: WebEmbed, audience: investor}`.
fn pin_public_channel(req: &str, question: &str) -> String {
    let headers = req.split_once("\r\n\r\n").map_or(req, |(h, _)| h);
    let body = json!({
        "question": question,
        "channel": PUBLIC_CHANNEL,
        "audience": "investor",
    })
    .to_string();
    format!("{headers}\r\n\r\n{body}")
}

/// `400` envelope matching the authenticated path's `bad_request` shape.
fn bad_request(msg: &str) -> (&'static str, String) {
    (
        "400 Bad Request",
        json!({ "error": { "code": "bad_request", "message": msg } }).to_string(),
    )
}

/// `429` envelope. Carries both `retry_after` and `retry_after_secs` so it
/// matches the per-tenant / per-IP limiter response shape already used
/// across the gateway.
fn rate_limited(retry_after_secs: u32) -> (&'static str, String) {
    (
        "429 Too Many Requests",
        json!({
            "error": "rate_limited",
            "scope": "pitch_public",
            "retry_after": retry_after_secs,
            "retry_after_secs": retry_after_secs,
        })
        .to_string(),
    )
}

#[cfg(test)]
#[path = "routes_pitch_ask_public_tests.rs"]
mod tests;
