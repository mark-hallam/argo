//! `POST /api/v1/auth/logout` — terminate the cookie session
//! (`mission-revocation-with-verify-middleware-v1x`).
//!
//! Decodes the `__Host-session` cookie via [`super::middleware_session_verify`],
//! writes the JWT's `jti` claim into `revoked_tokens`, emits an
//! `AuditKind::LogoutCalled` row, and returns
//! `Set-Cookie: __Host-session=; Max-Age=0; ...` so the browser
//! purges the cookie immediately. Subsequent requests presenting the
//! same cookie are 401d by the verify-middleware.
//!
//! Stream-owning (mirroring `auth_magic_link_verify::serve_verify_stream`)
//! so the response can carry both a JSON body AND the cookie-clear
//! `Set-Cookie` header — the `(status, body)` tuple returned by
//! [`super::routes::dispatch_route_inner`] cannot carry headers.
//! Mounted in `boot_router::try_dispatch_gates` ahead of the
//! route-family fall-through.

use super::middleware_session_verify::{self, DecodeOutcome, VerifiedClaims};
use argo_audit::{AuditKind, AuditSink, ANONYMOUS_AUDIT_TENANT_ID};
use argo_store::SessionStore;
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

const LOGOUT_PATH: &str = "/api/v1/auth/logout";
/// Empty-value cookie with `Max-Age=0` instructs every conforming
/// browser to drop the original cookie immediately, regardless of the
/// path / domain the server previously set. `__Host-` prefix rules
/// (Secure; Path=/; no Domain) match the original cookie so the clear
/// supersedes it.
const COOKIE_CLEAR: &str =
    "__Host-session=; Max-Age=0; Path=/; HttpOnly; Secure; SameSite=Lax";

/// Returns `true` for `POST /api/v1/auth/logout` requests. The
/// matching handler is [`serve_logout_stream`], not a `dispatch`
/// shim, because the response must include a `Set-Cookie` header.
pub(super) fn is_logout_request(req: &str) -> bool {
    req.starts_with(&format!("POST {LOGOUT_PATH} "))
        || req.starts_with(&format!("POST {LOGOUT_PATH}?"))
}

/// Owned outcome of the logout handler, exposed for unit tests so
/// they can drive the logic without setting up a real `TcpStream`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LogoutOutcome {
    /// HTTP status line — `"200 OK"` on accepted logout,
    /// `"401 Unauthorized"` when the cookie was missing/invalid/expired.
    pub status: &'static str,
    /// JSON body the SPA edge route may surface to the operator.
    pub body: String,
    /// Set-Cookie header value emitted on success (always the
    /// `__Host-session` clear directive). `None` for 401 paths so
    /// failed logouts don't accidentally drop a still-valid cookie.
    pub set_cookie: Option<&'static str>,
}

/// Compute the logout outcome — verify cookie, write revocation row,
/// emit audit. Pure of any TcpStream I/O so the route tests can drive
/// the full path against an in-memory store.
pub(super) async fn logout_outcome(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> LogoutOutcome {
    let Some(s) = store else {
        return reject("store_unavailable");
    };
    // Logout uses the decode-only path (no revocation check) so a
    // confused client that double-clicks logout still reaches the
    // cookie-clear path — the verify-middleware enforces revocation
    // on EVERY OTHER route, but the logout endpoint is the way OUT
    // of the session and must remain reachable even when the cookie
    // is already in `revoked_tokens`. `argo_store::revoked_tokens::revoke`
    // is idempotent (ON CONFLICT DO NOTHING) so the second call is a
    // no-op against the table.
    match middleware_session_verify::decode_session_cookie(req) {
        DecodeOutcome::NoCookie => reject("missing_cookie"),
        DecodeOutcome::Reject(reason) => reject(reason),
        DecodeOutcome::Decoded(claims) => accept_logout(s, claims).await,
    }
}

/// Stream-owning entry point used by `boot_router::try_dispatch_gates`.
/// Writes the formatted HTTP/1.1 response (status line + headers +
/// body) directly to the connection. Best-effort: any I/O error
/// surfaces to the caller but the audit row still landed.
pub(super) async fn serve_logout_stream(
    req: &str,
    store: Option<&Arc<SessionStore>>,
    out: &mut TcpStream,
) -> tokio::io::Result<()> {
    let outcome = logout_outcome(req, store).await;
    let mut head = format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n",
        outcome.status,
        outcome.body.len()
    );
    if let Some(cookie) = outcome.set_cookie {
        head.push_str("Set-Cookie: ");
        head.push_str(cookie);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");
    head.push_str(&outcome.body);
    out.write_all(head.as_bytes()).await?;
    out.shutdown().await
}

async fn accept_logout(store: &Arc<SessionStore>, claims: VerifiedClaims) -> LogoutOutcome {
    let now = Utc::now().timestamp();
    let tenant_for_audit = claims
        .tnt
        .as_deref()
        .unwrap_or(ANONYMOUS_AUDIT_TENANT_ID);
    // Only write a revocation row when the cookie actually carries a
    // jti — pre-jti cookies in the wild (sibling brief had not landed
    // when they were minted) cannot be revoked, but the logout still
    // succeeds: the browser drops the cookie and the JWT drains on
    // the 7-day exp window. Worth recording the no-jti path in audit
    // so the auth-funnel rollup can quantify the cohort.
    if let Some(jti) = claims.jti.as_deref() {
        if let Err(e) = argo_store::revoked_tokens::revoke(
            store.pool(),
            jti,
            tenant_for_audit,
            Some(&claims.sub),
            Some("logout"),
            now,
        )
        .await
        {
            tracing::warn!(
                target: "gateway::auth_logout",
                error = %e,
                jti = jti,
                "revoked_tokens write failed; cookie clear still emitted"
            );
        }
    }
    emit_logout_audit(store, &claims, now).await;
    LogoutOutcome {
        status: "200 OK",
        body: json!({
            "status": "logged_out",
            "had_jti": claims.jti.is_some(),
        })
        .to_string(),
        set_cookie: Some(COOKIE_CLEAR),
    }
}

async fn emit_logout_audit(
    store: &Arc<SessionStore>,
    claims: &VerifiedClaims,
    at: i64,
) {
    let tenant_for_audit = claims
        .tnt
        .as_deref()
        .unwrap_or(ANONYMOUS_AUDIT_TENANT_ID);
    let payload = json!({
        "user_id": claims.sub,
        "tenant_id": claims.tnt,
        "jti": claims.jti,
        "at": at,
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_for_audit.to_string());
    if let Err(e) = sink.write(None, AuditKind::LogoutCalled, payload, None).await {
        tracing::warn!(
            target: "gateway::auth_logout",
            error = %e,
            "logout audit write failed"
        );
    }
}

fn reject(reason: &'static str) -> LogoutOutcome {
    LogoutOutcome {
        status: "401 Unauthorized",
        body: json!({ "error": reason }).to_string(),
        set_cookie: None,
    }
}

#[cfg(test)]
#[path = "routes_auth_logout_tests.rs"]
mod tests;
