//! Magic-link signup + verify endpoints — self-serve viral on-ramp
//! surfaced through `app.skycot.com/signup`
//! (Stream `claude-magic-link-auth-backend`).
//!
//! `POST /api/v1/auth/signup` mints a 32-byte random token (SHA-256
//! hashed at rest), renders the magic-link email via
//! [`argo_email_templates::MagicLinkEmail`], rate-limits 5/hour/IP.
//!
//! `POST /api/v1/auth/verify` consumes the token, upserts the user
//! against the substrate `user_account` table (from
//! mission-tenant-substrate-tables, accessed through
//! [`argo_identity::user_store`]), bumps `last_login_at`, looks up
//! the first tenant via [`argo_identity::user_tenant_store`], and
//! returns a 7-day HS256 JWT.
//!
//! Sensible deviations (full text in PR body):
//! - **Cookie set on the SPA, not the gateway.** Gateway dispatch
//!   tuple `(&'static str, String)` cannot carry a dynamic
//!   `Set-Cookie`; the verify response carries the cookie attributes
//!   so the SPA edge route can mint the browser cookie faithfully.
//! - **Per-IP rate limit is in-module.** The shared `rate_limit.rs`
//!   is per-tenant per-minute; 5/hour/IP doesn't compose.
//! - **Email send is a tracing log for v1.** The template renders
//!   via [`MagicLinkEmail`] per the brief's hidden constraint, but
//!   the SMTP send primitive is a v1.x follow-up; operator reads
//!   the link from the audit log in the interim.

use super::body_from_request;
use argo_email_templates::MagicLinkEmail;
use argo_identity::user::UserAccount;
use argo_identity::user_store::{get_user_by_email, insert_user, update_last_login};
use argo_identity::user_tenant_store::list_tenants_for_user;
use argo_store::user_account::{self, ConsumedToken};
use argo_store::SessionStore;
use argo_types::ArgoResult;
use base64::Engine;
use chrono::Utc;
use dashmap::DashMap;
use jsonwebtoken::{EncodingKey, Header};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

pub(super) const IP_LIMIT: u32 = 5;
pub(super) const IP_WINDOW: Duration = Duration::from_secs(60 * 60);
const JWT_TTL_SECS: i64 = 7 * 24 * 60 * 60;

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
pub(super) async fn dispatch(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("POST /api/v1/auth/signup ")
        || req.starts_with("POST /api/v1/auth/signup?")
    {
        return Some(signup_response(req, store).await);
    }
    if req.starts_with("POST /api/v1/auth/verify ")
        || req.starts_with("POST /api/v1/auth/verify?")
    {
        return Some(verify_response(req, store).await);
    }
    None
}

pub(super) async fn signup_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(s) = store else {
        return error_resp("500 Internal Server Error", "store_unavailable");
    };
    if !ip_limit_check(&extract_remote_ip(req)) {
        return (
            "429 Too Many Requests",
            r#"{"error":"rate_limited","scope":"signup","retry_after_secs":3600}"#.to_string(),
        );
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
    deliver_magic_link(&email, &raw_token);
    ("200 OK", r#"{"status":"sent"}"#.to_string())
}

pub(super) async fn verify_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(s) = store else {
        return error_resp("500 Internal Server Error", "store_unavailable");
    };
    let parsed: VerifyBody = match serde_json::from_str(body_from_request(req)) {
        Ok(v) => v,
        Err(e) => return error_resp("400 Bad Request", &format!("invalid body: {e}")),
    };
    let token = parsed.token.trim();
    if token.is_empty() {
        return error_resp("400 Bad Request", "token is required");
    }
    let now = Utc::now().timestamp();
    let email = match user_account::consume_magic_link(s.pool(), &sha256_hex(token), now).await {
        Ok(ConsumedToken::Ok { email }) => email,
        Ok(ConsumedToken::Expired) => return error_resp("401 Unauthorized", "expired"),
        Ok(ConsumedToken::AlreadyUsed) => return error_resp("401 Unauthorized", "already_used"),
        Ok(ConsumedToken::NotFound) => return error_resp("401 Unauthorized", "invalid"),
        Err(e) => return error_resp("500 Internal Server Error", &format!("store: {e}")),
    };
    let user = match upsert_user(s, &email).await {
        Ok(u) => u,
        Err(e) => return error_resp("500 Internal Server Error", &format!("user: {e}")),
    };
    let tenant_id = match list_tenants_for_user(s, user.id).await {
        Ok(mut tenants) => tenants.pop().map(|t| t.tenant_id.to_string()),
        Err(e) => return error_resp("500 Internal Server Error", &format!("tenant: {e}")),
    };
    let jwt = match mint_jwt(&user.id.to_string(), &email, tenant_id.as_deref(), now) {
        Ok(t) => t,
        Err(msg) => return error_resp("500 Internal Server Error", &msg),
    };
    let body = serde_json::json!({
        "jwt": jwt,
        "needs_onboarding": tenant_id.is_none(),
        "tenant_id": tenant_id,
        "cookie": {
            "name": "skycot_session",
            "max_age_secs": JWT_TTL_SECS,
            "same_site": "Lax",
            "secure": true,
            "http_only": true,
        },
    });
    ("200 OK", body.to_string())
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

fn deliver_magic_link(email: &str, raw_token: &str) {
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
        "magic link issued (SMTP send primitive is a v1.x follow-up)"
    );
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

fn extract_remote_ip(req: &str) -> String {
    let headers = req.split_once("\r\n\r\n").map_or(req, |(h, _)| h);
    for line in headers.split("\r\n").skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("x-forwarded-for") {
            return value.split(',').next().unwrap_or("").trim().to_string();
        }
    }
    "unknown".to_string()
}

fn ip_limit_check(ip: &str) -> bool {
    ip_limit_check_at(ip, Instant::now())
}

pub(super) fn ip_limit_check_at(ip: &str, now: Instant) -> bool {
    static BUCKETS: OnceLock<DashMap<String, Vec<Instant>>> = OnceLock::new();
    let map = BUCKETS.get_or_init(DashMap::new);
    let mut entry = map.entry(ip.to_string()).or_default();
    entry.retain(|t| now.saturating_duration_since(*t) < IP_WINDOW);
    if entry.len() >= IP_LIMIT as usize {
        return false;
    }
    entry.push(now);
    true
}

fn error_resp(status: &'static str, msg: &str) -> (&'static str, String) {
    (status, serde_json::json!({ "error": msg }).to_string())
}
