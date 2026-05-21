//! HS256 cookie-JWT verify gate
//! (`mission-revocation-with-verify-middleware-v1x`).
//!
//! This middleware closes the revocation enforcement half of the
//! 2026-05-17 HOLD review: shipping the `revoked_tokens` table +
//! `POST /api/v1/auth/logout` WITHOUT a request-path consultation of
//! the table is the "silent regression" the HOLD called out. This
//! file is that consultation.
//!
//! Wired by `routes::dispatch_route_inner` ahead of the bearer-token
//! gate. For every request that presents a `__Host-session` cookie
//! the middleware:
//!
//!   1. Extracts the cookie value from the `Cookie:` header,
//!   2. Decodes the HS256 JWT with the same secret
//!      `auth_magic_link_token::mint_jwt` signs with
//!      (env `SKYCOT_AUTH_JWT_SECRET`),
//!   3. Validates the `exp` claim against current wall-clock,
//!   4. Looks up the `jti` claim in `revoked_tokens` (when present),
//!   5. Returns either [`SessionVerifyOutcome::Valid`] (caller
//!      receives parsed [`VerifiedClaims`]) or
//!      [`SessionVerifyOutcome::Reject`] (caller short-circuits 401).
//!
//! Cookies WITHOUT a `jti` claim — i.e. issued by code paths that
//! pre-date the sibling cookie+CSRF+Origin+jti brief — pass the
//! revocation check unconditionally. The sibling brief (PR #1866)
//! added `jti` emission to `auth_magic_link_token::mint_jwt`; every
//! fresh session JWT minted on main is revocable. Cookies in the
//! wild before that point cannot be revoked but also cannot be
//! impersonated to gain new privileges, so the v1 trade-off is
//! acceptable; the cohort drains naturally on the 7-day `exp`
//! window.
//!
//! Single-secret v1: there is no key rotation. Rotation lands in a
//! follow-up brief (`mission-jwt-rotation-v1x`) that introduces a
//! key-id (`kid`) header field + a small verify-set.

use argo_store::SessionStore;
use jsonwebtoken::{DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// `__Host-session` cookie name — `__Host-` prefix discipline matches
/// `auth_magic_link_verify::SESSION_COOKIE_NAME` (kept duplicated
/// rather than reaching across module boundaries because that file is
/// outside this brief's may-touch list; both constants are validated
/// against `__Host-` semantics in the tests below).
const SESSION_COOKIE_NAME: &str = "__Host-session";

/// Env var holding the HS256 secret. Mirrors the env name
/// `auth_magic_link_token::jwt_secret` reads from so signing +
/// verification share the same key in production.
const SECRET_ENV: &str = "SKYCOT_AUTH_JWT_SECRET";

/// Parsed cookie-JWT claims, returned to the caller on a successful
/// verify so downstream route handlers (e.g. `routes_auth_logout`)
/// can write the principal/tenant binding without re-decoding the
/// cookie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct VerifiedClaims {
    /// `sub` — user_id from the `user_account` substrate.
    pub sub: String,
    /// `email` — denormalised onto the JWT for downstream audit
    /// emission; never used as an auth identifier.
    pub email: String,
    /// `tnt` — currently-bound tenant_id (may be absent for users
    /// mid-onboarding before they pick a workspace).
    #[serde(default)]
    pub tnt: Option<String>,
    /// `jti` — uuid v7 minted into every freshly-signed cookie JWT by
    /// the sibling cookies+jti brief. When present, the verify gate
    /// queries `revoked_tokens` on it. Optional so pre-jti cookies in
    /// the wild still verify (the brief's note: "v1.x rotation
    /// deferral" — cookies issued before jti-emission landed cannot
    /// be revoked, but the cohort drains naturally on the 7-day
    /// `exp` window).
    #[serde(default)]
    pub jti: Option<String>,
    /// `iat` — issued-at epoch seconds.
    pub iat: i64,
    /// `exp` — expiry epoch seconds.
    pub exp: i64,
}

/// Outcome of trying to verify a `__Host-session` cookie. Caller
/// decides whether to short-circuit (401) or let the request through
/// to the next gate (bearer-token / per-route).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SessionVerifyOutcome {
    /// No `Cookie:` header or no `__Host-session=…` value in it.
    /// Caller falls through to existing bearer-token auth.
    NoCookie,
    /// Cookie present and fully valid (signature + `exp` + not
    /// revoked). Caller may attach [`VerifiedClaims`] to the request
    /// context for downstream handlers (e.g. logout reads `jti`).
    Valid(VerifiedClaims),
    /// Cookie present but failed verification in some way. Caller
    /// MUST return 401 with the contained body so callers without a
    /// valid session cannot side-step the gate by relying on the
    /// bearer-token fallback.
    Reject(&'static str),
}

/// Verify the `__Host-session` cookie on `req`. Async because the
/// revocation check hits SQLite (`revoked_tokens.is_revoked`).
///
/// `store` is `Option` to match the surrounding dispatcher signature;
/// a missing store after a successful decode is treated as a Reject
/// to fail closed — the gateway booted without persistence and cannot
/// confirm the cookie is not revoked.
pub(super) async fn verify_session_cookie(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> SessionVerifyOutcome {
    let outcome = decode_session_cookie(req);
    let claims = match outcome {
        DecodeOutcome::NoCookie => return SessionVerifyOutcome::NoCookie,
        DecodeOutcome::Reject(reason) => return SessionVerifyOutcome::Reject(reason),
        DecodeOutcome::Decoded(c) => c,
    };
    let Some(s) = store else {
        return SessionVerifyOutcome::Reject("store unavailable");
    };
    if let Some(jti) = claims.jti.as_deref() {
        match argo_store::revoked_tokens::is_revoked(s.pool(), jti).await {
            Ok(true) => return SessionVerifyOutcome::Reject("session revoked"),
            Ok(false) => {}
            Err(_) => return SessionVerifyOutcome::Reject("store error"),
        }
    }
    SessionVerifyOutcome::Valid(claims)
}

/// Outcome of [`decode_session_cookie`] — signature + `exp` validation
/// only, no revocation check. Callers that need the revocation gate
/// (the verify-middleware on authenticated routes) wrap this via
/// [`verify_session_cookie`]; callers that need raw claims regardless
/// of revocation (the logout endpoint, which must remain reachable
/// even for an already-revoked cookie so a confused client can still
/// reach the cookie-clear path) call [`decode_session_cookie`]
/// directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DecodeOutcome {
    /// No cookie at all.
    NoCookie,
    /// Cookie present but signature or `exp` failed.
    Reject(&'static str),
    /// Cookie present, signature + `exp` both valid. Returns claims
    /// *without* consulting `revoked_tokens` — caller decides what to
    /// do with that.
    Decoded(VerifiedClaims),
}

/// Decode + verify-signature + check-exp for a `__Host-session`
/// cookie. Does NOT consult `revoked_tokens` — see [`DecodeOutcome`].
pub(super) fn decode_session_cookie(req: &str) -> DecodeOutcome {
    let Some(cookie_value) = extract_session_cookie(req) else {
        return DecodeOutcome::NoCookie;
    };
    let secret = jwt_secret();
    if secret.is_empty() {
        // No secret configured — every signature check would fail
        // anyway. Treat as no-cookie so dev runs that haven't set
        // `SKYCOT_AUTH_JWT_SECRET` don't 401 every browser request
        // and break the recovery path. Production callers detect the
        // missing secret upstream.
        return DecodeOutcome::NoCookie;
    }
    let key = DecodingKey::from_secret(&secret);
    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    // `exp` validation is on by default. `iat` is not required (the
    // signing path mints it; verification accepts missing for
    // forward-compat with future signer variants).
    validation.required_spec_claims.clear();
    validation.required_spec_claims.insert("exp".to_string());
    match jsonwebtoken::decode::<VerifiedClaims>(&cookie_value, &key, &validation) {
        Ok(data) => DecodeOutcome::Decoded(data.claims),
        Err(err) => match err.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                DecodeOutcome::Reject("session expired")
            }
            _ => DecodeOutcome::Reject("session invalid"),
        },
    }
}

/// Parse the `__Host-session` value out of the request's `Cookie:`
/// header. Returns `None` when the header is absent or contains no
/// `__Host-session=...` pair. Header line is case-insensitive
/// (RFC 7230); cookie names are case-sensitive (RFC 6265).
pub(super) fn extract_session_cookie(req: &str) -> Option<String> {
    let header = req
        .lines()
        .find(|l| l.split_once(':').is_some_and(|(k, _)| k.eq_ignore_ascii_case("Cookie")))?;
    let (_, value) = header.split_once(':')?;
    for pair in value.split(';') {
        let pair = pair.trim();
        if let Some(rest) = pair.strip_prefix(&format!("{SESSION_COOKIE_NAME}=")) {
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

/// Read the HS256 secret. Returns the env-var bytes (verbatim) when
/// `SKYCOT_AUTH_JWT_SECRET` is set, empty vec otherwise. Production
/// callers ensure the env var is set; dev runs without it cannot
/// verify any cookie (and the [`verify_session_cookie`] gate treats
/// the empty-secret case as `NoCookie` so the bearer path still
/// works).
pub(super) fn jwt_secret() -> Vec<u8> {
    std::env::var(SECRET_ENV)
        .unwrap_or_default()
        .into_bytes()
}

#[cfg(test)]
#[path = "middleware_session_verify_tests.rs"]
mod tests;
