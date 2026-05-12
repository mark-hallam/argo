//! Wayland-side capability probe.
//!
//! Pure inspection: reads env vars, checks `PATH` for `wtype`/`grim`,
//! and (on Linux) attempts a session-bus introspection of
//! `org.freedesktop.portal.Desktop`. Returns a [`WaylandCapabilities`]
//! that the root [`crate::probe`] dispatcher consumes.

use crate::probe::{binary_on_path, WaylandCapabilities};

/// Probe Wayland capabilities for the current environment.
#[must_use]
pub fn probe_capabilities() -> WaylandCapabilities {
    WaylandCapabilities {
        compositor: detect_compositor(),
        portal_reachable: super::portal::is_reachable(),
        wtype: binary_on_path("wtype"),
        grim: binary_on_path("grim"),
    }
}

/// True when the Wayland backend has enough primitives to ship at
/// least typing + screenshots.
#[must_use]
pub fn caps_are_sufficient(caps: &WaylandCapabilities) -> bool {
    // Typing is the floor: without `wtype` we cannot satisfy the
    // contract. Screenshots can come from either portal or grim.
    caps.wtype && (caps.portal_reachable || caps.grim)
}

/// True when the compositor is on the documented support matrix.
/// Used for diagnostic messages, not the support decision.
#[must_use]
pub fn compositor_is_known(compositor: &str) -> bool {
    matches!(
        compositor,
        "gnome" | "kde" | "sway" | "hyprland"
    )
}

/// Operator-facing explanation of why a Wayland session was not
/// usable. Always non-empty.
#[must_use]
pub fn unsupported_reason(caps: &WaylandCapabilities) -> String {
    if !caps.wtype {
        return "Wayland session detected but `wtype` missing on PATH; install wtype for keyboard input".into();
    }
    if !caps.portal_reachable && !caps.grim {
        return "Wayland session detected but neither xdg-desktop-portal nor `grim` reachable for screenshots".into();
    }
    format!(
        "Wayland session on `{}` has insufficient input/screenshot primitives",
        caps.compositor
    )
}

/// Mouse-click guidance string returned when a compositor lacks
/// portal RemoteDesktop support. Kept here so [`crate::wayland::input`]
/// and tests share one phrasing.
#[must_use]
pub fn mouse_unsupported_hint(compositor: &str) -> String {
    format!(
        "Wayland input emulation requires compositor support: \
         `{compositor}` does not advertise portal RemoteDesktop. \
         use GNOME/KDE/sway/Hyprland for full input support"
    )
}

/// Best-effort compositor detection from `XDG_CURRENT_DESKTOP` /
/// `XDG_SESSION_DESKTOP`. Returns a lowercase short name.
#[must_use]
pub fn detect_compositor() -> String {
    classify_compositor(
        std::env::var("XDG_CURRENT_DESKTOP").ok().as_deref(),
        std::env::var("XDG_SESSION_DESKTOP").ok().as_deref(),
    )
}

/// Pure classifier separated from env reads for unit testing.
#[must_use]
pub fn classify_compositor(
    xdg_current_desktop: Option<&str>,
    xdg_session_desktop: Option<&str>,
) -> String {
    let candidates = [xdg_current_desktop, xdg_session_desktop];
    for raw in candidates.into_iter().flatten() {
        for token in raw.split(':') {
            let lower = token.trim().to_ascii_lowercase();
            if lower.contains("gnome") {
                return "gnome".into();
            }
            if lower.contains("kde") || lower.contains("plasma") {
                return "kde".into();
            }
            if lower.contains("sway") {
                return "sway".into();
            }
            if lower.contains("hyprland") {
                return "hyprland".into();
            }
        }
    }
    "unknown".into()
}
