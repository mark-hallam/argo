//! Configuration for the macOS computer-use executor.
//!
//! Defaults are PoC-conservative — Notes only, no destructive
//! shortcuts, slow dwell — and meant to be tightened (not loosened)
//! per-vertical via overrides.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Static configuration for one [`crate::MacosCuExecutor`]
/// instance. Cloneable so the same config can drive multiple
/// concurrent dispatches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacosCuConfig {
    /// Bundle ids this executor may activate or send events to.
    /// Defends against the model trying to escape via `Cmd+Space
    /// → "Terminal" → Return`.
    pub allowed_apps: HashSet<String>,
    /// Action kinds this executor will pass through to the OS.
    /// Anything not in this set raises
    /// [`crate::MacosCuError::ActionNotAllowed`].
    pub allowed_actions: HashSet<String>,
    /// Bundle id of the target app for this run. Must also be a
    /// member of `allowed_apps`.
    pub target_app: String,
    /// Sleep between input events to allow operator-kill mid-loop.
    /// Defaults to 200 ms — long enough that a runaway loop is
    /// pause-able by `Ctrl+C` on the parent process.
    pub dwell_ms: u32,
    /// Override for the Anthropic API base URL (so tests can point
    /// at a mock server). `None` uses the production endpoint.
    pub anthropic_base_url: Option<String>,
    /// Anthropic model id to drive the loop. Defaults to
    /// `claude-opus-4-7` — the same model the E2B executor uses
    /// in production.
    pub anthropic_model: String,
    /// Per-Anthropic-call timeout in seconds.
    pub anthropic_timeout_seconds: u32,
    /// Hard upper bound on CU loop steps before forcing handoff.
    /// Default: 25 (matches the E2B executor's per-action ceiling).
    pub max_steps: u32,
    /// Hard upper bound on wallclock per dispatch, seconds.
    pub max_wallclock_seconds: u32,
}

impl MacosCuConfig {
    /// Bundle id for macOS Notes. Used as the V1.5 PoC default.
    pub const NOTES_BUNDLE_ID: &'static str = "com.apple.Notes";

    /// PoC default: only Notes, only the safe-CU action set.
    /// Used by the smoke test and as the recommended starting
    /// point for any operator-supplied config.
    #[must_use]
    pub fn for_notes_app() -> Self {
        let mut allowed_apps = HashSet::new();
        allowed_apps.insert(Self::NOTES_BUNDLE_ID.to_string());

        let mut allowed_actions = HashSet::new();
        for kind in ["click", "double_click", "type", "key", "scroll", "mouse_move"] {
            allowed_actions.insert(kind.to_string());
        }

        Self {
            allowed_apps,
            allowed_actions,
            target_app: Self::NOTES_BUNDLE_ID.to_string(),
            dwell_ms: 200,
            anthropic_base_url: None,
            anthropic_model: "claude-opus-4-7".to_string(),
            anthropic_timeout_seconds: 60,
            max_steps: 25,
            max_wallclock_seconds: 600,
        }
    }

    /// Validates the config: `target_app` must be in
    /// `allowed_apps`, `allowed_actions` must be non-empty.
    pub fn validate(&self) -> Result<(), String> {
        if !self.allowed_apps.contains(&self.target_app) {
            return Err(format!(
                "target_app `{}` is not in allowed_apps",
                self.target_app
            ));
        }
        if self.allowed_actions.is_empty() {
            return Err("allowed_actions must contain at least one entry".to_string());
        }
        if self.max_steps == 0 {
            return Err("max_steps must be > 0".to_string());
        }
        if self.max_wallclock_seconds == 0 {
            return Err("max_wallclock_seconds must be > 0".to_string());
        }
        Ok(())
    }

    /// Whether the action kind passes the action allowlist.
    #[must_use]
    pub fn allows_action(&self, kind: &str) -> bool {
        self.allowed_actions.contains(kind)
    }

    /// Whether the bundle id passes the app allowlist.
    #[must_use]
    pub fn allows_app(&self, bundle_id: &str) -> bool {
        self.allowed_apps.contains(bundle_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_default_validates() {
        let cfg = MacosCuConfig::for_notes_app();
        cfg.validate().unwrap();
        assert!(cfg.allows_app(MacosCuConfig::NOTES_BUNDLE_ID));
        assert!(cfg.allows_action("click"));
        assert!(cfg.allows_action("type"));
    }

    #[test]
    fn destructive_actions_blocked_by_default() {
        let cfg = MacosCuConfig::for_notes_app();
        assert!(!cfg.allows_action("drag"));
        assert!(!cfg.allows_action("close_app"));
        assert!(!cfg.allows_action("delete"));
    }

    #[test]
    fn non_allowed_target_app_fails_validate() {
        let mut cfg = MacosCuConfig::for_notes_app();
        cfg.target_app = "com.apple.Terminal".to_string();
        let err = cfg.validate().unwrap_err();
        assert!(err.contains("Terminal"));
        assert!(err.contains("allowed_apps"));
    }

    #[test]
    fn empty_allowed_actions_fails_validate() {
        let mut cfg = MacosCuConfig::for_notes_app();
        cfg.allowed_actions.clear();
        let err = cfg.validate().unwrap_err();
        assert!(err.contains("allowed_actions"));
    }

    #[test]
    fn zero_max_steps_fails_validate() {
        let mut cfg = MacosCuConfig::for_notes_app();
        cfg.max_steps = 0;
        let err = cfg.validate().unwrap_err();
        assert!(err.contains("max_steps"));
    }
}
