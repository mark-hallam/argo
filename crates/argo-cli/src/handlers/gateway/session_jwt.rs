//! Session-JWT verification (HS256) for the bearer-auth path that
//! carries a per-user role claim.
//!
//! The magic-link verify endpoint (`auth_magic_link_token::mint_jwt`)
//! issues an HS256 JWT with `{ sub, email, tnt, role, iat, exp }` and
//! sets it as the `__Host-session` cookie. The SPA forwards that JWT
//! as `Authorization: Bearer <jwt>` on every gateway call. This module
//! verifies it against the same `SKYCOT_AUTH_JWT_SECRET` the minter
//! uses, and surfaces the claims to the role-gate.
//!
//! Why a sibling module to [`super::jwt`]: that one is the Ed25519 WS
//! token (`/inbox/ws?token=...`). HS256 session JWTs are a different
//! algorithm and signing key — separate file keeps the two trust
//! domains visually distinct and stops anyone wiring the wrong
//! verifier path.
//!
//! Production sets `SKYCOT_AUTH_JWT_SECRET`; dev runs without it
//! verify-fail closed (the role-gate falls through to its dev-only
//! `X-Argo-Role` trust path only when `ARGO_GATEWAY_TRUST_ROLE_HEADER`
//! is set — never in production).

use super::auth_magic_link;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;

/// Decoded session-JWT payload. Mirrors the encode-side struct in
/// [`super::auth_magic_link::mint_jwt`]; unknown fields are dropped.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(super) struct SessionClaims {
    /// User-id (UUID stringified) of the principal who verified the
    /// magic link.
    pub sub: String,
    /// Verified email address.
    #[serde(default)]
    pub email: String,
    /// Active tenant id (UUID stringified). `None` between signup and
    /// onboarding-provision.
    #[serde(default)]
    pub tnt: Option<String>,
    /// Lowercase role string the verify-time membership lookup stamped
    /// into the token. `None` when the user has no membership row yet.
    #[serde(default)]
    pub role: Option<String>,
    /// Issued-at (unix seconds).
    pub iat: i64,
    /// Expiry (unix seconds).
    pub exp: i64,
}

/// Verify `bearer` as a session JWT. Returns the decoded claims if the
/// signature, algorithm, and `exp` pass; otherwise returns `None` so
/// the caller can fall through to other auth strategies.
///
/// `None` is intentionally opaque — the gateway does NOT leak verify
/// reason back to the client because doing so would let attackers
/// probe which static bearer token the gateway accepts.
pub(super) fn verify_session_jwt(bearer: &str) -> Option<SessionClaims> {
    // A static bearer token won't have the `header.payload.sig` shape;
    // skip the secret-lookup cost on that path.
    if bearer.split('.').count() != 3 {
        return None;
    }
    let key = DecodingKey::from_secret(&auth_magic_link::jwt_secret());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    // The minter never sets `aud` / `iss` claims, so disable those
    // gates to match the issued shape.
    validation.required_spec_claims.clear();
    validation.required_spec_claims.insert("exp".into());
    validation.required_spec_claims.insert("iat".into());
    validation.leeway = 60;
    decode::<SessionClaims>(bearer, &key, &validation)
        .ok()
        .map(|t| t.claims)
}

/// Extract the bearer token from an `Authorization: Bearer <...>`
/// header. Mirrors [`super::routes_auth::presented_bearer`] but lives
/// here so the role-gate doesn't reach back into another module's
/// private helper.
pub(super) fn bearer_from_request(req: &str) -> Option<String> {
    req.lines()
        .find_map(|l| {
            l.split_once(':')
                .and_then(|(k, v)| k.eq_ignore_ascii_case("Authorization").then(|| v.trim()))
        })
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(str::trim)
        .map(str::to_string)
}

/// Extract the JWT from a `Cookie: __Host-session=<...>` header. The
/// SPA's API client sends the bearer in the Authorization header, but
/// server-rendered Next.js routes proxy with the cookie attached —
/// the gateway honors either source so audit / streaming routes don't
/// need an explicit bearer-attach step.
pub(super) fn session_cookie_from_request(req: &str) -> Option<String> {
    // Skip the request line ("GET / HTTP/1.1"); only headers carry
    // `Cookie: …`. Using `?` on `split_once(':')` would short-circuit
    // the whole function on the first colon-less line.
    for line in req.lines().skip(1) {
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        if !k.eq_ignore_ascii_case("Cookie") {
            continue;
        }
        for pair in v.split(';') {
            let pair = pair.trim();
            if let Some(rest) = pair.strip_prefix("__Host-session=") {
                return Some(rest.to_string());
            }
        }
    }
    None
}

/// Try every JWT source on a request — Authorization bearer first,
/// then `__Host-session` cookie — and return the first that verifies.
pub(super) fn extract_session_claims(req: &str) -> Option<SessionClaims> {
    if let Some(b) = bearer_from_request(req) {
        if let Some(c) = verify_session_jwt(&b) {
            return Some(c);
        }
    }
    if let Some(c) = session_cookie_from_request(req) {
        if let Some(c) = verify_session_jwt(&c) {
            return Some(c);
        }
    }
    None
}

#[cfg(test)]
#[path = "session_jwt_tests.rs"]
mod tests;
