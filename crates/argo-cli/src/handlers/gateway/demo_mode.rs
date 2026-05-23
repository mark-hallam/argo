//! Env-var-gated demo-mode signup bypass for v1.0 mission-MVP
//! recording (`demo-ops-add-magic-link-bypass-for-recording`).
//!
//! `POST /api/v1/demo/mint-principal` mints a verified
//! `human:<email>` principal session WITHOUT a real magic-link
//! email round-trip, so the operator can record the 5-7 minute
//! demo as a single take. Active only when `ARGO_DEMO_MODE=1` is
//! set in the gateway's startup environment — the flag is cached
//! on first read, no runtime flip.
//!
//! Every mint emits a `tracing::warn!` line and writes an
//! `AuditKind::AuthSucceeded` row whose payload carries
//! `demo_mode_bypass: true` — so an operator who accidentally
//! booted with the flag in a prod-shaped environment sees obvious
//! noise plus a permanent forensic record.
//!
//! Mounted ahead of the bearer-auth gate (mirroring `auth_magic_link::dispatch`)
//! so the SPA can call it anonymously. Startup-time refusal to
//! co-exist with bearer-auth lives in [`super::startup_check`].
use super::auth_magic_link::{jwt_secret, looks_like_email, JWT_TTL_SECS};
use argo_audit::{AuditKind, AuditSink, ANONYMOUS_AUDIT_TENANT_ID};
use argo_store::SessionStore;
use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::{Arc, OnceLock};

/// Env var that gates the bypass. Read once via [`is_enabled`] and
/// cached for the lifetime of the gateway process.
pub(super) const DEMO_MODE_ENV: &str = "ARGO_DEMO_MODE";

/// Returns `true` when `ARGO_DEMO_MODE=1` was set at gateway start.
/// Cached on first read so a malicious caller can't flip the flag
/// mid-process via `std::env::set_var`.
pub(super) fn is_enabled() -> bool {
    static CACHED: OnceLock<bool> = OnceLock::new();
    *CACHED.get_or_init(|| {
        std::env::var(DEMO_MODE_ENV)
            .map(|v| v == "1")
            .unwrap_or(false)
    })
}

/// Route classifier — `true` for `POST /api/v1/demo/mint-principal`.
pub(super) fn is_mint_principal_request(req: &str) -> bool {
    req.starts_with("POST /api/v1/demo/mint-principal ")
        || req.starts_with("POST /api/v1/demo/mint-principal?")
}

#[derive(Debug, Deserialize)]
struct MintBody {
    email: String,
}

#[derive(Debug, Serialize)]
struct Claims<'a> {
    sub: &'a str,
    email: &'a str,
    iat: i64,
    exp: i64,
    demo: bool,
}

/// Dispatch entry-point. Returns `None` when demo mode is off or
/// the request isn't the mint route; the caller falls through to
/// the normal route table (which serves the path as a 404).
pub(super) async fn dispatch(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if !is_enabled() {
        return None;
    }
    if !is_mint_principal_request(req) {
        return None;
    }
    Some(mint_principal_response(req, store).await)
}

async fn mint_principal_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let parsed: MintBody =
        match serde_json::from_str(crate::handlers::gateway::body_from_request(req)) {
            Ok(v) => v,
            Err(e) => return error_resp("400 Bad Request", &format!("invalid body: {e}")),
        };
    let email = parsed.email.trim().to_ascii_lowercase();
    if !looks_like_email(&email) {
        return error_resp("400 Bad Request", "email is required");
    }
    let now = Utc::now().timestamp();
    let token = match mint_demo_session(&email, now) {
        Ok(t) => t,
        Err(msg) => return error_resp("500 Internal Server Error", &msg),
    };
    tracing::warn!(
        target: "argo_cli::demo_mode",
        email = %email,
        "demo-mode magic-link bypass minted principal for {email}",
    );
    if let Some(s) = store {
        write_demo_audit(s, &email).await;
    }
    let principal = format!("human:{email}");
    let body = json!({
        "ok": true,
        "principal": principal,
        "session_token": token,
    });
    ("200 OK", body.to_string())
}

fn mint_demo_session(email: &str, now: i64) -> Result<String, String> {
    let key = EncodingKey::from_secret(&jwt_secret());
    jsonwebtoken::encode(
        &Header::new(jsonwebtoken::Algorithm::HS256),
        &Claims {
            sub: email,
            email,
            iat: now,
            exp: now + JWT_TTL_SECS,
            demo: true,
        },
        &key,
    )
    .map_err(|e| format!("jwt encode: {e}"))
}

async fn write_demo_audit(store: &Arc<SessionStore>, email: &str) {
    let payload = json!({
        "method": "demo_mode_bypass",
        "demo_mode_bypass": true,
        "email": email,
    });
    let sink = AuditSink::new_with_tenant(
        Arc::clone(store),
        ANONYMOUS_AUDIT_TENANT_ID.to_string(),
    );
    if let Err(e) = sink
        .write(None, AuditKind::AuthSucceeded, payload, None)
        .await
    {
        tracing::warn!(
            target: "argo_cli::demo_mode",
            error = %e,
            "demo-mode audit write failed",
        );
    }
}

fn error_resp(status: &'static str, msg: &str) -> (&'static str, String) {
    (status, json!({ "error": msg }).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_mint_principal_request_matches_both_forms() {
        assert!(is_mint_principal_request(
            "POST /api/v1/demo/mint-principal HTTP/1.1\r\n\r\n"
        ));
        assert!(is_mint_principal_request(
            "POST /api/v1/demo/mint-principal?source=spa HTTP/1.1\r\n\r\n"
        ));
        assert!(!is_mint_principal_request(
            "POST /api/v1/demo/other HTTP/1.1\r\n\r\n"
        ));
        assert!(!is_mint_principal_request(
            "GET /api/v1/demo/mint-principal HTTP/1.1\r\n\r\n"
        ));
    }

    #[test]
    fn mint_demo_session_round_trips() {
        let jwt = mint_demo_session("demo@example.com", 1_700_000_000).expect("mint");
        let key = jsonwebtoken::DecodingKey::from_secret(&jwt_secret());
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.validate_exp = false;
        #[derive(Debug, serde::Deserialize)]
        struct Verify {
            email: String,
            demo: bool,
        }
        let data = jsonwebtoken::decode::<Verify>(&jwt, &key, &validation).expect("decode");
        assert_eq!(data.claims.email, "demo@example.com");
        assert!(data.claims.demo);
    }
}
