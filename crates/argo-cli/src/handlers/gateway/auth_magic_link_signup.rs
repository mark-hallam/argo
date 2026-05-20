use super::{deliver_magic_link, error_resp, looks_like_email, mint_token, sha256_hex};
use super::{signup_source_ip, SessionStore};
use crate::handlers::gateway::rate_limit_ip::{self, RateLimitIpCheckResult};
use crate::handlers::gateway::signup_protection::{SignupFields, SignupProtectionOutcome};
use argo_store::user_account;
use chrono::Utc;
use serde::Deserialize;
use std::sync::Arc;

const SIGNUP_SENT_BODY: &str = r#"{"status":"sent"}"#;

#[derive(Debug, Deserialize)]
struct SignupBody {
    email: String,
    #[serde(rename = "cf-turnstile-response")]
    cf_turnstile_response: Option<String>,
    honeypot_url: Option<String>,
}

pub(crate) async fn signup_response(
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
    let parsed: SignupBody =
        match serde_json::from_str(crate::handlers::gateway::body_from_request(req)) {
            Ok(v) => v,
            Err(e) => return error_resp("400 Bad Request", &format!("invalid body: {e}")),
        };
    let email = parsed.email.trim().to_ascii_lowercase();
    if !looks_like_email(&email) {
        return error_resp("400 Bad Request", "email is required");
    }
    let fields = SignupFields {
        email: &email,
        honeypot_url: parsed.honeypot_url.as_deref(),
        turnstile_response: parsed.cf_turnstile_response.as_deref(),
    };
    match crate::handlers::gateway::signup_protection::check_signup(req, fields).await {
        SignupProtectionOutcome::Allow => {}
        SignupProtectionOutcome::SilentOk => return ("200 OK", SIGNUP_SENT_BODY.to_string()),
        SignupProtectionOutcome::Deny { status, body } => return (status, body),
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
    ("200 OK", SIGNUP_SENT_BODY.to_string())
}
