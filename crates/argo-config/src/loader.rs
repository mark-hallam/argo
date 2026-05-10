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
use crate::{defaults, ArgoConfig};
use argo_types::{ArgoError, ArgoResult};
use std::path::{Path, PathBuf};

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
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
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
            .map_err(|e| ArgoError::Provider(format!("read {}: {e}", path.display())))?;
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

        if let Some(v) = env_string("ARGO_HOME") {
            cfg.core.home = v;
            overrides.core_home = true;
        }
        if let Some(v) = env_string("ARGO_PROFILE") {
            cfg.core.profile = v;
            overrides.core_profile = true;
        }
        if let Some(v) = env_string("ARGO_DEFAULT_TENANT") {
            cfg.tenant.default_id = v;
            overrides.tenant_default_id = true;
        }
        if let Some(v) = env_string("ARGO_TELEMETRY_EXPORTER") {
            cfg.telemetry.exporter = v;
            overrides.telemetry_exporter = true;
        }
        if let Some(v) = env_string("ARGO_TELEMETRY_ENDPOINT") {
            cfg.telemetry.endpoint = v;
            overrides.telemetry_endpoint = true;
        }
        if let Some(v) = env_bool("ARGO_TELEMETRY_ENABLED")? {
            cfg.telemetry.enabled = v;
            overrides.telemetry_enabled = true;
        }
        if let Some(v) = env_u16("ARGO_GATEWAY_PORT")? {
            cfg.gateway.port = v;
            overrides.gateway_port = true;
        }
        if let Some(v) = env_string("ARGO_GATEWAY_URL") {
            cfg.gateway.url = v;
            overrides.gateway_url = true;
        }
        if let Some(v) = env_string("ARGO_GATEWAY_HOST") {
            cfg.gateway.host = v;
            overrides.gateway_host = true;
        }
        if let Some(v) = env_u64("ARGO_DUTY_SCHEDULER_SECS")? {
            cfg.gateway.duty_scheduler_secs = v;
            overrides.gateway_duty_scheduler_secs = true;
        }
        if let Some(v) = env_u64("ARGO_HANDOFF_SWEEP_SECS")? {
            cfg.gateway.handoff_sweep_secs = v;
            overrides.gateway_handoff_sweep_secs = true;
        }

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
        // Mark merged fields in self too, so a subsequent merge
        // upstream of *self* knows these were overridden.
        self.overrides.combine(o);
    }
}

fn env_string(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.is_empty())
}

fn env_bool(key: &str) -> ArgoResult<Option<bool>> {
    let Some(raw) = env_string(key) else {
        return Ok(None);
    };
    match raw.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(Some(true)),
        "0" | "false" | "no" | "off" => Ok(Some(false)),
        _ => Err(ArgoError::Provider(format!(
            "{key}: expected bool (1/0/true/false), got '{raw}'"
        ))),
    }
}

fn env_u16(key: &str) -> ArgoResult<Option<u16>> {
    let Some(raw) = env_string(key) else {
        return Ok(None);
    };
    raw.parse::<u16>()
        .map(Some)
        .map_err(|e| ArgoError::Provider(format!("{key}: {e}")))
}

fn env_u64(key: &str) -> ArgoResult<Option<u64>> {
    let Some(raw) = env_string(key) else {
        return Ok(None);
    };
    raw.parse::<u64>()
        .map(Some)
        .map_err(|e| ArgoError::Provider(format!("{key}: {e}")))
}

/// Bookkeeping for which fields a particular [`ArgoConfig`] layer
/// explicitly set, so [`ArgoConfig::merge`] can do a precedence-aware
/// overlay without nuking unrelated defaults.
///
/// Toml-deserialized layers leave this all-`false` (every field came
/// from `#[serde(default)]`), which would normally make merge a
/// no-op. We special-case TOML in [`super::ArgoConfig::merge_toml`]
/// by setting the mask post-parse for the fields that were actually
/// present in the source — see that method for details.
#[derive(Debug, Clone, Default)]
pub(crate) struct ArgoConfigOverrides {
    pub core_home: bool,
    pub core_profile: bool,
    pub tenant_default_id: bool,
    pub telemetry_enabled: bool,
    pub telemetry_exporter: bool,
    pub telemetry_endpoint: bool,
    pub gateway_host: bool,
    pub gateway_port: bool,
    pub gateway_url: bool,
    pub gateway_duty_scheduler_secs: bool,
    pub gateway_handoff_sweep_secs: bool,
}

impl ArgoConfigOverrides {
    fn combine(&mut self, other: &Self) {
        self.core_home |= other.core_home;
        self.core_profile |= other.core_profile;
        self.tenant_default_id |= other.tenant_default_id;
        self.telemetry_enabled |= other.telemetry_enabled;
        self.telemetry_exporter |= other.telemetry_exporter;
        self.telemetry_endpoint |= other.telemetry_endpoint;
        self.gateway_host |= other.gateway_host;
        self.gateway_port |= other.gateway_port;
        self.gateway_url |= other.gateway_url;
        self.gateway_duty_scheduler_secs |= other.gateway_duty_scheduler_secs;
        self.gateway_handoff_sweep_secs |= other.gateway_handoff_sweep_secs;
    }

    /// Mark every field as overridden — used when a TOML layer is
    /// loaded in full and the operator's intent is "these are my
    /// values for everything I wrote in the file."
    fn mark_all_from_toml(&mut self, raw: &str) {
        // Only mark fields whose section appears in the raw TOML.
        // Coarse but cheap; works because the keys are unique.
        self.core_home |= raw.contains("[core]") && raw.contains("home");
        self.core_profile |= raw.contains("[core]") && raw.contains("profile");
        self.tenant_default_id |= raw.contains("[tenant]") && raw.contains("default_id");
        self.telemetry_enabled |= raw.contains("[telemetry]") && raw.contains("enabled");
        self.telemetry_exporter |= raw.contains("[telemetry]") && raw.contains("exporter");
        self.telemetry_endpoint |= raw.contains("[telemetry]") && raw.contains("endpoint");
        self.gateway_host |= raw.contains("[gateway]") && raw.contains("host");
        self.gateway_port |= raw.contains("[gateway]") && raw.contains("port");
        self.gateway_url |= raw.contains("[gateway]") && raw.contains("url");
        self.gateway_duty_scheduler_secs |=
            raw.contains("[gateway]") && raw.contains("duty_scheduler_secs");
        self.gateway_handoff_sweep_secs |=
            raw.contains("[gateway]") && raw.contains("handoff_sweep_secs");
    }
}

impl ArgoConfig {
    /// Variant of [`from_toml`](ArgoConfig::from_toml) that records
    /// which fields the operator set in the TOML body so subsequent
    /// merges respect their intent.
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] on parse failure.
    pub fn from_toml_with_overrides(s: &str) -> ArgoResult<Self> {
        let mut cfg = Self::from_toml(s)?;
        cfg.overrides.mark_all_from_toml(s);
        Ok(cfg)
    }
}

#[cfg(test)]
#[path = "loader_tests.rs"]
mod loader_tests;
