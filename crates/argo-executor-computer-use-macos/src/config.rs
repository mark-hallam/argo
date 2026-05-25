//! Configuration for the macOS computer-use executor.
//!
//! Defaults are PoC-conservative — Notes only, no destructive
//! shortcuts, slow dwell — and meant to be tightened (not loosened)
//! per-vertical via overrides.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Optional post-capture transform for screenshots persisted to the
/// audit bundle and sent to the Anthropic CU loop. Native macOS
/// Retina captures land at 5-10× the size of an E2B browser
/// screenshot (see `docs/skycot/COMPUTER_USE_NATIVE_MACOS.md` open
/// question #3), which becomes GB-per-day per worker on long
/// trajectories.
///
/// Default ([`ScreenshotCompression::Png`] with `max_edge: None`)
/// preserves the V1.5 PoC behavior: native PNG, no resample, no
/// re-encode. Operators opt into a downscale and/or JPEG conversion
/// by setting an explicit variant. The transform runs through the
/// macOS `sips` binary so the crate stays dependency-free.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "format", rename_all = "snake_case")]
pub enum ScreenshotCompression {
    /// Keep PNG encoding. With `max_edge = None` the capture is
    /// passed through untouched; with `Some(edge)` the image is
    /// resampled so its longest side is `edge` pixels.
    Png { max_edge: Option<u32> },
    /// Re-encode to JPEG at `quality` (1..=100). `max_edge` is
    /// optional; when set the resample runs before the re-encode.
    Jpeg {
        max_edge: Option<u32>,
        quality: u8,
    },
}

impl Default for ScreenshotCompression {
    fn default() -> Self {
        Self::Png { max_edge: None }
    }
}

impl ScreenshotCompression {
    /// `true` when this setting requires any post-capture work
    /// (downscale and/or re-encode). The pass-through default
    /// returns `false`.
    #[must_use]
    pub const fn is_transform(&self) -> bool {
        match self {
            Self::Png { max_edge } => max_edge.is_some(),
            Self::Jpeg { .. } => true,
        }
    }

    /// File extension the transform writes (without the leading
    /// dot). The audit-bundle filename stays `step-NNN.png`
    /// regardless — this is the extension a caller would pick for
    /// a sibling temp file if it wanted truthful naming.
    #[must_use]
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::Png { .. } => "png",
            Self::Jpeg { .. } => "jpg",
        }
    }

    /// Validates the variant in isolation. Returns the same error
    /// strings [`MacosCuConfig::validate`] surfaces.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Png { max_edge } => check_max_edge(*max_edge),
            Self::Jpeg { max_edge, quality } => {
                check_max_edge(*max_edge)?;
                if !(1..=100).contains(quality) {
                    return Err(format!(
                        "screenshot_compression.jpeg.quality `{quality}` must be in 1..=100"
                    ));
                }
                Ok(())
            }
        }
    }
}

fn check_max_edge(edge: Option<u32>) -> Result<(), String> {
    match edge {
        Some(0) => Err("screenshot_compression.max_edge `0` must be > 0".to_string()),
        _ => Ok(()),
    }
}

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
    /// Post-capture screenshot transform. Default
    /// [`ScreenshotCompression::default`] preserves V1.5 PoC
    /// behavior (PNG, no downscale, no re-encode). Operators opt
    /// into a max-edge resample and/or JPEG conversion to bound
    /// audit storage cost on long native trajectories.
    #[serde(default)]
    pub screenshot_compression: ScreenshotCompression,
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
        for kind in [
            "click",
            "double_click",
            "type",
            "key",
            "scroll",
            "mouse_move",
        ] {
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
            screenshot_compression: ScreenshotCompression::default(),
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
        self.screenshot_compression.validate()?;
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

    #[test]
    fn notes_default_uses_pass_through_compression() {
        let cfg = MacosCuConfig::for_notes_app();
        assert_eq!(
            cfg.screenshot_compression,
            ScreenshotCompression::Png { max_edge: None }
        );
        assert!(!cfg.screenshot_compression.is_transform());
        assert_eq!(cfg.screenshot_compression.extension(), "png");
    }

    #[test]
    fn screenshot_compression_default_matches_pass_through() {
        assert_eq!(
            ScreenshotCompression::default(),
            ScreenshotCompression::Png { max_edge: None }
        );
    }

    #[test]
    fn png_with_max_edge_is_transform() {
        let c = ScreenshotCompression::Png {
            max_edge: Some(1280),
        };
        assert!(c.is_transform());
        c.validate().unwrap();
    }

    #[test]
    fn jpeg_is_always_transform() {
        let c = ScreenshotCompression::Jpeg {
            max_edge: None,
            quality: 80,
        };
        assert!(c.is_transform());
        assert_eq!(c.extension(), "jpg");
        c.validate().unwrap();
    }

    #[test]
    fn zero_max_edge_fails_validate() {
        let err = ScreenshotCompression::Png { max_edge: Some(0) }
            .validate()
            .unwrap_err();
        assert!(err.contains("max_edge"));
        let err = ScreenshotCompression::Jpeg {
            max_edge: Some(0),
            quality: 80,
        }
        .validate()
        .unwrap_err();
        assert!(err.contains("max_edge"));
    }

    #[test]
    fn jpeg_quality_outside_range_fails_validate() {
        for q in [0u8, 101u8, 200u8] {
            let err = ScreenshotCompression::Jpeg {
                max_edge: None,
                quality: q,
            }
            .validate()
            .unwrap_err();
            assert!(err.contains("quality"), "quality={q} should fail: {err}");
        }
    }

    #[test]
    fn jpeg_quality_boundary_values_validate() {
        for q in [1u8, 50u8, 100u8] {
            ScreenshotCompression::Jpeg {
                max_edge: None,
                quality: q,
            }
            .validate()
            .unwrap();
        }
    }

    #[test]
    fn config_validate_surfaces_compression_error() {
        let mut cfg = MacosCuConfig::for_notes_app();
        cfg.screenshot_compression = ScreenshotCompression::Jpeg {
            max_edge: None,
            quality: 0,
        };
        let err = cfg.validate().unwrap_err();
        assert!(err.contains("quality"));
    }

    #[test]
    fn screenshot_compression_omitted_from_toml_uses_default() {
        // Round-trip a config missing `screenshot_compression` to
        // confirm the `#[serde(default)]` lands the pass-through
        // variant rather than failing deserialization.
        let cfg = MacosCuConfig::for_notes_app();
        let mut value = serde_json::to_value(&cfg).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .remove("screenshot_compression");
        let round_tripped: MacosCuConfig = serde_json::from_value(value).unwrap();
        assert_eq!(
            round_tripped.screenshot_compression,
            ScreenshotCompression::Png { max_edge: None }
        );
    }
}
