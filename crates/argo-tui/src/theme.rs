//! Pluggable color themes for the TUI.
//!
//! Centralises every styled color used by `render.rs`, `viewer.rs`,
//! and the overlay files into a single [`Theme`] struct. Four
//! built-in presets ship: `dark` (default — matches today's terminal
//! defaults exactly), `light`, `solarized`, `dracula`. Operators
//! select a preset via `[tui.theme] name = "..."` in `argo.toml` and
//! may layer per-field overrides (e.g.
//! `overrides = { accent = "#ff8800" }`).
//!
//! This module is intentionally pure: it constructs `Theme` values
//! and does not touch I/O. Threading `&Theme` through render
//! functions is a follow-up sweep — see the
//! `claude-tui-theme-system` brief.
use ratatui::style::Color;
use std::collections::BTreeMap;

/// Named bag of colors used across the TUI surface. Every styled
/// cell (foreground, background, border, status badge, selection
/// highlight) reads from one of these slots, so swapping a `Theme`
/// repaints the whole UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    /// Preset name (e.g. `"dark"`, `"light"`, `"solarized"`).
    pub name: String,
    /// Default foreground.
    pub fg: Color,
    /// Default background.
    pub bg: Color,
    /// Accent / primary action color.
    pub accent: Color,
    /// Success / OK badge.
    pub success: Color,
    /// Warning badge.
    pub warning: Color,
    /// Error / failure badge.
    pub error: Color,
    /// Muted / secondary text.
    pub muted: Color,
    /// Pane border.
    pub border: Color,
    /// Background for the highlighted list item.
    pub selection_bg: Color,
    /// Foreground for the highlighted list item.
    pub selection_fg: Color,
    /// Idle agent / session badge.
    pub status_idle: Color,
    /// Busy / running badge.
    pub status_busy: Color,
    /// Error-state badge.
    pub status_error: Color,
    /// Offline / disconnected badge.
    pub status_offline: Color,
}

impl Theme {
    /// Default dark preset — uses `Color::Reset` for fg/bg so the
    /// terminal's own defaults bleed through. This is what the TUI
    /// rendered before the theme system landed; PR1 must be a
    /// zero-visual-diff change.
    ///
    /// ```
    /// use argo_tui::Theme;
    /// let dark = Theme::dark();
    /// assert_eq!(dark.name, "dark");
    /// ```
    #[must_use]
    pub fn dark() -> Self {
        Self {
            name: "dark".into(),
            fg: Color::Reset,
            bg: Color::Reset,
            accent: Color::Cyan,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            muted: Color::DarkGray,
            border: Color::Reset,
            selection_bg: Color::Reset,
            selection_fg: Color::Reset,
            status_idle: Color::Gray,
            status_busy: Color::Cyan,
            status_error: Color::Red,
            status_offline: Color::DarkGray,
        }
    }

    /// Light preset — inverted lightness, kept readable on a white
    /// terminal background.
    #[must_use]
    pub fn light() -> Self {
        Self {
            name: "light".into(),
            fg: Color::Black,
            bg: Color::White,
            accent: Color::Blue,
            success: Color::Green,
            warning: Color::Rgb(0xb5, 0x82, 0x00),
            error: Color::Red,
            muted: Color::Gray,
            border: Color::Gray,
            selection_bg: Color::Rgb(0xee, 0xee, 0xee),
            selection_fg: Color::Black,
            status_idle: Color::DarkGray,
            status_busy: Color::Blue,
            status_error: Color::Red,
            status_offline: Color::Gray,
        }
    }

    /// Solarized preset — Ethan Schoonover's palette, dark variant.
    #[must_use]
    pub fn solarized() -> Self {
        Self {
            name: "solarized".into(),
            fg: Color::Rgb(0x83, 0x94, 0x96), // base0
            bg: Color::Rgb(0x00, 0x2b, 0x36), // base03
            accent: Color::Rgb(0x26, 0x8b, 0xd2), // blue
            success: Color::Rgb(0x85, 0x99, 0x00), // green
            warning: Color::Rgb(0xb5, 0x89, 0x00), // yellow
            error: Color::Rgb(0xdc, 0x32, 0x2f), // red
            muted: Color::Rgb(0x58, 0x6e, 0x75), // base01
            border: Color::Rgb(0x07, 0x36, 0x42), // base02
            selection_bg: Color::Rgb(0x07, 0x36, 0x42),
            selection_fg: Color::Rgb(0x93, 0xa1, 0xa1),
            status_idle: Color::Rgb(0x65, 0x7b, 0x83),
            status_busy: Color::Rgb(0x26, 0x8b, 0xd2),
            status_error: Color::Rgb(0xdc, 0x32, 0x2f),
            status_offline: Color::Rgb(0x58, 0x6e, 0x75),
        }
    }

    /// Dracula preset — popular high-contrast purple/pink palette.
    #[must_use]
    pub fn dracula() -> Self {
        Self {
            name: "dracula".into(),
            fg: Color::Rgb(0xf8, 0xf8, 0xf2),
            bg: Color::Rgb(0x28, 0x2a, 0x36),
            accent: Color::Rgb(0xbd, 0x93, 0xf9), // purple
            success: Color::Rgb(0x50, 0xfa, 0x7b), // green
            warning: Color::Rgb(0xf1, 0xfa, 0x8c), // yellow
            error: Color::Rgb(0xff, 0x55, 0x55), // red
            muted: Color::Rgb(0x62, 0x72, 0xa4),
            border: Color::Rgb(0x44, 0x47, 0x5a),
            selection_bg: Color::Rgb(0x44, 0x47, 0x5a),
            selection_fg: Color::Rgb(0xf8, 0xf8, 0xf2),
            status_idle: Color::Rgb(0x62, 0x72, 0xa4),
            status_busy: Color::Rgb(0x8b, 0xe9, 0xfd), // cyan
            status_error: Color::Rgb(0xff, 0x55, 0x55),
            status_offline: Color::Rgb(0x44, 0x47, 0x5a),
        }
    }

    /// Look up a preset by name. Unknown names fall back to
    /// [`Theme::dark`] and emit a `tracing::warn!` so the operator
    /// notices a typo without the TUI failing to start.
    #[must_use]
    pub fn lookup(name: &str) -> Self {
        match name {
            "dark" => Self::dark(),
            "light" => Self::light(),
            "solarized" => Self::solarized(),
            "dracula" => Self::dracula(),
            other => {
                tracing_warn(&format!(
                    "unknown tui theme {other:?}; falling back to \"dark\""
                ));
                Self::dark()
            }
        }
    }

    /// Apply per-field overrides parsed from `[tui.theme] overrides`.
    /// Each entry's key names a slot (`fg`, `accent`, `border`, …);
    /// the value is a hex string (`#rrggbb` or `rrggbb`). Unknown
    /// slot names and invalid hex log a warning and leave the slot
    /// unchanged — the TUI never panics on bad config.
    #[must_use]
    pub fn with_overrides(mut self, overrides: &BTreeMap<String, String>) -> Self {
        for (key, hex) in overrides {
            let Some(color) = parse_hex(hex) else {
                tracing_warn(&format!(
                    "tui theme override {key:?} = {hex:?}: invalid hex; ignoring"
                ));
                continue;
            };
            if !apply_override(&mut self, key, color) {
                tracing_warn(&format!(
                    "tui theme override {key:?}: unknown slot; ignoring"
                ));
            }
        }
        self
    }
}

/// Build a [`Theme`] from a config-shaped pair of inputs. The
/// brief's `from_config(cfg: &TuiConfig)` reduces to this once the
/// caller pulls `cfg.theme.name` and `cfg.theme.overrides` apart;
/// expressing the dependency at this granularity keeps `argo-tui`
/// from needing to import `argo-config` directly (no new crate
/// dependency edge).
#[must_use]
pub fn from_config(name: Option<&str>, overrides: &BTreeMap<String, String>) -> Theme {
    let base = match name {
        Some(n) if !n.is_empty() => Theme::lookup(n),
        _ => Theme::dark(),
    };
    base.with_overrides(overrides)
}

fn apply_override(theme: &mut Theme, key: &str, color: Color) -> bool {
    match key {
        "fg" => theme.fg = color,
        "bg" => theme.bg = color,
        "accent" => theme.accent = color,
        "success" => theme.success = color,
        "warning" => theme.warning = color,
        "error" => theme.error = color,
        "muted" => theme.muted = color,
        "border" => theme.border = color,
        "selection_bg" => theme.selection_bg = color,
        "selection_fg" => theme.selection_fg = color,
        "status_idle" => theme.status_idle = color,
        "status_busy" => theme.status_busy = color,
        "status_error" => theme.status_error = color,
        "status_offline" => theme.status_offline = color,
        _ => return false,
    }
    true
}

/// Parse `#rrggbb` / `rrggbb` into a 24-bit `Color::Rgb`. Returns
/// `None` for any input that isn't exactly 6 hex digits.
pub(crate) fn parse_hex(s: &str) -> Option<Color> {
    let body = s.strip_prefix('#').unwrap_or(s);
    if body.len() != 6 || !body.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&body[0..2], 16).ok()?;
    let g = u8::from_str_radix(&body[2..4], 16).ok()?;
    let b = u8::from_str_radix(&body[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

fn tracing_warn(msg: &str) {
    // argo-tui has no direct `tracing` dep (out of this brief's
    // may-touch Cargo.toml scope). Theme parsing happens at config
    // load — before the TUI grabs the terminal — so a stderr line
    // is the safe "warning log" channel.
    eprintln!("argo-tui theme: {msg}");
}

#[cfg(test)]
#[path = "theme_tests.rs"]
mod tests;
