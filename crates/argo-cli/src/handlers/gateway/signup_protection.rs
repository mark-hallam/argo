//! Bot and abuse controls for anonymous magic-link signup.
//!
//! Retry hints are returned in JSON bodies rather than `Retry-After`
//! headers because the gateway route contract is `(status, body)`.
//! This matches the existing per-route and per-tenant limiter
//! deviation; adding arbitrary response headers belongs in the
//! shared response formatter.
//!
//! State is process-local. Gateway restart drops all signup-protection
//! buckets, and live buckets are swept after 24 hours.

use argo_config::{ArgoConfig, AuthConfig};
use dashmap::DashMap;
use serde::Deserialize;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const IP_WINDOW: Duration = Duration::from_secs(60 * 60);
const STATE_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const TURNSTILE_TIMEOUT: Duration = Duration::from_secs(2);

#[cfg(test)]
#[path = "signup_protection_tests.rs"]
mod tests;

pub(super) struct SignupFields<'a> {
    pub email: &'a str,
    pub honeypot_url: Option<&'a str>,
    pub turnstile_response: Option<&'a str>,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) enum SignupProtectionOutcome {
    Allow,
    SilentOk,
    Deny { status: &'static str, body: String },
}

#[derive(Default)]
pub(super) struct SignupProtectionState {
    ip_attempts: DashMap<String, Vec<Instant>>,
    email_attempts: DashMap<String, Instant>,
}

pub(super) async fn check_signup(req: &str, fields: SignupFields<'_>) -> SignupProtectionOutcome {
    let cfg = load_auth_config();
    check_signup_with(
        global_state(),
        req,
        fields,
        &cfg,
        Instant::now(),
        verify_turnstile_http,
    )
    .await
}

pub(super) async fn check_signup_with<F, Fut>(
    state: &SignupProtectionState,
    req: &str,
    fields: SignupFields<'_>,
    cfg: &AuthConfig,
    now: Instant,
    verifier: F,
) -> SignupProtectionOutcome
where
    F: FnOnce(String, String) -> Fut,
    Fut: Future<Output = bool>,
{
    if fields.honeypot_url.is_some_and(non_empty_constant_time) {
        return SignupProtectionOutcome::SilentOk;
    }

    state.sweep_expired(now);

    if let Some(retry) = state.check_ip(req, cfg, now) {
        return deny_rate_limited("ip", retry);
    }

    let turnstile = &cfg.signup_protection.turnstile;
    if turnstile.enabled && !turnstile.secret_key.trim().is_empty() {
        let token = fields.turnstile_response.unwrap_or("").trim();
        if token.is_empty() || !verifier(turnstile.secret_key.clone(), token.to_string()).await {
            return SignupProtectionOutcome::Deny {
                status: "403 Forbidden",
                body: r#"{"error":"signup_protection_failed"}"#.to_string(),
            };
        }
    }

    if let Some(retry) = state.check_email(fields.email, cfg, now) {
        return deny_rate_limited("email", retry);
    }

    SignupProtectionOutcome::Allow
}

fn global_state() -> &'static SignupProtectionState {
    static STATE: OnceLock<SignupProtectionState> = OnceLock::new();
    STATE.get_or_init(SignupProtectionState::default)
}

#[cfg(test)]
fn load_auth_config() -> AuthConfig {
    ArgoConfig::default().auth
}

#[cfg(not(test))]
fn load_auth_config() -> AuthConfig {
    argo_config::load().map(|cfg| cfg.auth).unwrap_or_else(|e| {
        tracing::warn!(
            target: "gateway::signup_protection",
            error = %e,
            "using default signup protection config"
        );
        ArgoConfig::default().auth
    })
}

impl SignupProtectionState {
    fn sweep_expired(&self, now: Instant) {
        self.ip_attempts.retain(|_, attempts| {
            attempts.retain(|t| now.saturating_duration_since(*t) < STATE_TTL);
            !attempts.is_empty()
        });
        self.email_attempts
            .retain(|_, last| now.saturating_duration_since(*last) < STATE_TTL);
    }

    fn check_ip(&self, req: &str, cfg: &AuthConfig, now: Instant) -> Option<u64> {
        let limit = cfg.signup_protection.ip_limit_per_hour;
        if limit == 0 {
            return None;
        }
        let key = signup_ip_key(req, cfg.trust_proxy_header);
        let mut entry = self.ip_attempts.entry(key).or_default();
        entry.retain(|t| now.saturating_duration_since(*t) < IP_WINDOW);
        if entry.len() >= limit as usize {
            let oldest = entry.iter().min().copied().unwrap_or(now);
            return Some(retry_after_secs(now, oldest, IP_WINDOW));
        }
        entry.push(now);
        None
    }

    fn check_email(&self, email: &str, cfg: &AuthConfig, now: Instant) -> Option<u64> {
        let cooldown = Duration::from_secs(cfg.signup_protection.email_cooldown_secs);
        if cooldown.is_zero() {
            return None;
        }
        let key = email.trim().to_ascii_lowercase();
        if let Some(last) = self.email_attempts.get(&key) {
            let elapsed = now.saturating_duration_since(*last);
            if elapsed < cooldown {
                return Some(retry_after_secs(now, *last, cooldown));
            }
        }
        self.email_attempts.insert(key, now);
        None
    }
}

fn retry_after_secs(now: Instant, since: Instant, window: Duration) -> u64 {
    window
        .saturating_sub(now.saturating_duration_since(since))
        .as_secs()
        .max(1)
}

fn deny_rate_limited(scope: &str, retry_after_secs: u64) -> SignupProtectionOutcome {
    SignupProtectionOutcome::Deny {
        status: "429 Too Many Requests",
        body: format!(
            "{{\"error\":\"rate_limited\",\"scope\":\"{scope}\",\"retry_after_secs\":{retry_after_secs},\"retry_after\":{retry_after_secs}}}"
        ),
    }
}

fn signup_ip_key(req: &str, trust_proxy_header: bool) -> String {
    let ip = if trust_proxy_header {
        parse_first_xff_hop(req).unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
    } else {
        IpAddr::V4(Ipv4Addr::UNSPECIFIED)
    };
    normalize_ip_key(ip)
}

fn parse_first_xff_hop(req: &str) -> Option<IpAddr> {
    let headers = req.split_once("\r\n\r\n").map_or(req, |(h, _)| h);
    for line in headers.split("\r\n").skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if !name.trim().eq_ignore_ascii_case("x-forwarded-for") {
            continue;
        }
        return value.split(',').next()?.trim().parse().ok();
    }
    None
}

fn normalize_ip_key(ip: IpAddr) -> String {
    match ip {
        IpAddr::V4(v4) => v4.to_string(),
        IpAddr::V6(v6) => {
            let s = v6.segments();
            format!("{:x}:{:x}:{:x}:{:x}::/64", s[0], s[1], s[2], s[3])
        }
    }
}

fn non_empty_constant_time(value: &str) -> bool {
    let mut seen = 0u8;
    for b in value.as_bytes() {
        seen |= *b;
    }
    !value.is_empty() && seen != 0
}

async fn verify_turnstile_http(secret: String, token: String) -> bool {
    #[derive(Deserialize)]
    struct SiteVerify {
        success: bool,
    }

    let Ok(client) = reqwest::Client::builder()
        .timeout(TURNSTILE_TIMEOUT)
        .build()
    else {
        return false;
    };
    let resp = client
        .post("https://challenges.cloudflare.com/turnstile/v0/siteverify")
        .form(&[("secret", secret.as_str()), ("response", token.as_str())])
        .send()
        .await;
    let Ok(resp) = resp else {
        return false;
    };
    resp.json::<SiteVerify>()
        .await
        .map(|body| body.success)
        .unwrap_or(false)
}
