//! Runtime support probe — selects the backend ([`crate::wayland`]
//! or [`crate::x11`]) for each call. Inspects `WAYLAND_DISPLAY` first
//! and falls back to `$DISPLAY` (X11) when no Wayland session is up.

use crate::wayland;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Platform support status. `Supported` is a unit variant so
/// downstream callers compile without churn; backend detail lives on
/// [`ProbeReport::display_mode`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeStatus {
    /// At least one backend (X11 or Wayland) is fully available.
    Supported,
    /// Host cannot run the native Linux CU executor.
    Unsupported(String),
}

/// Selected display-server backend.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayMode {
    /// No supported backend.
    #[default]
    None,
    /// Legacy X11 via `xdotool` + `scrot`.
    X11,
    /// Wayland via `wtype` / `grim` / xdg-desktop-portal.
    Wayland(WaylandCapabilities),
}

/// Capability snapshot for the Wayland backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WaylandCapabilities {
    /// Compositor: `gnome`, `kde`, `sway`, `hyprland`, `unknown`.
    pub compositor: String,
    /// xdg-desktop-portal Desktop reachable over the session bus.
    pub portal_reachable: bool,
    /// `wtype` binary on `PATH`.
    pub wtype: bool,
    /// `grim` binary on `PATH`.
    pub grim: bool,
}

/// Probe report for required Linux CU primitives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbeReport {
    /// Current target OS according to Rust cfg.
    pub platform: String,
    /// Whether `xdotool` is available on PATH.
    pub xdotool: bool,
    /// Whether `scrot` is available on PATH.
    pub scrot: bool,
    /// Whether an X11 display is advertised.
    pub x11_display: bool,
    /// Whether a Wayland display is advertised.
    pub wayland_display: bool,
    /// Selected backend with capability detail.
    pub display_mode: DisplayMode,
    /// Final support decision.
    pub status: ProbeStatus,
}

impl ProbeReport {
    /// True when native Linux CU can run.
    #[must_use]
    pub fn is_supported(&self) -> bool {
        matches!(self.status, ProbeStatus::Supported)
    }
}

/// Check the current host for Linux CU support. Wayland is
/// preferred when both backends could plausibly run.
#[must_use]
pub fn probe() -> ProbeReport {
    let xdotool = binary_on_path("xdotool");
    let scrot = binary_on_path("scrot");
    let x11_display = std::env::var_os("DISPLAY").is_some();
    let wayland_display = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let platform = std::env::consts::OS.to_string();
    let (display_mode, status) = decide(xdotool, scrot, x11_display, wayland_display);
    ProbeReport {
        platform,
        xdotool,
        scrot,
        x11_display,
        wayland_display,
        display_mode,
        status,
    }
}

fn decide(
    xdotool: bool,
    scrot: bool,
    x11_display: bool,
    wayland_display: bool,
) -> (DisplayMode, ProbeStatus) {
    if !cfg!(target_os = "linux") {
        let reason = "native Linux CU requires target_os=linux".to_string();
        return (DisplayMode::None, ProbeStatus::Unsupported(reason));
    }
    if wayland_display {
        let caps = wayland::probe::probe_capabilities();
        if wayland::probe::caps_are_sufficient(&caps) {
            return (DisplayMode::Wayland(caps), ProbeStatus::Supported);
        }
        if !(x11_display && xdotool && scrot) {
            let reason = wayland::probe::unsupported_reason(&caps);
            return (DisplayMode::None, ProbeStatus::Unsupported(reason));
        }
    }
    if xdotool && scrot && x11_display {
        return (DisplayMode::X11, ProbeStatus::Supported);
    }
    let reason = if !xdotool {
        "xdotool not found on PATH"
    } else if !scrot {
        "scrot not found on PATH"
    } else {
        "DISPLAY is unset; X11 desktop not detected"
    };
    (DisplayMode::None, ProbeStatus::Unsupported(reason.into()))
}

pub(crate) fn ensure_supported() -> crate::LinuxCuResult<()> {
    let report = probe();
    if report.is_supported() {
        Ok(())
    } else {
        Err(crate::LinuxCuError::Unsupported {
            reason: match report.status {
                ProbeStatus::Supported => "supported".into(),
                ProbeStatus::Unsupported(reason) => reason,
            },
        })
    }
}

pub(crate) fn binary_on_path(binary: &str) -> bool {
    find_binary(binary).is_some()
}

pub(crate) fn find_binary(binary: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(binary))
        .find(|candidate| is_executable(candidate))
}

fn is_executable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_is_unsupported_off_linux() {
        if !cfg!(target_os = "linux") {
            assert!(!probe().is_supported());
        }
    }

    #[test]
    fn binary_lookup_returns_none_for_impossible_name() {
        assert!(find_binary("argo-definitely-not-a-real-binary").is_none());
    }

    #[test]
    fn display_mode_default_is_none() {
        assert_eq!(DisplayMode::default(), DisplayMode::None);
    }
}
