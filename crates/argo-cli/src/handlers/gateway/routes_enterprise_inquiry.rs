//! Public enterprise Expression-of-Interest capture —
//! `POST /api/v1/enterprise-inquiry` (WS6-2).
//!
//! The `/pricing` + `/contact` enterprise "Talk to sales" CTA used to be a
//! `mailto:` only — no structured capture, no record, no audit trail. This
//! handler is the real front door for the EOI→MOU→contract motion: it persists
//! a structured inquiry (`company` + contact `name` + work `email`, plus
//! optional `seats` + `message`) and writes a hash-chained audit breadcrumb so
//! a lead arriving is tamper-evident — the moat applies to leads too.
//!
//! **Public / anonymous-safe.** Mounted ahead of the bearer-auth gate in
//! `routes.rs`, in the same un-gated lane as magic-link signup
//! (`auth_magic_link::dispatch`) and the waitlist (`routes_waitlist`). It
//! reuses the existing per-IP signup limiter (`rate_limit_ip::signup_limiter()`)
//! and the same hidden-field honeypot convention the signup + waitlist forms
//! use. Pattern mirrors `routes_waitlist.rs` (#2675).
//!
//! **Platform-level (pre-tenant).** The inquiry arrives before any workspace
//! exists, so rows carry NO `tenant_id` — exactly like the pre-tenant
//! `waitlist_signups` / `magic_link` tables. Persistence is the additive
//! `enterprise_inquiry` table (migration v89). It is **append-only**: there is
//! no `UNIQUE(email)` dedupe because a legitimate enterprise lead may inquire
//! more than once and every inquiry is a distinct sales record. Success is a
//! `202 {"queued":true}`.

use crate::handlers::gateway::rate_limit_ip::{self, RateLimitIpCheckResult};
use argo_audit::{AuditKind, AuditSink, ANONYMOUS_AUDIT_TENANT_ID};
use argo_store::SessionStore;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

const QUEUED_BODY: &str = r#"{"queued":true}"#;

/// Wire shape posted by `web/lib/api/enterprise-inquiry.ts`. `company`, `name`,
/// and `email` are required; `seats`/`message` are optional. `honeypot_url`
/// feeds the shared signup honeypot guard (a bot that fills the hidden field is
/// silently accepted with no row written).
#[derive(Debug, Deserialize)]
struct EnterpriseInquiryBody {
    company: Option<String>,
    name: Option<String>,
    email: String,
    seats: Option<String>,
    message: Option<String>,
    honeypot_url: Option<String>,
}

/// Match the public enterprise-inquiry POST (with or without a query string).
pub(super) fn is_enterprise_inquiry_request(req: &str) -> bool {
    req.starts_with("POST /api/v1/enterprise-inquiry ")
        || req.starts_with("POST /api/v1/enterprise-inquiry?")
}

/// Handle `POST /api/v1/enterprise-inquiry`. Returns the canonical gateway
/// `(status, body)` tuple. Validates the required fields + email shape, runs
/// the shared signup abuse controls, appends to `enterprise_inquiry`, and
/// writes a hash-chained audit row.
pub(super) async fn enterprise_inquiry_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(s) = store else {
        return super::auth_magic_link::error_resp("500 Internal Server Error", "store_unavailable");
    };
    // Hard per-IP cap (same bucket the magic-link signup + waitlist use).
    let ip = super::auth_magic_link::signup_source_ip(req);
    if let RateLimitIpCheckResult::Deny { retry_after_secs } = rate_limit_ip::signup_limiter().check(ip)
    {
        return (
            "429 Too Many Requests",
            json!({
                "error": "rate_limited",
                "scope": "enterprise_inquiry",
                "retry_after_secs": retry_after_secs,
            })
            .to_string(),
        );
    }
    let parsed: EnterpriseInquiryBody =
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
    // SilentOk semantics of the signup + waitlist honeypot.
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
    let Some(company) = normalize_opt(parsed.company) else {
        return super::auth_magic_link::error_resp("400 Bad Request", "company is required");
    };
    let Some(name) = normalize_opt(parsed.name) else {
        return super::auth_magic_link::error_resp("400 Bad Request", "name is required");
    };
    let seats = normalize_opt(parsed.seats);
    let message = normalize_opt(parsed.message);
    let id = uuid::Uuid::now_v7().to_string();
    let created_at = chrono::Utc::now().timestamp();
    let insert = sqlx::query(
        "INSERT INTO enterprise_inquiry (id, company, name, email, seats, message, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&company)
    .bind(&name)
    .bind(&email)
    .bind(&seats)
    .bind(&message)
    .bind(created_at)
    .execute(s.pool())
    .await;
    if let Err(e) = insert {
        return super::auth_magic_link::error_resp(
            "500 Internal Server Error",
            &format!("store: {e}"),
        );
    }
    // Hash-chained audit breadcrumb. The contact name, seats, and free-text
    // message are intentionally NOT written into the audit chain — the
    // `enterprise_inquiry` row is the lead record of truth; the audit row is the
    // tamper-evident "a lead arrived" marker. Best-effort: a failed audit write
    // is logged but does NOT fail the request (the lead is already persisted).
    write_inquiry_audit(s, &id, &company, &email, message.is_some(), ip).await;
    ("202 Accepted", QUEUED_BODY.to_string())
}

/// Trim an optional string and treat an empty result as absent.
fn normalize_opt(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// Write the hash-chained `EnterpriseInquiryReceived` audit row under the
/// anonymous (pre-tenant) audit tenant. Mirrors `auth_magic_link_audit::
/// write_audit` — best-effort, never panics, logs on failure.
async fn write_inquiry_audit(
    store: &Arc<SessionStore>,
    inquiry_id: &str,
    company: &str,
    email: &str,
    has_message: bool,
    source_ip: std::net::IpAddr,
) {
    let payload = json!({
        "inquiry_id": inquiry_id,
        "company": company,
        "email": email,
        "has_message": has_message,
        "source_ip": source_ip.to_string(),
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), ANONYMOUS_AUDIT_TENANT_ID.to_string());
    if let Err(e) = sink
        .write(None, AuditKind::EnterpriseInquiryReceived, payload, None)
        .await
    {
        tracing::warn!(
            target: "gateway::routes_enterprise_inquiry",
            error = %e,
            "enterprise inquiry audit write failed",
        );
    }
}

#[cfg(test)]
#[path = "routes_enterprise_inquiry_tests.rs"]
mod tests;
