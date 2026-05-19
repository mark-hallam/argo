//! Tests for signup-specific bot protection.

use super::*;
use argo_config::{AuthSignupProtectionConfig, AuthTurnstileConfig};

fn req(ip: &str) -> String {
    format!("POST /api/v1/auth/signup HTTP/1.1\r\nX-Forwarded-For: {ip}\r\n\r\n")
}

fn fields(email: &str) -> SignupFields<'_> {
    SignupFields {
        email,
        honeypot_url: None,
        turnstile_response: None,
    }
}

fn cfg() -> AuthConfig {
    AuthConfig::default()
}

#[tokio::test]
async fn sixth_ip_attempt_in_hour_is_rate_limited() {
    let state = SignupProtectionState::default();
    let mut cfg = cfg();
    cfg.signup_protection.email_cooldown_secs = 0;
    let now = Instant::now();
    let request = req("203.0.113.20");

    for i in 0..cfg.signup_protection.ip_limit_per_hour {
        let email = format!("user{i}@example.com");
        let out = check_signup_with(&state, &request, fields(&email), &cfg, now, |_, _| async {
            false
        })
        .await;
        assert_eq!(out, SignupProtectionOutcome::Allow);
    }

    let out = check_signup_with(
        &state,
        &request,
        fields("blocked@example.com"),
        &cfg,
        now,
        |_, _| async { false },
    )
    .await;
    assert!(matches!(
        out,
        SignupProtectionOutcome::Deny {
            status: "429 Too Many Requests",
            ..
        }
    ));
}

#[tokio::test]
async fn email_cooldown_is_case_insensitive() {
    let state = SignupProtectionState::default();
    let mut cfg = cfg();
    cfg.signup_protection.ip_limit_per_hour = 0;
    let now = Instant::now();

    let first = check_signup_with(
        &state,
        &req("203.0.113.21"),
        fields("Mark@Example.COM"),
        &cfg,
        now,
        |_, _| async { false },
    )
    .await;
    assert_eq!(first, SignupProtectionOutcome::Allow);

    let second = check_signup_with(
        &state,
        &req("203.0.113.22"),
        fields("mark@example.com"),
        &cfg,
        now + Duration::from_secs(10),
        |_, _| async { false },
    )
    .await;
    assert!(matches!(
        second,
        SignupProtectionOutcome::Deny {
            status: "429 Too Many Requests",
            ..
        }
    ));
}

#[tokio::test]
async fn honeypot_short_circuits_without_consuming_cooldown() {
    let state = SignupProtectionState::default();
    let mut cfg = cfg();
    cfg.signup_protection.ip_limit_per_hour = 0;
    let now = Instant::now();
    let bot = SignupFields {
        email: "bot@example.com",
        honeypot_url: Some("https://spam.invalid"),
        turnstile_response: None,
    };
    let out = check_signup_with(&state, &req("203.0.113.23"), bot, &cfg, now, |_, _| async {
        false
    })
    .await;
    assert_eq!(out, SignupProtectionOutcome::SilentOk);

    let human = check_signup_with(
        &state,
        &req("203.0.113.23"),
        fields("bot@example.com"),
        &cfg,
        now,
        |_, _| async { false },
    )
    .await;
    assert_eq!(human, SignupProtectionOutcome::Allow);
}

#[tokio::test]
async fn turnstile_valid_token_allows_and_invalid_token_forbids() {
    let state = SignupProtectionState::default();
    let mut cfg = cfg();
    cfg.signup_protection = AuthSignupProtectionConfig {
        ip_limit_per_hour: 0,
        email_cooldown_secs: 0,
        turnstile: AuthTurnstileConfig {
            enabled: true,
            secret_key: "secret".into(),
        },
    };
    let good = SignupFields {
        email: "good@example.com",
        honeypot_url: None,
        turnstile_response: Some("good"),
    };
    let out = check_signup_with(
        &state,
        &req("203.0.113.24"),
        good,
        &cfg,
        Instant::now(),
        |secret, token| std::future::ready(secret == "secret" && token == "good"),
    )
    .await;
    assert_eq!(out, SignupProtectionOutcome::Allow);

    let bad = SignupFields {
        email: "bad@example.com",
        honeypot_url: None,
        turnstile_response: Some("bad"),
    };
    let out = check_signup_with(
        &state,
        &req("203.0.113.24"),
        bad,
        &cfg,
        Instant::now(),
        |_, _| async { false },
    )
    .await;
    assert_eq!(
        out,
        SignupProtectionOutcome::Deny {
            status: "403 Forbidden",
            body: r#"{"error":"signup_protection_failed"}"#.to_string()
        }
    );
}

#[tokio::test]
async fn ipv6_addresses_share_64_bucket() {
    let state = SignupProtectionState::default();
    let mut cfg = cfg();
    cfg.signup_protection.ip_limit_per_hour = 1;
    cfg.signup_protection.email_cooldown_secs = 0;
    let now = Instant::now();

    let first = check_signup_with(
        &state,
        &req("2001:db8::1"),
        fields("first@example.com"),
        &cfg,
        now,
        |_, _| async { false },
    )
    .await;
    assert_eq!(first, SignupProtectionOutcome::Allow);

    let second = check_signup_with(
        &state,
        &req("2001:db8::2"),
        fields("second@example.com"),
        &cfg,
        now,
        |_, _| async { false },
    )
    .await;
    assert!(matches!(
        second,
        SignupProtectionOutcome::Deny {
            status: "429 Too Many Requests",
            ..
        }
    ));
}
