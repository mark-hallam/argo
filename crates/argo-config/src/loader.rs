//! Layered config loader — single source of truth for
//! workspace-level config across the argo binary.
//!
//! # Precedence (highest wins)
//!
//! 1. **Environment variables** (`ARGO_*`)
//! 2. **Project-local TOML** — `.skycot/argo.toml` (preferred) or
//!    `argo.toml` (legacy) under the workspace cwd
//! 3. **User-level TOML** — `$XDG_CONFIG_HOME/argo/argo.toml`
//!    (fallback `~/.config/argo/argo.toml`)
//! 4. **Built-in defaults** — see [`crate::defaults`]
//!
//! Handlers should call [`load`] once at the entry point and pass
//! the resulting [`ArgoConfig`] (or a `&` slice of it) downward.
//! Direct `std::env::var("ARGO_*")` reads outside this crate are
//! flagged by `scripts/check-no-direct-env-reads.sh`.
use crate::loader_env::{
    env_bool, env_bool_not_zero, env_bool_one, env_f64_positive_lossy, env_string, env_u16,
    env_u32_lossy, env_u32_positive_lossy, env_u64,
};
use crate::{defaults, ArgoConfig};
use argo_types::{ArgoError, ArgoResult};
use std::path::{Path, PathBuf};

// Re-exported so `loader::ArgoConfigOverrides` (referenced from `lib.rs`)
// keeps resolving after the sibling-file split.
pub(crate) use crate::loader_overrides::ArgoConfigOverrides;

/// Load the effective [`ArgoConfig`] using the layered precedence
/// described in the [crate-level docs](self).
///
/// The cwd is determined via [`std::env::current_dir`]. For
/// deterministic loading in tests, prefer [`load_from`].
///
/// # Errors
/// Returns [`ArgoError::Provider`] on read or parse failure.
///
/// # Example
/// ```no_run
/// let cfg = argo_config::load().unwrap();
/// println!("gateway port = {}", cfg.gateway.port);
/// ```
pub fn load() -> ArgoResult<ArgoConfig> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    load_from(&cwd)
}

/// Layered load with `cwd` as the project root. Tests pass a
/// scratch directory here to avoid leaking the developer's real
/// `~/.config/argo/argo.toml`.
///
/// # Errors
/// Returns [`ArgoError::Provider`] on read or parse failure.
pub fn load_from(cwd: &Path) -> ArgoResult<ArgoConfig> {
    // Layer 1 (lowest): built-in defaults.
    let mut cfg = ArgoConfig::default();

    // Layer 2: user-level TOML.
    if let Some(user_path) = user_config_path() {
        if user_path.exists() {
            let user_cfg = ArgoConfig::load_from_file(&user_path)?;
            cfg.merge(user_cfg);
        }
    }

    // Layer 3: project-local TOML.
    if let Some(local_path) = project_local_path(cwd) {
        let local_cfg = ArgoConfig::load_from_file(&local_path)?;
        cfg.merge(local_cfg);
    }

    // Layer 4 (highest): environment variables.
    let env_cfg = ArgoConfig::from_env()?;
    cfg.merge(env_cfg);

    Ok(cfg)
}

/// Resolve the project-local config path under `cwd`. Prefers
/// `.skycot/argo.toml` (forward-compatible Skycot rebrand) and
/// falls back to `argo.toml` at the workspace root.
fn project_local_path(cwd: &Path) -> Option<PathBuf> {
    let skycot = cwd.join(defaults::PROJECT_LOCAL_SKYCOT_CONFIG);
    if skycot.exists() {
        return Some(skycot);
    }
    let legacy = cwd.join(defaults::PROJECT_LOCAL_CONFIG);
    if legacy.exists() {
        return Some(legacy);
    }
    None
}

/// Resolve `$XDG_CONFIG_HOME/argo/argo.toml`, falling back to
/// `$HOME/.config/argo/argo.toml`. Returns `None` when neither
/// `XDG_CONFIG_HOME` nor `HOME` is set.
fn user_config_path() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg).join(defaults::USER_CONFIG_RELATIVE));
    }
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join(".config")
            .join(defaults::USER_CONFIG_RELATIVE),
    )
}

impl ArgoConfig {
    /// Read and parse a TOML file with overrides bookkeeping populated.
    /// Missing file ⇒ defaults (no overrides marked).
    fn load_from_file(path: &Path) -> ArgoResult<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let body = std::fs::read_to_string(path)
            .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
        Self::from_toml_with_overrides(&body)
            .map_err(|e| ArgoError::Provider(format!("parse {}: {e}", path.display())))
    }

    /// Parse a TOML body into an [`ArgoConfig`].
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] on parse failure.
    ///
    /// ```
    /// use argo_config::ArgoConfig;
    /// let cfg = ArgoConfig::from_toml("[gateway]\nport = 7777\n").unwrap();
    /// assert_eq!(cfg.gateway.port, 7777);
    /// assert!(ArgoConfig::from_toml("not = [valid").is_err());
    /// ```
    pub fn from_toml(s: &str) -> ArgoResult<Self> {
        toml::from_str(s).map_err(|e| ArgoError::Provider(format!("parse: {e}")))
    }

    /// Build an [`ArgoConfig`] populated only with the env-var
    /// overrides currently set. Fields not bound to an env var are
    /// left at their `Default` so [`merge`](ArgoConfig::merge) only
    /// overwrites the entries the operator actually exported.
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] when an env var is set but
    /// fails to parse (e.g. non-numeric `ARGO_GATEWAY_PORT`).
    pub fn from_env() -> ArgoResult<Self> {
        let mut cfg = Self::default();
        let mut overrides = ArgoConfigOverrides::default();
        macro_rules! set {
            ($flag:ident, $target:expr, $value:expr) => {
                if let Some(v) = $value {
                    $target = v;
                    overrides.$flag = true;
                }
            };
        }

        set!(core_home, cfg.core.home, env_string("ARGO_HOME"));
        set!(core_profile, cfg.core.profile, env_string("ARGO_PROFILE"));
        set!(
            tenant_default_id,
            cfg.tenant.default_id,
            env_string("ARGO_DEFAULT_TENANT")
        );
        set!(
            telemetry_exporter,
            cfg.telemetry.exporter,
            env_string("ARGO_TELEMETRY_EXPORTER")
        );
        set!(
            telemetry_endpoint,
            cfg.telemetry.endpoint,
            env_string("ARGO_TELEMETRY_ENDPOINT")
        );
        set!(
            telemetry_enabled,
            cfg.telemetry.enabled,
            env_bool("ARGO_TELEMETRY_ENABLED")?
        );
        set!(
            gateway_port,
            cfg.gateway.port,
            env_u16("ARGO_GATEWAY_PORT")?
        );
        set!(gateway_url, cfg.gateway.url, env_string("ARGO_GATEWAY_URL"));
        set!(
            gateway_host,
            cfg.gateway.host,
            env_string("ARGO_GATEWAY_HOST")
        );
        set!(
            gateway_duty_scheduler_secs,
            cfg.gateway.duty_scheduler_secs,
            env_u64("ARGO_DUTY_SCHEDULER_SECS")?
        );
        set!(
            gateway_handoff_sweep_secs,
            cfg.gateway.handoff_sweep_secs,
            env_u64("ARGO_HANDOFF_SWEEP_SECS")?
        );
        set!(
            gateway_auth_bypass,
            cfg.gateway.auth_bypass,
            env_bool_one("ARGO_GATEWAY_AUTH_BYPASS")
        );
        set!(
            gateway_rate_limit_ip_enabled,
            cfg.gateway.rate_limit.ip.enabled,
            env_bool_not_zero("ARGO_IP_RATE_LIMIT")
        );
        set!(
            gateway_rate_limit_ip_capacity,
            cfg.gateway.rate_limit.ip.capacity,
            env_u32_positive_lossy("ARGO_IP_RATE_CAPACITY")
        );
        set!(
            gateway_rate_limit_ip_refill_per_sec,
            cfg.gateway.rate_limit.ip.refill_per_sec,
            env_f64_positive_lossy("ARGO_IP_RATE_REFILL")
        );
        set!(
            gateway_rate_limit_ip_trust_xff,
            cfg.gateway.rate_limit.ip.trust_xff,
            env_bool_one("ARGO_TRUST_XFF")
        );
        set!(
            gateway_rate_limit_route_default_rpm,
            cfg.gateway.rate_limit.route.default_rpm,
            env_u32_positive_lossy("ARGO_RATE_LIMIT_DEFAULT_RPM")
        );
        if let Some(v) = env_u32_positive_lossy("ARGO_RATE_LIMIT_FORGE_RUN_RPM") {
            cfg.gateway
                .rate_limit
                .route
                .overrides
                .insert("forge.run".into(), v);
            overrides.gateway_rate_limit_route_forge_run_rpm = true;
        }
        if let Some(v) = env_u32_positive_lossy("ARGO_RATE_LIMIT_SKILL_EXECUTE_RPM") {
            cfg.gateway
                .rate_limit
                .route
                .overrides
                .insert("skill.execute".into(), v);
            overrides.gateway_rate_limit_route_skill_execute_rpm = true;
        }
        set!(
            gateway_rate_limit_tenant_rpm,
            cfg.gateway.rate_limit.tenant.requests_per_minute,
            env_u32_lossy("ARGO_RATE_LIMIT_TENANT_PER_MIN")
        );
        set!(
            channels_webinbox_tenant_id,
            cfg.channels.webinbox.tenant_id,
            env_string("ARGO_WEBINBOX_TENANT_ID").map(Some)
        );
        set!(
            gateway_security_headers_csp_report_only,
            cfg.gateway.security_headers.csp_report_only,
            env_bool_one("ARGO_CSP_REPORT_ONLY")
        );

        cfg.overrides = overrides;
        Ok(cfg)
    }

    /// Merge `other` into `self`, overwriting only the fields
    /// `other` explicitly set (per its `overrides` mask). Used by
    /// [`load_from`] to layer env > project-local > user-level >
    /// defaults.
    ///
    /// ```
    /// use argo_config::ArgoConfig;
    /// let mut base = ArgoConfig::from_toml_with_overrides("[gateway]\nport = 7000\n").unwrap();
    /// let layer = ArgoConfig::from_toml_with_overrides("[gateway]\nport = 9000\n").unwrap();
    /// base.merge(layer);
    /// assert_eq!(base.gateway.port, 9000);
    /// ```
    pub fn merge(&mut self, other: Self) {
        let o = &other.overrides;
        let forge_run_rpm = other
            .gateway
            .rate_limit
            .route
            .overrides
            .get("forge.run")
            .copied();
        let skill_execute_rpm = other
            .gateway
            .rate_limit
            .route
            .overrides
            .get("skill.execute")
            .copied();
        if o.auth_trust_proxy_header {
            self.auth.trust_proxy_header = other.auth.trust_proxy_header;
        }
        if o.auth_signup_ip_limit_per_hour {
            self.auth.signup_protection.ip_limit_per_hour =
                other.auth.signup_protection.ip_limit_per_hour;
        }
        if o.auth_signup_email_cooldown_secs {
            self.auth.signup_protection.email_cooldown_secs =
                other.auth.signup_protection.email_cooldown_secs;
        }
        if o.auth_signup_turnstile_enabled {
            self.auth.signup_protection.turnstile.enabled =
                other.auth.signup_protection.turnstile.enabled;
        }
        if o.auth_signup_turnstile_secret_key {
            self.auth.signup_protection.turnstile.secret_key =
                other.auth.signup_protection.turnstile.secret_key;
        }
        if o.core_home {
            self.core.home = other.core.home;
        }
        if o.core_profile {
            self.core.profile = other.core.profile;
        }
        if o.tenant_default_id {
            self.tenant.default_id = other.tenant.default_id;
        }
        if o.telemetry_enabled {
            self.telemetry.enabled = other.telemetry.enabled;
        }
        if o.telemetry_exporter {
            self.telemetry.exporter = other.telemetry.exporter;
        }
        if o.telemetry_endpoint {
            self.telemetry.endpoint = other.telemetry.endpoint;
        }
        if o.error_reporting_mode {
            self.error_reporting.mode = other.error_reporting.mode;
        }
        if o.error_reporting_dsn {
            self.error_reporting.dsn = other.error_reporting.dsn;
        }
        if o.error_reporting_otlp_endpoint {
            self.error_reporting.otlp_endpoint = other.error_reporting.otlp_endpoint;
        }
        if o.error_reporting_pseudonym_salt {
            self.error_reporting.pseudonym_salt = other.error_reporting.pseudonym_salt;
        }
        if o.gateway_host {
            self.gateway.host = other.gateway.host;
        }
        if o.gateway_port {
            self.gateway.port = other.gateway.port;
        }
        if o.gateway_url {
            self.gateway.url = other.gateway.url;
        }
        if o.gateway_duty_scheduler_secs {
            self.gateway.duty_scheduler_secs = other.gateway.duty_scheduler_secs;
        }
        if o.gateway_handoff_sweep_secs {
            self.gateway.handoff_sweep_secs = other.gateway.handoff_sweep_secs;
        }
        if o.gateway_auth_bypass {
            self.gateway.auth_bypass = other.gateway.auth_bypass;
        }
        if o.gateway_rate_limit_ip_enabled {
            self.gateway.rate_limit.ip.enabled = other.gateway.rate_limit.ip.enabled;
        }
        if o.gateway_rate_limit_ip_capacity {
            self.gateway.rate_limit.ip.capacity = other.gateway.rate_limit.ip.capacity;
        }
        if o.gateway_rate_limit_ip_refill_per_sec {
            self.gateway.rate_limit.ip.refill_per_sec = other.gateway.rate_limit.ip.refill_per_sec;
        }
        if o.gateway_rate_limit_ip_trust_xff {
            self.gateway.rate_limit.ip.trust_xff = other.gateway.rate_limit.ip.trust_xff;
        }
        if o.gateway_rate_limit_route_default_rpm {
            self.gateway.rate_limit.route.default_rpm = other.gateway.rate_limit.route.default_rpm;
        }
        if o.gateway_rate_limit_route_forge_run_rpm {
            copy_route_override(self, "forge.run", forge_run_rpm);
        }
        if o.gateway_rate_limit_route_skill_execute_rpm {
            copy_route_override(self, "skill.execute", skill_execute_rpm);
        }
        if o.gateway_rate_limit_tenant_rpm {
            self.gateway.rate_limit.tenant.requests_per_minute =
                other.gateway.rate_limit.tenant.requests_per_minute;
        }
        if o.channels_webinbox_tenant_id {
            self.channels.webinbox.tenant_id = other.channels.webinbox.tenant_id;
        }
        if o.gateway_security_headers_csp_report_only {
            self.gateway.security_headers.csp_report_only =
                other.gateway.security_headers.csp_report_only;
        }
        // Mark merged fields in self too, so a subsequent merge
        // upstream of *self* knows these were overridden.
        self.overrides.combine(o);
    }
}

fn copy_route_override(base: &mut ArgoConfig, route: &str, value: Option<u32>) {
    if let Some(v) = value {
        base.gateway
            .rate_limit
            .route
            .overrides
            .insert(route.to_string(), v);
    }
}

#[cfg(test)]
#[path = "loader_tests.rs"]
mod loader_tests;
