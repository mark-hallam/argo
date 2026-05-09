//! `[tui]` — interactive TUI surface (§20).
//!
//! Lives in its own module to keep `lib.rs` under the 500-LOC
//! invariant. Currently carries a single `[tui.theme]` block; future
//! TUI-wide options (keymap, pane defaults) belong here too.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// `[tui]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TuiConfig {
    /// `[tui.theme]`.
    pub theme: TuiThemeConfig,
}

/// `[tui.theme]` — pluggable color palette.
///
/// The TUI ships four presets (`dark`, `light`, `solarized`,
/// `dracula`); operators pick one with `name = "..."` and layer
/// per-slot hex overrides via the `overrides` map, e.g.:
///
/// ```toml
/// [tui.theme]
/// name = "solarized"
/// overrides = { accent = "#ff8800" }
/// ```
///
/// Unknown names and invalid hex values fall back to the base
/// preset with a warning log — the TUI never refuses to start on a
/// bad theme config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TuiThemeConfig {
    /// Preset name. Empty / unset ⇒ dark.
    pub name: String,
    /// Per-slot hex overrides keyed by slot name (`accent`, `fg`,
    /// `border`, …). Values are `#rrggbb` or `rrggbb`.
    pub overrides: BTreeMap<String, String>,
}

impl Default for TuiThemeConfig {
    fn default() -> Self {
        Self {
            name: "dark".into(),
            overrides: BTreeMap::new(),
        }
    }
}
