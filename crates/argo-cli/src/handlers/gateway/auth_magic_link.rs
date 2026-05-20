//! Magic-link signup + verify endpoints — self-serve viral on-ramp
//! surfaced through `app.skycot.com/signup`.
//!
//! `POST /api/v1/auth/signup` mints a 32-byte random token (SHA-256
//! hashed at rest), renders the magic-link email via
//! [`argo_email_templates::MagicLinkEmail`], dispatches it through
//! the configured outbound provider (default
//! [`EmailOutboundProvider::Noop`] preserves the v1.0 log-only
//! behavior), and rate-limits 5/hour/IP through the shared
//! [`super::rate_limit_ip::signup_limiter`] bucket.
//!
//! `POST /api/v1/auth/verify` consumes the token, upserts the user
//! against the substrate `user_account` table (from
//! mission-tenant-substrate-tables, accessed through
//! [`argo_identity::user_store`]), bumps `last_login_at`, looks up
//! the first tenant via [`argo_identity::user_tenant_store`], and
//! returns a 7-day HS256 JWT.
//!
//! Verify is stream-owning (mirroring [`super::sso_oidc`]) so it can
//! emit a `Set-Cookie: __Host-session=...; HttpOnly; Secure;
//! SameSite=Lax; Path=/; Max-Age=...` response header directly from
//! the gateway. The previous shape sent the JWT in the response body
//! and asked the SPA edge route to mint the browser cookie; the SPA
//! now just forwards whatever the gateway emits, matching the
//! `__Host-` prefix discipline from
//! [`claude-jwt-cookie-security-hardening`].
use argo_store::SessionStore;
use std::sync::Arc;

#[path = "auth_magic_link_audit.rs"]
mod auth_audit;
#[path = "auth_magic_link_delivery.rs"]
mod auth_delivery;
#[path = "auth_magic_link_signup.rs"]
mod auth_signup;
#[path = "auth_magic_link_token.rs"]
mod auth_token;
#[path = "auth_magic_link_verify.rs"]
mod auth_verify;

#[cfg(test)]
pub(super) use crate::handlers::gateway::rate_limit_ip;
#[cfg(test)]
pub(super) use auth_audit::VERIFY_RATE_LIMIT_PER_MIN;
pub(super) use auth_audit::{
    emit_auth_failed, emit_auth_succeeded, error_resp, signup_source_ip, verify_rate_limit_check,
    VERIFY_RATE_LIMIT_WINDOW,
};
pub(super) use auth_delivery::{deliver_magic_link, looks_like_email};
pub(super) use auth_signup::signup_response;
pub(super) use auth_token::{mint_jwt, mint_token, sha256_hex};
pub(super) use auth_verify::serve_verify_stream;
#[cfg(test)]
pub(super) use auth_verify::verify_outcome;
#[cfg(test)]
pub(super) use std::net::IpAddr;

#[cfg(test)]
#[path = "auth_magic_link_tests.rs"]
mod tests;

/// Mounted from `routes::dispatch_route_inner` ahead of the
/// operator-bearer-auth gate so anonymous browsers can sign up.
///
/// `verify` is intentionally NOT served here: it owns its own
/// TcpStream via [`serve_verify_stream`] so it can emit a
/// `Set-Cookie` response header. The dispatch chain calls
/// [`is_verify_request`] earlier (in
/// [`super::boot_router::try_dispatch_gates`]).
pub(super) async fn dispatch(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("POST /api/v1/auth/signup ") || req.starts_with("POST /api/v1/auth/signup?")
    {
        return Some(signup_response(req, store).await);
    }
    None
}

/// Returns `true` for `POST /api/v1/auth/verify`. The matching
/// handler is [`serve_verify_stream`], not `dispatch`, because the
/// verify response must include a `Set-Cookie` header which the
/// `(status, body)` dispatch tuple cannot carry.
pub(super) fn is_verify_request(req: &str) -> bool {
    req.starts_with("POST /api/v1/auth/verify ") || req.starts_with("POST /api/v1/auth/verify?")
}
