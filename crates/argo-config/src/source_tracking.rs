//! Lightweight source annotations for the layered config loader.
//!
//! This module mirrors the loader's env/project/user/default
//! precedence for display purposes only; it does not participate in
//! merging config values.

use crate::defaults;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const ENV_SOURCES: &[(&str, &str)] = &[
    ("core.home", "ARGO_HOME"),
    ("core.profile", "ARGO_PROFILE"),
    ("tenant.default_id", "ARGO_DEFAULT_TENANT"),
    ("telemetry.enabled", "ARGO_TELEMETRY_ENABLED"),
    ("telemetry.exporter", "ARGO_TELEMETRY_EXPORTER"),
    ("telemetry.endpoint", "ARGO_TELEMETRY_ENDPOINT"),
    ("gateway.host", "ARGO_GATEWAY_HOST"),
    ("gateway.port", "ARGO_GATEWAY_PORT"),
    ("gateway.url", "ARGO_GATEWAY_URL"),
    ("gateway.duty_scheduler_secs", "ARGO_DUTY_SCHEDULER_SECS"),
    ("gateway.handoff_sweep_secs", "ARGO_HANDOFF_SWEEP_SECS"),
    ("gateway.auth_bypass", "ARGO_GATEWAY_AUTH_BYPASS"),
    ("gateway.rate_limit.ip.enabled", "ARGO_IP_RATE_LIMIT"),
    ("gateway.rate_limit.ip.capacity", "ARGO_IP_RATE_CAPACITY"),
    (
        "gateway.rate_limit.ip.refill_per_sec",
        "ARGO_IP_RATE_REFILL",
    ),
    ("gateway.rate_limit.ip.trust_xff", "ARGO_TRUST_XFF"),
    (
        "gateway.rate_limit.route.default_rpm",
        "ARGO_RATE_LIMIT_DEFAULT_RPM",
    ),
    (
        "gateway.rate_limit.route.overrides.forge.run",
        "ARGO_RATE_LIMIT_FORGE_RUN_RPM",
    ),
    (
        "gateway.rate_limit.route.overrides.skill.execute",
        "ARGO_RATE_LIMIT_SKILL_EXECUTE_RPM",
    ),
    (
        "gateway.rate_limit.tenant.requests_per_minute",
        "ARGO_RATE_LIMIT_TENANT_PER_MIN",
    ),
    ("channels.webinbox.tenant_id", "ARGO_WEBINBOX_TENANT_ID"),
    (
        "gateway.security_headers.csp_report_only",
        "ARGO_CSP_REPORT_ONLY",
    ),
];
const SHOW_SECRETS_ENV: &str = "ARGO_SHOW_SECRETS";

/// Return whether a config display may reveal secret-looking fields.
#[must_use]
pub fn secrets_display_allowed(requested: bool) -> bool {
    requested && std::env::var_os(SHOW_SECRETS_ENV).as_deref() == Some(std::ffi::OsStr::new("1"))
}

/// Mapping from dotted config keys to a human-readable source layer.
#[derive(Debug, Clone, Default)]
pub struct ConfigSourceMap {
    sources: BTreeMap<String, String>,
}

impl ConfigSourceMap {
    /// Detect config sources using the process cwd.
    ///
    /// Missing cwd or unreadable files simply reduce annotation
    /// precision; the effective config load remains authoritative.
    #[must_use]
    pub fn detect() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::detect_from(&cwd)
    }

    /// Detect config sources for a specific workspace root.
    #[must_use]
    pub fn detect_from(cwd: &Path) -> Self {
        let mut map = Self::default();
        if let Some(path) = user_config_path() {
            map.record_file(&path, "user config");
        }
        if let Some(path) = project_local_path(cwd) {
            map.record_file(&path, "project config");
        }
        map.record_env();
        map
    }

    /// Return the source label for `dotted_key`, defaulting to built-ins.
    #[must_use]
    pub fn source_for(&self, dotted_key: &str) -> &str {
        self.sources
            .get(dotted_key)
            .map(String::as_str)
            .unwrap_or("default")
    }

    fn record_file(&mut self, path: &Path, label: &str) {
        if !path.exists() {
            return;
        }
        let Ok(body) = std::fs::read_to_string(path) else {
            return;
        };
        let Ok(value) = body.parse::<toml::Value>() else {
            return;
        };
        let source = format!("{label} at {}", path.display());
        collect_toml_paths("", &value, &mut |key| {
            self.sources.insert(key, source.clone());
        });
    }

    fn record_env(&mut self) {
        for (key, env) in ENV_SOURCES {
            let Some(value) = std::env::var_os(env) else {
                continue;
            };
            if value.is_empty() {
                continue;
            }
            self.sources.insert((*key).into(), format!("env {env}"));
        }
    }
}

fn collect_toml_paths(prefix: &str, value: &toml::Value, f: &mut impl FnMut(String)) {
    match value {
        toml::Value::Table(table) => {
            for (key, child) in table {
                let dotted = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                collect_toml_paths(&dotted, child, f);
            }
        }
        _ if !prefix.is_empty() => f(prefix.to_string()),
        _ => {}
    }
}

fn project_local_path(cwd: &Path) -> Option<PathBuf> {
    let skycot = cwd.join(defaults::PROJECT_LOCAL_SKYCOT_CONFIG);
    if skycot.exists() {
        return Some(skycot);
    }
    let legacy = cwd.join(defaults::PROJECT_LOCAL_CONFIG);
    legacy.exists().then_some(legacy)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_file_sources_are_recorded() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("argo.toml"), "[custom]\nvalue = 7777\n").unwrap();
        let sources = ConfigSourceMap::detect_from(dir.path());
        assert!(sources
            .source_for("custom.value")
            .contains("project config"));
    }
}
