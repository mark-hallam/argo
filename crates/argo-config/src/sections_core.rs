//! Identity / runtime-control config sections: `[core]`, `[tenant]`,
//! `[loop]`, `[plugin]`. Extracted from [`crate::sections`] to keep
//! that module under the 500-LOC ceiling. Re-exported through
//! `sections` so the public API is unchanged.
use crate::defaults;
use argo_core::RestartPolicy;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// `[core]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CoreConfig {
    /// Argo home directory; supports `~` expansion.
    pub home: String,
    /// Working workspace path.
    pub workspace: String,
    /// Profile name.
    pub profile: String,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            home: defaults::DEFAULT_HOME.into(),
            workspace: defaults::DEFAULT_WORKSPACE.into(),
            profile: defaults::DEFAULT_PROFILE.into(),
        }
    }
}

impl CoreConfig {
    /// Resolve `home` to an absolute path, expanding `~`.
    #[must_use]
    pub fn resolved_home(&self) -> PathBuf {
        expand_tilde(&self.home)
    }
}

/// `[tenant]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TenantConfig {
    /// `single` or `multi`.
    pub mode: String,
    /// Tenant id used in single-tenant mode.
    pub default_id: String,
    /// `none`, `oidc`, or `header`.
    pub auth_provider: String,
}

impl Default for TenantConfig {
    fn default() -> Self {
        Self {
            mode: defaults::DEFAULT_TENANT_MODE.into(),
            default_id: defaults::DEFAULT_TENANT_ID.into(),
            auth_provider: defaults::DEFAULT_AUTH_PROVIDER.into(),
        }
    }
}

/// `[loop]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoopConfig {
    /// Restart policy (Amendment 13).
    pub on_restart: String,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            on_restart: defaults::DEFAULT_ON_RESTART.into(),
        }
    }
}

impl LoopConfig {
    /// Bind `on_restart` to [`RestartPolicy`]. Unrecognized values
    /// fall back to `prompt_user` (the spec default).
    #[must_use]
    pub fn restart_policy(&self) -> RestartPolicy {
        match self.on_restart.as_str() {
            "auto_resume" => RestartPolicy::AutoResume,
            "discard" => RestartPolicy::Discard,
            _ => RestartPolicy::PromptUser,
        }
    }
}

/// `[plugin]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginConfig {
    /// Plugins to launch on startup.
    pub auto_load: Vec<String>,
}

fn expand_tilde(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    } else if s == "~" {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home);
        }
    }
    PathBuf::from(s)
}
