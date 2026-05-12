//! Configuration for the native Windows computer-use executor.
//!
//! Defaults are PoC-conservative — Notepad only, no destructive
//! shortcuts, slow click-settle — and meant to be tightened (not
//! loosened) per-vertical via overrides.
//!
//! Reads from `[executor.computer_use.windows]` in the operator's
//! `argo.toml`; the section is additive and does not collide with
//! `[executor.computer_use.macos]`.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Screenshot strategy. `Full` returns one image of the entire
/// virtual screen; `PerMonitor` returns one image per attached
/// monitor (preferred for multi-monitor sessions because each
/// image carries its own scale-factor).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenshotKind {
    /// One image of the full virtual desktop.
    Full,
    /// One image per attached monitor.
    #[default]
    PerMonitor,
}

/// Static configuration for one [`crate::WindowsCuExecutor`]
/// instance. Cloneable so the same config can drive multiple
/// concurrent dispatches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsCuConfig {
    /// Process basenames or window classes this executor may
    /// activate or send input to (e.g., `notepad.exe`).
    pub allowed_apps: HashSet<String>,
    /// Action kinds this executor will pass through to the OS.
    /// Anything not in this set raises
    /// [`crate::WindowsCuError::ActionNotAllowed`].
    pub allowed_actions: HashSet<String>,
    /// Target app id (process basename or window class) for this
    /// run. Must also be a member of `allowed_apps`.
    pub target_app: String,
    /// Whether the executor is allowed to call `SendInput` with
    /// keyboard events. Some operators run in a restricted desktop
    /// where keystroke injection is gated by group policy; this
    /// flag flips the keyboard path off without disabling mouse
    /// input.
    pub allow_send_keys: bool,
    /// Whether the executor declares itself per-monitor v2 DPI
    /// aware. Required for correct scale-factor screenshots on
    /// high-density monitors. Default: `true`.
    pub dpi_aware: bool,
    /// Per-monitor vs full-screen screenshot strategy.
    pub screenshot_kind: ScreenshotKind,
    /// Settle ms after a click before the next event. Default:
    /// 50 ms.
    pub click_settle_ms: u64,
    /// Settle ms between successive key events when typing a
    /// string. Default: 15 ms.
    pub key_delay_ms: u64,
    /// Override for the Anthropic API base URL (so tests can
    /// point at a mock). `None` uses the production endpoint.
    pub anthropic_base_url: Option<String>,
    /// Anthropic model id driving the loop.
    pub anthropic_model: String,
    /// Per-Anthropic-call timeout in seconds.
    pub anthropic_timeout_seconds: u32,
    /// Hard upper bound on CU loop steps before forcing handoff.
    pub max_steps: u32,
    /// Hard upper bound on wallclock per dispatch, seconds.
    pub max_wallclock_seconds: u32,
}

impl WindowsCuConfig {
    /// Process basename for Windows Notepad. Used as the V1.5
    /// PoC default.
    pub const NOTEPAD_APP: &'static str = "notepad.exe";

    /// PoC default: only Notepad, only the safe-CU action set.
    /// Recommended starting point for any operator-supplied
    /// override.
    #[must_use]
    pub fn for_notepad_app() -> Self {
        let mut allowed_apps = HashSet::new();
        allowed_apps.insert(Self::NOTEPAD_APP.to_string());

        let mut allowed_actions = HashSet::new();
        for kind in ["click", "double_click", "type", "key", "scroll", "mouse_move"] {
            allowed_actions.insert(kind.to_string());
        }

        Self {
            allowed_apps,
            allowed_actions,
            target_app: Self::NOTEPAD_APP.to_string(),
            allow_send_keys: true,
            dpi_aware: true,
            screenshot_kind: ScreenshotKind::PerMonitor,
            click_settle_ms: 50,
            key_delay_ms: 15,
            anthropic_base_url: None,
            anthropic_model: "claude-opus-4-7".to_string(),
            anthropic_timeout_seconds: 60,
            max_steps: 25,
            max_wallclock_seconds: 600,
        }
    }

    /// Validates the config: `target_app` must be in
    /// `allowed_apps`, `allowed_actions` must be non-empty,
    /// budget limits must be > 0.
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
        if !self.allow_send_keys
            && (self.allowed_actions.contains("key") || self.allowed_actions.contains("type"))
        {
            return Err("allow_send_keys=false but allowlist contains key/type actions".to_string());
        }
        Ok(())
    }

    /// Whether the action kind passes the action allowlist.
    #[must_use]
    pub fn allows_action(&self, kind: &str) -> bool {
        self.allowed_actions.contains(kind)
    }

    /// Whether the app id passes the app allowlist.
    #[must_use]
    pub fn allows_app(&self, app: &str) -> bool {
        self.allowed_apps.contains(app)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notepad_default_validates() {
        let cfg = WindowsCuConfig::for_notepad_app();
        cfg.validate().unwrap();
        assert!(cfg.allows_app(WindowsCuConfig::NOTEPAD_APP));
        assert!(cfg.allows_action("click"));
        assert!(cfg.allows_action("type"));
        assert_eq!(cfg.screenshot_kind, ScreenshotKind::PerMonitor);
        assert!(cfg.dpi_aware);
    }

    #[test]
    fn destructive_actions_blocked_by_default() {
        let cfg = WindowsCuConfig::for_notepad_app();
        assert!(!cfg.allows_action("drag"));
        assert!(!cfg.allows_action("close_app"));
        assert!(!cfg.allows_action("delete"));
    }

    #[test]
    fn non_allowed_target_app_fails_validate() {
        let mut cfg = WindowsCuConfig::for_notepad_app();
        cfg.target_app = "cmd.exe".to_string();
        let err = cfg.validate().unwrap_err();
        assert!(err.contains("cmd.exe"));
        assert!(err.contains("allowed_apps"));
    }

    #[test]
    fn allow_send_keys_false_blocks_keyboard_allowlist() {
        let mut cfg = WindowsCuConfig::for_notepad_app();
        cfg.allow_send_keys = false;
        let err = cfg.validate().unwrap_err();
        assert!(err.contains("allow_send_keys"));
    }

    #[test]
    fn allow_send_keys_false_with_only_mouse_actions_is_valid() {
        let mut cfg = WindowsCuConfig::for_notepad_app();
        cfg.allow_send_keys = false;
        cfg.allowed_actions.remove("key");
        cfg.allowed_actions.remove("type");
        cfg.validate().unwrap();
    }

    #[test]
    fn empty_allowed_actions_fails_validate() {
        let mut cfg = WindowsCuConfig::for_notepad_app();
        cfg.allowed_actions.clear();
        let err = cfg.validate().unwrap_err();
        assert!(err.contains("allowed_actions"));
    }

    #[test]
    fn zero_max_steps_fails_validate() {
        let mut cfg = WindowsCuConfig::for_notepad_app();
        cfg.max_steps = 0;
        let err = cfg.validate().unwrap_err();
        assert!(err.contains("max_steps"));
    }

    #[test]
    fn screenshot_kind_default_is_per_monitor() {
        assert_eq!(ScreenshotKind::default(), ScreenshotKind::PerMonitor);
    }

    #[test]
    fn screenshot_kind_serialises_snake_case() {
        let s = serde_json::to_string(&ScreenshotKind::PerMonitor).unwrap();
        assert_eq!(s, "\"per_monitor\"");
        let s = serde_json::to_string(&ScreenshotKind::Full).unwrap();
        assert_eq!(s, "\"full\"");
    }
}
