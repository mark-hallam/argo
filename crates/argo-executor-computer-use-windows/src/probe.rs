//! Runtime support probe for native Windows desktop computer use.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

/// Platform support status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeStatus {
    /// Windows + a usable PowerShell ≥ 5.1 are available.
    Supported,
    /// Host cannot run the native Windows POC.
    Unsupported(String),
}

/// Probe report for required Windows CU primitives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbeReport {
    /// Current target OS according to Rust cfg.
    pub platform: String,
    /// Whether `powershell.exe` is present on PATH.
    pub powershell: bool,
    /// Detected major version of PowerShell, if probed.
    pub powershell_major: Option<u32>,
    /// Whether an interactive desktop session appears to be present.
    pub interactive_session: bool,
    /// Final support decision.
    pub status: ProbeStatus,
}

impl ProbeReport {
    /// True when native Windows CU can run.
    #[must_use]
    pub fn is_supported(&self) -> bool {
        matches!(self.status, ProbeStatus::Supported)
    }
}

/// Check the current host for Windows CU support.
#[must_use]
pub fn probe() -> ProbeReport {
    let powershell = binary_on_path("powershell.exe");
    let powershell_major = if powershell {
        detect_powershell_major()
    } else {
        None
    };
    let interactive_session = has_interactive_session();
    let platform = std::env::consts::OS.to_string();
    let status = if !cfg!(target_os = "windows") {
        ProbeStatus::Unsupported("native Windows CU requires target_os=windows".into())
    } else if !powershell {
        ProbeStatus::Unsupported("powershell.exe not found on PATH".into())
    } else if powershell_major.unwrap_or(0) < 5 {
        ProbeStatus::Unsupported(
            "PowerShell ≥ 5.1 required (Windows 7+ ships it built-in)".into(),
        )
    } else if !interactive_session {
        ProbeStatus::Unsupported("no interactive session".into())
    } else {
        ProbeStatus::Supported
    };
    ProbeReport {
        platform,
        powershell,
        powershell_major,
        interactive_session,
        status,
    }
}

pub(crate) fn ensure_supported() -> crate::WindowsCuResult<()> {
    let report = probe();
    if report.is_supported() {
        Ok(())
    } else {
        Err(crate::WindowsCuError::Unsupported {
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
        .find(|candidate| candidate.is_file())
}

fn has_interactive_session() -> bool {
    if !cfg!(target_os = "windows") {
        return false;
    }
    std::env::var_os("SESSIONNAME")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
}

fn detect_powershell_major() -> Option<u32> {
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$PSVersionTable.PSVersion.Major",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    raw.trim().parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_is_unsupported_off_windows() {
        if !cfg!(target_os = "windows") {
            assert!(!probe().is_supported());
        }
    }

    #[test]
    fn binary_lookup_returns_none_for_impossible_name() {
        assert!(find_binary("argo-definitely-not-a-real-binary.exe").is_none());
    }

    #[test]
    fn probe_report_serialises() {
        let r = probe();
        let body = serde_json::to_string(&r).unwrap();
        assert!(body.contains("platform"));
    }

    #[test]
    fn off_windows_includes_target_os_reason() {
        if !cfg!(target_os = "windows") {
            let r = probe();
            match r.status {
                ProbeStatus::Unsupported(reason) => assert!(reason.contains("windows")),
                ProbeStatus::Supported => panic!("expected unsupported off windows"),
            }
        }
    }
}
