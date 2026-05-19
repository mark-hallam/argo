//! Overrides bookkeeping for the layered config loader.
//!
//! Companion to [`crate::loader`]. Carries the mask that drives
//! precedence-aware merging plus the [`ArgoConfig::from_toml_with_overrides`]
//! constructor that populates the mask from a TOML body.
use crate::ArgoConfig;
use argo_types::ArgoResult;

/// Bookkeeping for which fields a particular [`ArgoConfig`] layer
/// explicitly set, so [`ArgoConfig::merge`] can do a precedence-aware
/// overlay without nuking unrelated defaults.
///
/// Toml-deserialized layers leave this all-`false` (every field came
/// from `#[serde(default)]`), which would normally make merge a
/// no-op. We special-case TOML in [`ArgoConfig::from_toml_with_overrides`]
/// by setting the mask post-parse for the fields that were actually
/// present in the source — see that method for details.
#[derive(Debug, Clone, Default)]
pub(crate) struct ArgoConfigOverrides {
    pub auth_trust_proxy_header: bool,
    pub auth_signup_ip_limit_per_hour: bool,
    pub auth_signup_email_cooldown_secs: bool,
    pub auth_signup_turnstile_enabled: bool,
    pub auth_signup_turnstile_secret_key: bool,
    pub core_home: bool,
    pub core_profile: bool,
    pub tenant_default_id: bool,
    pub telemetry_enabled: bool,
    pub telemetry_exporter: bool,
    pub telemetry_endpoint: bool,
    pub error_reporting_mode: bool,
    pub error_reporting_dsn: bool,
    pub error_reporting_otlp_endpoint: bool,
    pub error_reporting_pseudonym_salt: bool,
    pub gateway_host: bool,
    pub gateway_port: bool,
    pub gateway_url: bool,
    pub gateway_duty_scheduler_secs: bool,
    pub gateway_handoff_sweep_secs: bool,
    pub gateway_auth_bypass: bool,
    pub gateway_rate_limit_ip_enabled: bool,
    pub gateway_rate_limit_ip_capacity: bool,
    pub gateway_rate_limit_ip_refill_per_sec: bool,
    pub gateway_rate_limit_ip_trust_xff: bool,
    pub gateway_rate_limit_route_default_rpm: bool,
    pub gateway_rate_limit_route_forge_run_rpm: bool,
    pub gateway_rate_limit_route_skill_execute_rpm: bool,
    pub gateway_rate_limit_tenant_rpm: bool,
    pub channels_webinbox_tenant_id: bool,
    pub gateway_security_headers_csp_report_only: bool,
}

impl ArgoConfigOverrides {
    pub(crate) fn combine(&mut self, other: &Self) {
        self.auth_trust_proxy_header |= other.auth_trust_proxy_header;
        self.auth_signup_ip_limit_per_hour |= other.auth_signup_ip_limit_per_hour;
        self.auth_signup_email_cooldown_secs |= other.auth_signup_email_cooldown_secs;
        self.auth_signup_turnstile_enabled |= other.auth_signup_turnstile_enabled;
        self.auth_signup_turnstile_secret_key |= other.auth_signup_turnstile_secret_key;
        self.core_home |= other.core_home;
        self.core_profile |= other.core_profile;
        self.tenant_default_id |= other.tenant_default_id;
        self.telemetry_enabled |= other.telemetry_enabled;
        self.telemetry_exporter |= other.telemetry_exporter;
        self.telemetry_endpoint |= other.telemetry_endpoint;
        self.error_reporting_mode |= other.error_reporting_mode;
        self.error_reporting_dsn |= other.error_reporting_dsn;
        self.error_reporting_otlp_endpoint |= other.error_reporting_otlp_endpoint;
        self.error_reporting_pseudonym_salt |= other.error_reporting_pseudonym_salt;
        self.gateway_host |= other.gateway_host;
        self.gateway_port |= other.gateway_port;
        self.gateway_url |= other.gateway_url;
        self.gateway_duty_scheduler_secs |= other.gateway_duty_scheduler_secs;
        self.gateway_handoff_sweep_secs |= other.gateway_handoff_sweep_secs;
        self.gateway_auth_bypass |= other.gateway_auth_bypass;
        self.gateway_rate_limit_ip_enabled |= other.gateway_rate_limit_ip_enabled;
        self.gateway_rate_limit_ip_capacity |= other.gateway_rate_limit_ip_capacity;
        self.gateway_rate_limit_ip_refill_per_sec |= other.gateway_rate_limit_ip_refill_per_sec;
        self.gateway_rate_limit_ip_trust_xff |= other.gateway_rate_limit_ip_trust_xff;
        self.gateway_rate_limit_route_default_rpm |= other.gateway_rate_limit_route_default_rpm;
        self.gateway_rate_limit_route_forge_run_rpm |= other.gateway_rate_limit_route_forge_run_rpm;
        self.gateway_rate_limit_route_skill_execute_rpm |=
            other.gateway_rate_limit_route_skill_execute_rpm;
        self.gateway_rate_limit_tenant_rpm |= other.gateway_rate_limit_tenant_rpm;
        self.channels_webinbox_tenant_id |= other.channels_webinbox_tenant_id;
        self.gateway_security_headers_csp_report_only |=
            other.gateway_security_headers_csp_report_only;
    }

    /// Mark every field as overridden — used when a TOML layer is
    /// loaded in full and the operator's intent is "these are my
    /// values for everything I wrote in the file."
    pub(crate) fn mark_all_from_toml(&mut self, raw: &str) {
        // Only mark fields whose section appears in the raw TOML.
        // Coarse but cheap; works because the keys are unique.
        self.auth_trust_proxy_header |=
            raw.contains("[auth]") && raw.contains("trust_proxy_header");
        self.auth_signup_ip_limit_per_hour |=
            raw.contains("[auth.signup_protection]") && raw.contains("ip_limit_per_hour");
        self.auth_signup_email_cooldown_secs |=
            raw.contains("[auth.signup_protection]") && raw.contains("email_cooldown_secs");
        self.auth_signup_turnstile_enabled |=
            raw.contains("[auth.signup_protection.turnstile]") && raw.contains("enabled");
        self.auth_signup_turnstile_secret_key |=
            raw.contains("[auth.signup_protection.turnstile]") && raw.contains("secret_key");
        self.core_home |= raw.contains("[core]") && raw.contains("home");
        self.core_profile |= raw.contains("[core]") && raw.contains("profile");
        self.tenant_default_id |= raw.contains("[tenant]") && raw.contains("default_id");
        self.telemetry_enabled |= raw.contains("[telemetry]") && raw.contains("enabled");
        self.telemetry_exporter |= raw.contains("[telemetry]") && raw.contains("exporter");
        self.telemetry_endpoint |= raw.contains("[telemetry]") && raw.contains("endpoint");
        self.error_reporting_mode |= raw.contains("[error_reporting]") && raw.contains("mode");
        self.error_reporting_dsn |= raw.contains("[error_reporting]") && raw.contains("dsn");
        self.error_reporting_otlp_endpoint |=
            raw.contains("[error_reporting]") && raw.contains("otlp_endpoint");
        self.error_reporting_pseudonym_salt |=
            raw.contains("[error_reporting]") && raw.contains("pseudonym_salt");
        self.gateway_host |= raw.contains("[gateway]") && raw.contains("host");
        self.gateway_port |= raw.contains("[gateway]") && raw.contains("port");
        self.gateway_url |= raw.contains("[gateway]") && raw.contains("url");
        self.gateway_duty_scheduler_secs |=
            raw.contains("[gateway]") && raw.contains("duty_scheduler_secs");
        self.gateway_handoff_sweep_secs |=
            raw.contains("[gateway]") && raw.contains("handoff_sweep_secs");
        self.gateway_auth_bypass |= raw.contains("[gateway]") && raw.contains("auth_bypass");
        self.gateway_rate_limit_ip_enabled |=
            raw.contains("[gateway.rate_limit.ip]") && raw.contains("enabled");
        self.gateway_rate_limit_ip_capacity |=
            raw.contains("[gateway.rate_limit.ip]") && raw.contains("capacity");
        self.gateway_rate_limit_ip_refill_per_sec |=
            raw.contains("[gateway.rate_limit.ip]") && raw.contains("refill_per_sec");
        self.gateway_rate_limit_ip_trust_xff |=
            raw.contains("[gateway.rate_limit.ip]") && raw.contains("trust_xff");
        self.gateway_rate_limit_route_default_rpm |=
            raw.contains("[gateway.rate_limit.route]") && raw.contains("default_rpm");
        self.gateway_rate_limit_route_forge_run_rpm |=
            raw.contains("[gateway.rate_limit.route.overrides]") && raw.contains("forge.run");
        self.gateway_rate_limit_route_skill_execute_rpm |=
            raw.contains("[gateway.rate_limit.route.overrides]") && raw.contains("skill.execute");
        self.gateway_rate_limit_tenant_rpm |=
            raw.contains("[gateway.rate_limit.tenant]") && raw.contains("requests_per_minute");
        self.channels_webinbox_tenant_id |=
            raw.contains("[channels.webinbox]") && raw.contains("tenant_id");
        self.gateway_security_headers_csp_report_only |=
            raw.contains("[gateway.security_headers]") && raw.contains("csp_report_only");
    }
}

impl ArgoConfig {
    /// Variant of [`from_toml`](ArgoConfig::from_toml) that records
    /// which fields the operator set in the TOML body so subsequent
    /// merges respect their intent.
    ///
    /// # Errors
    /// Returns [`argo_types::ArgoError::Provider`] on parse failure.
    pub fn from_toml_with_overrides(s: &str) -> ArgoResult<Self> {
        let mut cfg = Self::from_toml(s)?;
        cfg.overrides.mark_all_from_toml(s);
        Ok(cfg)
    }
}
