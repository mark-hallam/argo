//! Identity / runtime-control config sections: `[core]`, `[tenant]`,
//! `[loop]`, `[plugin]`. Extracted from [`crate::sections`] to keep
//! that module under the 500-LOC ceiling. Re-exported through
//! `sections` so the public API is unchanged.
use crate::defaults;
use argo_core::RestartPolicy;
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::path::PathBuf;

/// `[core]`.
///
/// # Examples
///
/// ```rust
/// use argo_config::CoreConfig;
///
/// let cfg = CoreConfig::default();
/// assert_eq!(cfg.profile, "default");
/// ```
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
///
/// # Examples
///
/// ```rust
/// use argo_config::TenantConfig;
///
/// let cfg = TenantConfig::default();
/// assert_eq!(cfg.mode, "single");
/// ```
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
///
/// # Examples
///
/// ```rust
/// use argo_config::LoopConfig;
///
/// let cfg = LoopConfig::default();
/// assert_eq!(cfg.on_restart, "prompt_user");
/// ```
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
///
/// # Examples
///
/// ```rust
/// use argo_config::PluginConfig;
///
/// let cfg = PluginConfig::default();
/// assert!(cfg.auto_load.is_empty());
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginConfig {
    /// Plugins to launch on startup.
    pub auto_load: Vec<String>,
}

fn expand_tilde(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = home_dir() {
            return home.join(rest);
        }
    } else if s == "~" {
        if let Some(home) = home_dir() {
            return home;
        }
    }
    PathBuf::from(s)
}

/// Resolve the user's home directory cross-platform for tilde expansion.
///
/// `HOME` is the unix convention and wins when set, so unix behavior is
/// byte-identical (and a git-bash shell on Windows that exports `HOME`
/// keeps working). On native Windows `HOME` is usually unset, so we fall
/// back to `USERPROFILE` (e.g. `C:\Users\alice`). Without this, the
/// `~/.argo` default ([`crate::defaults::DEFAULT_HOME`]) would expand to a
/// bare relative `~/.argo` on Windows. See
/// `win-portability-tmp-and-home-paths`.
fn home_dir() -> Option<PathBuf> {
    home_dir_from(std::env::var_os("HOME"), std::env::var_os("USERPROFILE"))
}

/// Pure core of [`home_dir`] — takes the env values explicitly so it can be
/// asserted without mutating process-global environment in tests. Empty
/// values are treated as unset so a stray `HOME=` does not win.
fn home_dir_from(home: Option<OsString>, userprofile: Option<OsString>) -> Option<PathBuf> {
    home.filter(|h| !h.is_empty())
        .or_else(|| userprofile.filter(|h| !h.is_empty()))
        .map(PathBuf::from)
}

/// Resolve the user-level config path cross-platform.
///
/// `$XDG_CONFIG_HOME/argo/argo.toml` when `XDG_CONFIG_HOME` is set, else
/// `<home>/.config/argo/argo.toml` where `<home>` resolves `HOME` → `USERPROFILE`
/// (see [`home_dir`]). `pub(crate)` so [`crate::loader`] and
/// [`crate::source_tracking`] share ONE resolver: they previously each carried a
/// HOME-only copy that returned `None` on native Windows (`HOME` unset), so the
/// user-config layer silently vanished and `argo config show --source` dropped
/// its user-config annotations. #2904 fixed the same class in `home_dir`/
/// `expand_tilde` but missed these two call sites. See
/// `win-config-user-config-path-home-userprofile-fallback`.
pub(crate) fn user_config_path() -> Option<PathBuf> {
    user_config_path_from(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
        std::env::var_os("USERPROFILE"),
    )
}

/// Pure core of [`user_config_path`] — env values passed explicitly so the
/// precedence (XDG wins; else HOME→USERPROFILE joined with `.config`) is
/// asserted without mutating process-global environment. Unix behaviour is
/// byte-identical to the prior HOME-only code (HOME set ⇒ same path).
fn user_config_path_from(
    xdg: Option<OsString>,
    home: Option<OsString>,
    userprofile: Option<OsString>,
) -> Option<PathBuf> {
    if let Some(xdg) = xdg {
        return Some(PathBuf::from(xdg).join(defaults::USER_CONFIG_RELATIVE));
    }
    home_dir_from(home, userprofile)
        .map(|h| h.join(".config").join(defaults::USER_CONFIG_RELATIVE))
}

#[cfg(test)]
mod home_dir_tests {
    use super::*;

    #[test]
    fn home_wins_when_set() {
        let got = home_dir_from(Some("/home/alice".into()), Some("C:\\Users\\alice".into()));
        assert_eq!(got, Some(PathBuf::from("/home/alice")));
    }

    #[test]
    fn user_config_prefers_xdg_unchanged_on_unix() {
        let got = user_config_path_from(
            Some("/xdg".into()),
            Some("/home/alice".into()),
            None,
        );
        assert_eq!(
            got,
            Some(PathBuf::from("/xdg").join(defaults::USER_CONFIG_RELATIVE))
        );
    }

    #[test]
    fn user_config_uses_home_dot_config_when_no_xdg() {
        let got = user_config_path_from(None, Some("/home/alice".into()), None);
        assert_eq!(
            got,
            Some(
                PathBuf::from("/home/alice")
                    .join(".config")
                    .join(defaults::USER_CONFIG_RELATIVE)
            )
        );
    }

    #[test]
    fn user_config_falls_back_to_userprofile_when_home_unset() {
        // Native Windows: HOME unset, no XDG → USERPROFILE/.config/argo/argo.toml
        // instead of the prior `None` (which dropped the user-config layer).
        let got = user_config_path_from(None, None, Some("C:\\Users\\alice".into()));
        assert_eq!(
            got,
            Some(
                PathBuf::from("C:\\Users\\alice")
                    .join(".config")
                    .join(defaults::USER_CONFIG_RELATIVE)
            )
        );
    }

    #[test]
    fn user_config_none_when_nothing_set() {
        assert_eq!(user_config_path_from(None, None, None), None);
    }

    #[test]
    fn falls_back_to_userprofile_when_home_unset() {
        let got = home_dir_from(None, Some("C:\\Users\\alice".into()));
        assert_eq!(got, Some(PathBuf::from("C:\\Users\\alice")));
    }

    #[test]
    fn empty_home_is_ignored_and_userprofile_used() {
        let got = home_dir_from(Some("".into()), Some("C:\\Users\\bob".into()));
        assert_eq!(got, Some(PathBuf::from("C:\\Users\\bob")));
    }

    #[test]
    fn none_when_neither_set() {
        assert_eq!(home_dir_from(None, None), None);
        assert_eq!(home_dir_from(Some("".into()), Some("".into())), None);
    }
}
