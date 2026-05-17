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

use super::body_from_request;
use super::email_outbound::{self, OutboundMessage};
use super::rate_limit_ip::{self, RateLimitIpCheckResult};
use argo_config::EmailOutboundConfig;
use argo_email_templates::MagicLinkEmail;
use argo_identity::user::UserAccount;
use argo_identity::user_store::{get_user_by_email, insert_user, update_last_login};
use argo_identity::user_tenant_store::list_tenants_for_user;
use argo_store::user_account::{self, ConsumedToken};
use argo_store::SessionStore;
use argo_types::ArgoResult;
use base64::Engine;
use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, OnceLock};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

const JWT_TTL_SECS: i64 = 7 * 24 * 60 * 60;
const SESSION_COOKIE_NAME: &str = "__Host-session";

#[cfg(test)]
#[path = "auth_magic_link_tests.rs"]
mod tests;

#[derive(Debug, Deserialize)]
struct SignupBody {
    email: String,
}

#[derive(Debug, Deserialize)]
struct VerifyBody {
    token: String,
}

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
    if req.starts_with("POST /api/v1/auth/signup ")
        || req.starts_with("POST /api/v1/auth/signup?")
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

pub(super) async fn signup_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(s) = store else {
        return error_resp("500 Internal Server Error", "store_unavailable");
    };
    let ip = signup_source_ip(req);
    match rate_limit_ip::signup_limiter().check(ip) {
        RateLimitIpCheckResult::Deny { retry_after_secs } => {
            return (
                "429 Too Many Requests",
                serde_json::json!({
                    "error": "rate_limited",
                    "scope": "signup",
                    "retry_after_secs": retry_after_secs,
                })
                .to_string(),
            );
        }
        RateLimitIpCheckResult::Allow => {}
    }
    let parsed: SignupBody = match serde_json::from_str(body_from_request(req)) {
        Ok(v) => v,
        Err(e) => return error_resp("400 Bad Request", &format!("invalid body: {e}")),
    };
    let email = parsed.email.trim().to_ascii_lowercase();
    if !looks_like_email(&email) {
        return error_resp("400 Bad Request", "email is required");
    }
    let raw_token = mint_token();
    let token_hash = sha256_hex(&raw_token);
    let now = Utc::now().timestamp();
    if let Err(e) = user_account::insert_magic_link(
        s.pool(),
        &token_hash,
        &email,
        now + user_account::MAGIC_LINK_TTL_SECS,
    )
    .await
    {
        return error_resp("500 Internal Server Error", &format!("store: {e}"));
    }
    deliver_magic_link(&email, &raw_token).await;
    ("200 OK", r#"{"status":"sent"}"#.to_string())
}

/// Stream-owning verify handler that emits `Set-Cookie:
/// __Host-session=...; HttpOnly; Secure; SameSite=Lax; Path=/;
/// Max-Age=...` directly. The body is the JSON envelope the SPA
/// reads for the `needs_onboarding` / `tenant_id` redirect choice;
/// the JWT itself is no longer in the body.
pub(super) async fn serve_verify_stream(
    req: &str,
    store: Option<&Arc<SessionStore>>,
    out: &mut TcpStream,
) -> tokio::io::Result<()> {
    let outcome = verify_outcome(req, store).await;
    let mut head = format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n",
        outcome.status,
        outcome.body.len()
    );
    if let Some(cookie) = &outcome.set_cookie {
        head.push_str("Set-Cookie: ");
        head.push_str(cookie);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");
    head.push_str(&outcome.body);
    out.write_all(head.as_bytes()).await?;
    out.shutdown().await
}

pub(super) struct VerifyOutcome {
    pub status: &'static str,
    pub body: String,
    pub set_cookie: Option<String>,
}

pub(super) async fn verify_outcome(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> VerifyOutcome {
    let Some(s) = store else {
        return err_outcome("500 Internal Server Error", "store_unavailable");
    };
    let parsed: VerifyBody = match serde_json::from_str(body_from_request(req)) {
        Ok(v) => v,
        Err(e) => return err_outcome("400 Bad Request", &format!("invalid body: {e}")),
    };
    let token = parsed.token.trim();
    if token.is_empty() {
        return err_outcome("400 Bad Request", "token is required");
    }
    let now = Utc::now().timestamp();
    let email = match user_account::consume_magic_link(s.pool(), &sha256_hex(token), now).await {
        Ok(ConsumedToken::Ok { email }) => email,
        Ok(ConsumedToken::Expired) => return err_outcome("401 Unauthorized", "expired"),
        Ok(ConsumedToken::AlreadyUsed) => return err_outcome("401 Unauthorized", "already_used"),
        Ok(ConsumedToken::NotFound) => return err_outcome("401 Unauthorized", "invalid"),
        Err(e) => return err_outcome("500 Internal Server Error", &format!("store: {e}")),
    };
    let user = match upsert_user(s, &email).await {
        Ok(u) => u,
        Err(e) => return err_outcome("500 Internal Server Error", &format!("user: {e}")),
    };
    let tenant_id = match list_tenants_for_user(s, user.id).await {
        Ok(mut tenants) => tenants.pop().map(|t| t.tenant_id.to_string()),
        Err(e) => return err_outcome("500 Internal Server Error", &format!("tenant: {e}")),
    };
    let jwt = match mint_jwt(&user.id.to_string(), &email, tenant_id.as_deref(), now) {
        Ok(t) => t,
        Err(msg) => return err_outcome("500 Internal Server Error", &msg),
    };
    let body = serde_json::json!({
        "needs_onboarding": tenant_id.is_none(),
        "tenant_id": tenant_id,
    });
    VerifyOutcome {
        status: "200 OK",
        body: body.to_string(),
        set_cookie: Some(format!(
            "{SESSION_COOKIE_NAME}={jwt}; Max-Age={JWT_TTL_SECS}; Path=/; HttpOnly; Secure; SameSite=Lax"
        )),
    }
}

fn err_outcome(status: &'static str, msg: &str) -> VerifyOutcome {
    VerifyOutcome {
        status,
        body: serde_json::json!({ "error": msg }).to_string(),
        set_cookie: None,
    }
}

/// Find-or-insert the `user_account` row for `email`. Existing users
/// bump `last_login_at`; new users are inserted with default status.
async fn upsert_user(store: &SessionStore, email: &str) -> ArgoResult<UserAccount> {
    if let Some(u) = get_user_by_email(store, email).await? {
        update_last_login(store, u.id, Utc::now()).await?;
        return Ok(u);
    }
    let user = UserAccount::new(email.to_string());
    insert_user(store, &user).await?;
    Ok(user)
}

fn mint_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn sha256_hex(s: &str) -> String {
    hex::encode(Sha256::digest(s.as_bytes()))
}

#[derive(Debug, Serialize)]
struct Claims<'a> {
    sub: &'a str,
    email: &'a str,
    tnt: Option<&'a str>,
    iat: i64,
    exp: i64,
}

fn mint_jwt(
    user_id: &str,
    email: &str,
    tenant_id: Option<&str>,
    now: i64,
) -> Result<String, String> {
    let key = EncodingKey::from_secret(&jwt_secret());
    jsonwebtoken::encode(
        &Header::new(jsonwebtoken::Algorithm::HS256),
        &Claims {
            sub: user_id,
            email,
            tnt: tenant_id,
            iat: now,
            exp: now + JWT_TTL_SECS,
        },
        &key,
    )
    .map_err(|e| format!("jwt encode: {e}"))
}

/// Production sets `SKYCOT_AUTH_JWT_SECRET`; dev runs mint an
/// ephemeral per-process secret so the loop works without explicit
/// config. The ephemeral key invalidates issued JWTs on restart.
/// (Hardening: `claude-jwt-cookie-security-hardening` follow-up.)
fn jwt_secret() -> Vec<u8> {
    if let Ok(s) = std::env::var("SKYCOT_AUTH_JWT_SECRET") {
        if !s.is_empty() {
            return s.into_bytes();
        }
    }
    static EPHEMERAL: OnceLock<Vec<u8>> = OnceLock::new();
    EPHEMERAL
        .get_or_init(|| {
            let mut buf = vec![0u8; 32];
            OsRng.fill_bytes(&mut buf);
            tracing::warn!(
                target: "gateway::auth_magic_link",
                "SKYCOT_AUTH_JWT_SECRET unset — ephemeral key minted"
            );
            buf
        })
        .clone()
}

async fn deliver_magic_link(email: &str, raw_token: &str) {
    let base = std::env::var("SKYCOT_PUBLIC_BASE_URL")
        .unwrap_or_else(|_| "https://app.skycot.com".to_string());
    let magic_url = format!("{base}/auth/verify?token={raw_token}");
    let rendered = MagicLinkEmail {
        magic_url: magic_url.clone(),
        email: email.to_string(),
        expires_in: "15 minutes".to_string(),
    }
    .render();
    tracing::warn!(
        target: "gateway::auth_magic_link",
        recipient = %email,
        subject = %rendered.subject,
        link = %magic_url,
        "magic link issued"
    );
    let cfg = outbound_config();
    let from = if cfg.from_address.is_empty() {
        "noreply@skycot.com"
    } else {
        cfg.from_address.as_str()
    };
    let msg = OutboundMessage {
        to: email,
        from,
        rendered: &rendered,
    };
    if let Err(reason) = email_outbound::send(cfg, msg).await {
        // No retry, no caller-visible failure — log and move on so
        // the signup endpoint always returns 200. Retry policy is a
        // follow-up brief in the `codex-billing-webhook-resilience`
        // style if real outbound is exercised at v1.x scale.
        tracing::warn!(
            target: "gateway::auth_magic_link",
            recipient = %email,
            provider = cfg.provider.as_str(),
            reason = %reason,
            "outbound email failed; magic link still in trace log for operator copy"
        );
    }
}

/// Load `[email.outbound]` lazily on first signup and cache it for
/// the lifetime of the process. Operator config edits require a
/// restart, matching the existing pattern for other gateway knobs.
/// Test runs that don't set `ARGO_CONFIG_PATH` see the default
/// (`provider = "noop"`).
fn outbound_config() -> &'static EmailOutboundConfig {
    static CFG: OnceLock<EmailOutboundConfig> = OnceLock::new();
    CFG.get_or_init(|| argo_config::load().unwrap_or_default().email.outbound)
}

fn looks_like_email(s: &str) -> bool {
    let Some((local, domain)) = s.split_once('@') else {
        return false;
    };
    s.len() >= 3
        && s.len() <= 254
        && !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
}

fn signup_source_ip(req: &str) -> IpAddr {
    rate_limit_ip::parse_first_xff_hop(req).unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST))
}

fn error_resp(status: &'static str, msg: &str) -> (&'static str, String) {
    (status, serde_json::json!({ "error": msg }).to_string())
}
