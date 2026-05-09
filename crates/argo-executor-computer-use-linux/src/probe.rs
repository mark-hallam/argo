//! Runtime support probe for Linux desktop computer use.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Platform support status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeStatus {
    /// Linux + required X11 binaries are available.
    Supported,
    /// Host cannot run the native Linux PoC.
    Unsupported(String),
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

/// Check the current host for Linux CU support.
#[must_use]
pub fn probe() -> ProbeReport {
    let xdotool = binary_on_path("xdotool");
    let scrot = binary_on_path("scrot");
    let x11_display = std::env::var_os("DISPLAY").is_some();
    let platform = std::env::consts::OS.to_string();
    let status = if !cfg!(target_os = "linux") {
        ProbeStatus::Unsupported("native Linux CU requires target_os=linux".into())
    } else if !xdotool {
        ProbeStatus::Unsupported("xdotool not found on PATH".into())
    } else if !scrot {
        ProbeStatus::Unsupported("scrot not found on PATH".into())
    } else if !x11_display {
        ProbeStatus::Unsupported("DISPLAY is unset; X11 desktop not detected".into())
    } else {
        ProbeStatus::Supported
    };
    ProbeReport {
        platform,
        xdotool,
        scrot,
        x11_display,
        status,
    }
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
}
