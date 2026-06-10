//! Tests for signup-specific bot protection.

use super::*;
use argo_config::{AuthSignupProtectionConfig, AuthTurnstileConfig};

/// The trusted `src_ip` the accept loop (`proxy_guard::extract_real_ip`)
/// hands the limiter. It is NEVER derived from a client-supplied
/// `X-Forwarded-For` — the limiter no longer sees the request string at
/// all (fix-signup-protection-xff-trust-and-default).
fn src(ip: &str) -> IpAddr {
    ip.parse().expect("valid test ip")
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
    let src_ip = src("203.0.113.20");

    for i in 0..cfg.signup_protection.ip_limit_per_hour {
        let email = format!("user{i}@example.com");
        let out = check_signup_with(&state, src_ip, fields(&email), &cfg, now, |_, _| async {
            false
        })
        .await;
        assert_eq!(out, SignupProtectionOutcome::Allow);
    }

    let out = check_signup_with(
        &state,
        src_ip,
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

/// Acceptance #1: the per-IP hourly cap keys on the trusted `src_ip`, so
/// requests sharing one `src_ip` share one bucket no matter what (forged)
/// `X-Forwarded-For` a client sends. The limiter cannot even observe the
/// header — the request string is no longer a parameter — so this is
/// structurally unspoofable.
#[tokio::test]
async fn ip_cap_keys_on_trusted_src_ip_not_forged_xff() {
    let state = SignupProtectionState::default();
    let mut cfg = cfg();
    cfg.signup_protection.ip_limit_per_hour = 1;
    cfg.signup_protection.email_cooldown_secs = 0;
    let now = Instant::now();
    let attacker = src("198.51.100.7");

    // First request from the trusted peer is allowed.
    let first = check_signup_with(
        &state,
        attacker,
        fields("a@example.com"),
        &cfg,
        now,
        |_, _| async { false },
    )
    .await;
    assert_eq!(first, SignupProtectionOutcome::Allow);

    // A second request from the SAME trusted peer (a real attacker would
    // be rotating X-Forwarded-For here to mint a fresh bucket) lands in
    // the same bucket and is rate-limited.
    let second = check_signup_with(
        &state,
        attacker,
        fields("b@example.com"),
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

    // A genuinely different trusted peer gets its own bucket.
    let other = check_signup_with(
        &state,
        src("198.51.100.8"),
        fields("c@example.com"),
        &cfg,
        now,
        |_, _| async { false },
    )
    .await;
    assert_eq!(other, SignupProtectionOutcome::Allow);
}

#[tokio::test]
async fn email_cooldown_is_case_insensitive() {
    let state = SignupProtectionState::default();
    let mut cfg = cfg();
    cfg.signup_protection.ip_limit_per_hour = 0;
    let now = Instant::now();

    let first = check_signup_with(
        &state,
        src("203.0.113.21"),
        fields("Mark@Example.COM"),
        &cfg,
        now,
        |_, _| async { false },
    )
    .await;
    assert_eq!(first, SignupProtectionOutcome::Allow);

    let second = check_signup_with(
        &state,
        src("203.0.113.22"),
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
    let out = check_signup_with(&state, src("203.0.113.23"), bot, &cfg, now, |_, _| async {
        false
    })
    .await;
    assert_eq!(out, SignupProtectionOutcome::SilentOk);

    let human = check_signup_with(
        &state,
        src("203.0.113.23"),
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
        src("203.0.113.24"),
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
        src("203.0.113.24"),
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
        src("2001:db8::1"),
        fields("first@example.com"),
        &cfg,
        now,
        |_, _| async { false },
    )
    .await;
    assert_eq!(first, SignupProtectionOutcome::Allow);

    let second = check_signup_with(
        &state,
        src("2001:db8::2"),
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
