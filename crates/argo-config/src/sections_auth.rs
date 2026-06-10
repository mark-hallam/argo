//! Authentication config sections.

use serde::{Deserialize, Serialize};

/// `[auth]` — public authentication surface controls.
///
/// # Examples
///
/// ```rust
/// use argo_config::AuthConfig;
///
/// let cfg = AuthConfig::default();
/// assert_eq!(cfg.signup_protection.ip_limit_per_hour, 5);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    /// Legacy footgun flag, now vestigial. Default `false`
    /// (secure-by-default). It once let the signup limiter trust the
    /// first `X-Forwarded-For` hop unconditionally; that limiter now
    /// keys on the trusted `src_ip` the accept loop extracts
    /// (`proxy_guard::extract_real_ip`), so nothing consumes this flag.
    /// The canonical proxy-trust contract is `[gateway.behind_proxy]`
    /// (`expected_proxy` + `trusted_proxies`). Slated for removal as a
    /// parallel abstraction (fix-signup-protection-xff-trust-and-default).
    pub trust_proxy_header: bool,
    /// `[auth.signup_protection]` bot and abuse controls.
    pub signup_protection: AuthSignupProtectionConfig,
}

/// `[auth.signup_protection]` — public signup abuse controls.
///
/// # Examples
///
/// ```rust
/// use argo_config::AuthSignupProtectionConfig;
///
/// let cfg = AuthSignupProtectionConfig::default();
/// assert_eq!(cfg.email_cooldown_secs, 60);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthSignupProtectionConfig {
    /// Per-IP signup attempts allowed per rolling hour. `0` disables
    /// the IP limiter. Default `5`.
    pub ip_limit_per_hour: u32,
    /// Per-email cooldown after a magic-link request. `0` disables
    /// the email limiter. Default `60`.
    pub email_cooldown_secs: u64,
    /// Cloudflare Turnstile verification settings.
    pub turnstile: AuthTurnstileConfig,
}

impl Default for AuthSignupProtectionConfig {
    fn default() -> Self {
        Self {
            ip_limit_per_hour: 5,
            email_cooldown_secs: 60,
            turnstile: AuthTurnstileConfig::default(),
        }
    }
}

/// `[auth.signup_protection.turnstile]` — optional Cloudflare
/// Turnstile verification.
///
/// # Examples
///
/// ```rust
/// use argo_config::AuthTurnstileConfig;
///
/// let cfg = AuthTurnstileConfig::default();
/// assert!(!cfg.enabled);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthTurnstileConfig {
    /// Enables Turnstile verification when paired with a non-empty
    /// [`secret_key`](Self::secret_key).
    pub enabled: bool,
    /// Cloudflare Turnstile secret key.
    pub secret_key: String,
}

#[cfg(test)]
mod tests {
    #[test]
    fn auth_signup_protection_toml_parses() {
        let cfg = crate::ArgoConfig::from_toml(
            r#"
[auth]
trust_proxy_header = false

[auth.signup_protection]
ip_limit_per_hour = 9
email_cooldown_secs = 15

[auth.signup_protection.turnstile]
enabled = true
secret_key = "secret"
"#,
        )
        .unwrap();

        assert!(!cfg.auth.trust_proxy_header);
        assert_eq!(cfg.auth.signup_protection.ip_limit_per_hour, 9);
        assert_eq!(cfg.auth.signup_protection.email_cooldown_secs, 15);
        assert!(cfg.auth.signup_protection.turnstile.enabled);
        assert_eq!(cfg.auth.signup_protection.turnstile.secret_key, "secret");
    }

    #[test]
    fn layered_loader_preserves_auth_overrides() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("argo.toml"),
            "[auth.signup_protection]\nip_limit_per_hour = 11\n",
        )
        .expect("write config");

        let cfg = crate::load_from(dir.path()).expect("load");
        assert_eq!(cfg.auth.signup_protection.ip_limit_per_hour, 11);
        assert_eq!(cfg.auth.signup_protection.email_cooldown_secs, 60);
    }
}
