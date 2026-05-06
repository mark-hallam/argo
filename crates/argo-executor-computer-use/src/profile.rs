//! Profile registry + schema. A "profile" picks the sandbox
//! image (Chromium-only browser, full desktop, …), the URL
//! whitelist for navigation, and per-profile budget caps. Each
//! action's manifest declares the profile name; the executor
//! resolves it at dispatch time.
//!
//! v1 ships exactly one profile: `marketplace_browser`. More
//! profiles land alongside their first consumer (e.g. when a
//! support-triage worker needs `desktop_full` it adds a profile
//! row in the TOML manifest, no code change here).

use crate::CuError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// One row from `manifests/executors/computer_use.toml`. Keyed
/// by profile name (`marketplace_browser`, `desktop_full`, …).
/// `deny_unknown_fields` so a typo in the manifest fails at
/// load time, not at session-open time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// E2B sandbox image / template name. The E2B managed
    /// service maps this to a Firecracker microVM with the
    /// right packages preinstalled.
    pub sandbox_template: String,
    /// Human-readable description shown in audit logs and
    /// error messages.
    #[serde(default)]
    pub description: String,
    /// URL prefix whitelist for navigation. The CU loop refuses
    /// to navigate to anything outside this list. Empty list
    /// means "no whitelist" — the profile trusts the CU model
    /// to stay in scope. Use cautiously.
    #[serde(default)]
    pub url_whitelist: Vec<String>,
    /// Per-action token budget cap. The CU loop refuses to
    /// continue when cumulative tokens exceed this. The brief
    /// expects this to be expressed in input+output tokens
    /// summed; the budget tracker enforces.
    #[serde(default = "default_token_budget")]
    pub max_tokens_per_action: u32,
    /// Per-action wallclock budget cap, seconds. Even if the
    /// token budget is fine, the loop bails after this much real
    /// time so a stuck session can't burn the whole quota.
    #[serde(default = "default_wallclock_seconds")]
    pub max_wallclock_seconds: u32,
    /// Maximum number of CU tool calls per action. A safety net
    /// for runaway loops where the model emits cheap actions
    /// without progressing.
    #[serde(default = "default_max_steps")]
    pub max_steps: u32,
    /// Width of the screenshot the CU loop sends to the model.
    /// Anthropic's CU tool variant expects 1280×800 baseline; we
    /// surface this so a future profile can override (e.g.
    /// retina, mobile viewport).
    #[serde(default = "default_screen_width")]
    pub screen_width: u32,
    /// Height of the screenshot the CU loop sends.
    #[serde(default = "default_screen_height")]
    pub screen_height: u32,
    /// Anthropic model identifier to drive the CU loop. Pinned
    /// per-profile so a model upgrade is an explicit edit.
    #[serde(default = "default_model")]
    pub model: String,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            sandbox_template: "skycot/marketplace-browser:latest".into(),
            description: "Chromium-only profile for marketplace listings".into(),
            url_whitelist: vec![
                "https://www.facebook.com/marketplace".into(),
                "https://www.ebay.com".into(),
                "https://www.gumtree.com".into(),
            ],
            max_tokens_per_action: default_token_budget(),
            max_wallclock_seconds: default_wallclock_seconds(),
            max_steps: default_max_steps(),
            screen_width: default_screen_width(),
            screen_height: default_screen_height(),
            model: default_model(),
        }
    }
}

fn default_token_budget() -> u32 {
    150_000
}

fn default_wallclock_seconds() -> u32 {
    600
}

fn default_max_steps() -> u32 {
    50
}

fn default_screen_width() -> u32 {
    1280
}

fn default_screen_height() -> u32 {
    800
}

fn default_model() -> String {
    "claude-opus-4-7".into()
}

/// Top-level shape of `manifests/executors/computer_use.toml`.
/// One `[profiles.<name>]` table per profile; the registry
/// flattens these into a name → Profile lookup.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct ProfilesFile {
    #[serde(default)]
    profiles: BTreeMap<String, Profile>,
}

/// Loaded profile registry. Built once at executor construction
/// and reused for every dispatch — profile resolution is a
/// `BTreeMap::get`.
#[derive(Debug, Clone, Default)]
pub struct ProfileRegistry {
    profiles: BTreeMap<String, Profile>,
}

impl ProfileRegistry {
    /// Load the registry from a TOML file path. Used at
    /// executor startup. Returns an empty registry when the file
    /// is absent — the caller falls back to
    /// [`ProfileRegistry::with_builtin_marketplace_browser`].
    pub fn load_from_path(path: &Path) -> Result<Self, CuError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let body = std::fs::read_to_string(path).map_err(|e| CuError::Io {
            what: "profile.read",
            source: e,
        })?;
        let parsed: ProfilesFile = toml::from_str(&body).map_err(|e| CuError::Toml {
            what: "profile.parse",
            source: e,
        })?;
        Ok(Self {
            profiles: parsed.profiles,
        })
    }

    /// Built-in registry containing the v1 default profile
    /// (`marketplace_browser`). Used when no manifest is
    /// present; tests use this to stay self-contained.
    #[must_use]
    pub fn with_builtin_marketplace_browser() -> Self {
        let mut profiles = BTreeMap::new();
        profiles.insert("marketplace_browser".to_string(), Profile::default());
        Self { profiles }
    }

    /// Look up a profile by name. Returns
    /// [`CuError::UnknownProfile`] (with the registered set in
    /// the message) when missing.
    pub fn get(&self, name: &str) -> Result<&Profile, CuError> {
        self.profiles.get(name).ok_or_else(|| {
            let registered = self
                .profiles
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            CuError::UnknownProfile {
                requested: name.to_string(),
                registered,
            }
        })
    }

    /// True when the registry holds at least one profile. Caller
    /// uses this to decide whether to fall back to the builtin
    /// after a failed load.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }

    /// Iterate every registered profile. Used for `--list` style
    /// CLI output and for budget reports.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Profile)> {
        self.profiles.iter().map(|(k, v)| (k.as_str(), v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_registry_resolves_marketplace_browser() {
        let reg = ProfileRegistry::with_builtin_marketplace_browser();
        let p = reg.get("marketplace_browser").expect("default profile");
        assert!(p.sandbox_template.contains("marketplace"));
        assert_eq!(p.screen_width, 1280);
        assert_eq!(p.screen_height, 800);
        assert!(p.max_tokens_per_action > 0);
    }

    #[test]
    fn unknown_profile_lists_registered() {
        let reg = ProfileRegistry::with_builtin_marketplace_browser();
        let err = reg
            .get("desktop_full")
            .expect_err("desktop_full not registered");
        let msg = err.to_string();
        assert!(msg.contains("desktop_full"), "got: {msg}");
        assert!(msg.contains("marketplace_browser"), "got: {msg}");
    }

    #[test]
    fn empty_registry_marks_is_empty() {
        let reg = ProfileRegistry::default();
        assert!(reg.is_empty());
        assert!(reg.iter().next().is_none());
    }

    #[test]
    fn loads_from_toml_with_one_profile() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("computer_use.toml");
        std::fs::write(
            &path,
            r#"
[profiles.kitchen_sink]
sandbox_template = "skycot/kitchen-sink:latest"
description = "everything-but-the-kitchen-sink for tests"
url_whitelist = ["https://example.com"]
max_tokens_per_action = 50000
max_wallclock_seconds = 120
max_steps = 25
screen_width = 1920
screen_height = 1080
model = "claude-sonnet-4-6"
"#,
        )
        .expect("write profile toml");
        let reg = ProfileRegistry::load_from_path(&path).expect("load profile registry");
        let p = reg.get("kitchen_sink").expect("loaded profile");
        assert_eq!(p.sandbox_template, "skycot/kitchen-sink:latest");
        assert_eq!(p.url_whitelist, vec!["https://example.com".to_string()]);
        assert_eq!(p.max_tokens_per_action, 50000);
        assert_eq!(p.screen_width, 1920);
        assert_eq!(p.model, "claude-sonnet-4-6");
    }

    #[test]
    fn loads_from_missing_path_returns_empty() {
        let reg = ProfileRegistry::load_from_path(std::path::Path::new(
            "/tmp/skycot-does-not-exist-marker.toml",
        ))
        .expect("missing path is not a load failure");
        assert!(reg.is_empty());
    }

    #[test]
    fn rejects_unknown_field_on_profile() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("computer_use.toml");
        std::fs::write(
            &path,
            r#"
[profiles.busted]
sandbox_template = "x"
mistyped_field = "should-fail"
"#,
        )
        .expect("write profile toml");
        let err = ProfileRegistry::load_from_path(&path)
            .expect_err("deny_unknown_fields rejects typo");
        assert!(err.to_string().contains("toml parse failure"));
    }
}
