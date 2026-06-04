//! Public early-access waitlist capture — `POST /api/v1/waitlist` (WS6).
//!
//! The landing page's waitlist form (`WaitlistForm.tsx` plus
//! `web/lib/api/waitlist.ts`) posts `{ email, source? }` (with optional
//! `name`/`company`) to capture organic top-of-funnel signups. Per NO-THEATER
//! (DEC-007) the form does not fake success — until this handler shipped every
//! call resolved to a 404. This is the funnel's front door.
//!
//! **Public / anonymous-safe.** Mounted ahead of the bearer-auth gate in
//! `routes.rs`, in the same un-gated lane as magic-link signup
//! (`auth_magic_link::dispatch`). It reuses the existing per-IP signup
//! limiter (`rate_limit_ip::signup_limiter()`) rather than inventing a new
//! one, plus the same hidden-field honeypot convention the signup form uses.
//!
//! It deliberately does NOT route through `signup_protection::check_signup`:
//! that path also enforces a 60s per-email cooldown, which would answer a
//! legitimate repeat signup with `429` and break the idempotent-`202`
//! acceptance criterion below. The `ON CONFLICT(email) DO NOTHING` insert IS
//! the idempotency mechanism — a repeat email is a no-op `202`, not a 429.
//!
//! **Platform-level (pre-tenant).** The waitlist sits before any workspace
//! exists, so rows carry NO `tenant_id` — exactly like the pre-tenant
//! `magic_link` table. Persistence is the additive `waitlist_signups` table
//! (migration v88), deduped on `email` via `ON CONFLICT(email) DO NOTHING` so
//! a repeat submission is an idempotent `202 {"queued":true}`, never a 500.

use crate::handlers::gateway::rate_limit_ip::{self, RateLimitIpCheckResult};
use argo_store::SessionStore;
use serde::Deserialize;
use std::sync::Arc;

const QUEUED_BODY: &str = r#"{"queued":true}"#;

/// Wire shape posted by `web/lib/api/waitlist.ts`. Only `email` is required;
/// `name`/`company`/`source` are optional. `honeypot_url` feeds the shared
/// signup honeypot guard (a bot that fills the hidden field is silently
/// accepted with no row written).
#[derive(Debug, Deserialize)]
struct WaitlistBody {
    email: String,
    name: Option<String>,
    company: Option<String>,
    source: Option<String>,
    honeypot_url: Option<String>,
}

/// Match the public waitlist POST (with or without a query string).
pub(super) fn is_waitlist_request(req: &str) -> bool {
    req.starts_with("POST /api/v1/waitlist ") || req.starts_with("POST /api/v1/waitlist?")
}

/// Handle `POST /api/v1/waitlist`. Returns the canonical gateway
/// `(status, body)` tuple. Validates the email, runs the shared signup abuse
/// controls, then performs an idempotent insert into `waitlist_signups`.
pub(super) async fn waitlist_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(s) = store else {
        return super::auth_magic_link::error_resp("500 Internal Server Error", "store_unavailable");
    };
    // Hard per-IP cap (same bucket the magic-link signup uses).
    let ip = super::auth_magic_link::signup_source_ip(req);
    if let RateLimitIpCheckResult::Deny { retry_after_secs } = rate_limit_ip::signup_limiter().check(ip)
    {
        return (
            "429 Too Many Requests",
            serde_json::json!({
                "error": "rate_limited",
                "scope": "waitlist",
                "retry_after_secs": retry_after_secs,
            })
            .to_string(),
        );
    }
    let parsed: WaitlistBody =
        match serde_json::from_str(crate::handlers::gateway::body_from_request(req)) {
            Ok(v) => v,
            Err(e) => {
                return super::auth_magic_link::error_resp(
                    "400 Bad Request",
                    &format!("invalid body: {e}"),
                )
            }
        };
    // Honeypot: a bot that fills the hidden `honeypot_url` field is silently
    // accepted (no row written) so it can't tell it was caught — mirrors the
    // SilentOk semantics of the magic-link signup honeypot.
    if parsed
        .honeypot_url
        .as_deref()
        .is_some_and(|v| !v.trim().is_empty())
    {
        return ("202 Accepted", QUEUED_BODY.to_string());
    }
    let email = parsed.email.trim().to_ascii_lowercase();
    if !super::auth_magic_link::looks_like_email(&email) {
        return super::auth_magic_link::error_resp("400 Bad Request", "email is required");
    }
    let name = normalize_opt(parsed.name);
    let company = normalize_opt(parsed.company);
    let source = normalize_opt(parsed.source);
    let id = uuid::Uuid::now_v7().to_string();
    let created_at = chrono::Utc::now().timestamp();
    let insert = sqlx::query(
        "INSERT INTO waitlist_signups (id, email, name, company, source, created_at) \
         VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT(email) DO NOTHING",
    )
    .bind(&id)
    .bind(&email)
    .bind(&name)
    .bind(&company)
    .bind(&source)
    .bind(created_at)
    .execute(s.pool())
    .await;
    if let Err(e) = insert {
        return super::auth_magic_link::error_resp(
            "500 Internal Server Error",
            &format!("store: {e}"),
        );
    }
    // Idempotent: a duplicate email inserts zero rows but still returns 202.
    ("202 Accepted", QUEUED_BODY.to_string())
}

/// Trim an optional string and treat an empty result as absent.
fn normalize_opt(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

#[cfg(test)]
#[path = "routes_waitlist_tests.rs"]
mod tests;
