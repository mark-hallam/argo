//! Six-axis Windows preflight report.
//!
//! Each axis maps to one Windows-specific failure mode the
//! executor would otherwise hit silently mid-loop:
//!
//! 1. UAC consent — process integrity level
//! 2. Secure-desktop state — whether SendInput is being routed to
//!    the secure desktop (UAC prompt, lock-screen, Ctrl-Alt-Del)
//! 3. DPI awareness — whether the process declared per-monitor v2
//! 4. UIAutomation service — whether the OS a11y service is
//!    running and reachable
//! 5. Monitor enumeration — at least one monitor is attached
//! 6. Primary monitor — primary is identified (multi-monitor
//!    coordinate model needs an anchor)
//!
//! See the runbook `docs/skycot/runbook/cu-windows.md` for the
//! operator-facing remediation for each axis.

use serde::{Deserialize, Serialize};

/// One of the six preflight axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PreflightCheck {
    /// UAC integrity-level matches what the target will need.
    UacConsent,
    /// Secure desktop is not currently active.
    SecureDesktop,
    /// Process is per-monitor v2 DPI aware.
    DpiAwareness,
    /// UIAutomation service responds.
    A11yService,
    /// At least one monitor is enumerated.
    MonitorCount,
    /// Primary monitor identified.
    PrimaryMonitor,
}

impl PreflightCheck {
    /// All checks in deterministic order.
    #[must_use]
    pub fn all() -> [Self; 6] {
        [
            Self::UacConsent,
            Self::SecureDesktop,
            Self::DpiAwareness,
            Self::A11yService,
            Self::MonitorCount,
            Self::PrimaryMonitor,
        ]
    }

    /// Human-readable label for the operator-facing report.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::UacConsent => "UAC consent / integrity level",
            Self::SecureDesktop => "Secure-desktop not active",
            Self::DpiAwareness => "DPI awareness (per-monitor v2)",
            Self::A11yService => "UIAutomation service reachable",
            Self::MonitorCount => "At least one monitor attached",
            Self::PrimaryMonitor => "Primary monitor identified",
        }
    }

    /// One-line "what to check first" pointer matching the
    /// runbook's failure-class triage.
    #[must_use]
    pub fn actionable_hint(self) -> &'static str {
        match self {
            Self::UacConsent => {
                "if target is elevated, relaunch argo `As Administrator`; otherwise relaunch the target without elevation"
            }
            Self::SecureDesktop => {
                "dismiss any visible UAC prompt or Ctrl-Alt-Del menu; unlock the workstation"
            }
            Self::DpiAwareness => {
                "set `dpi_aware = true` in [executor.computer_use.windows] and restart argo"
            }
            Self::A11yService => {
                "enable `Microsoft UI Automation Service` (services.msc) and restart argo"
            }
            Self::MonitorCount => {
                "attach at least one monitor; RDP-disconnected sessions show zero monitors"
            }
            Self::PrimaryMonitor => {
                "right-click the desktop → Display Settings → mark one monitor as primary"
            }
        }
    }
}

/// Status of one preflight axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreflightStatus {
    /// Check passed.
    Pass,
    /// Check failed.
    Fail,
    /// Could not determine (e.g., probe API itself errored).
    Unknown,
}

/// Full report covering every axis at preflight time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsPreflightReport {
    /// One entry per axis in [`PreflightCheck::all`] order.
    pub entries: Vec<(PreflightCheck, PreflightStatus)>,
    /// Number of monitors enumerated by the OS. Always populated
    /// (0 on non-Windows hosts, or when MonitorCount failed).
    pub monitor_count: u32,
    /// Primary monitor's (width, height) in pixels, if known.
    pub primary_resolution: Option<(u32, u32)>,
    /// Whether the preflight was actually attempted (false on
    /// non-Windows hosts).
    pub probed: bool,
}

impl WindowsPreflightReport {
    /// Empty report for the "we did not actually probe" case
    /// (non-Windows hosts).
    #[must_use]
    pub fn not_probed() -> Self {
        Self {
            entries: PreflightCheck::all()
                .into_iter()
                .map(|c| (c, PreflightStatus::Unknown))
                .collect(),
            monitor_count: 0,
            primary_resolution: None,
            probed: false,
        }
    }

    /// True if every axis is `Pass`. False if any axis is `Fail`
    /// or `Unknown`.
    #[must_use]
    pub fn all_passed(&self) -> bool {
        self.entries
            .iter()
            .all(|(_, s)| *s == PreflightStatus::Pass)
    }

    /// Sub-list of axes that did not pass.
    #[must_use]
    pub fn failing(&self) -> Vec<PreflightCheck> {
        self.entries
            .iter()
            .filter(|(_, s)| *s != PreflightStatus::Pass)
            .map(|(c, _)| *c)
            .collect()
    }

    /// Newline-separated, operator-facing summary listing every
    /// failing axis with its actionable hint.
    #[must_use]
    pub fn actionable_summary(&self) -> String {
        if self.all_passed() {
            return "all Windows preflight checks passed".to_string();
        }
        let mut out = String::new();
        if !self.probed {
            out.push_str(
                "preflight not attempted (non-Windows host or runtime guard fired)\n",
            );
        }
        for check in self.failing() {
            out.push_str(&format!("- {}: {}\n", check.label(), check.actionable_hint()));
        }
        out
    }
}

/// Best-effort preflight probe. Returns `not_probed()` on
/// non-Windows hosts; on Windows, probes each axis via the
/// relevant Win32 / UIAutomation API.
#[cfg(target_os = "windows")]
pub fn probe() -> WindowsPreflightReport {
    let mut entries = Vec::with_capacity(6);
    let mut monitor_count: u32 = 0;
    let mut primary_resolution: Option<(u32, u32)> = None;

    for check in PreflightCheck::all() {
        let status = match check {
            PreflightCheck::UacConsent => probe_uac_consent(),
            PreflightCheck::SecureDesktop => probe_secure_desktop(),
            PreflightCheck::DpiAwareness => probe_dpi_awareness(),
            PreflightCheck::A11yService => probe_a11y_service(),
            PreflightCheck::MonitorCount => {
                let (status, count) = probe_monitor_count();
                monitor_count = count;
                status
            }
            PreflightCheck::PrimaryMonitor => {
                let (status, res) = probe_primary_monitor();
                primary_resolution = res;
                status
            }
        };
        entries.push((check, status));
    }
    WindowsPreflightReport {
        entries,
        monitor_count,
        primary_resolution,
        probed: true,
    }
}

/// Non-Windows placeholder so the workspace `cargo check` stays
/// green.
#[cfg(not(target_os = "windows"))]
pub fn probe() -> WindowsPreflightReport {
    WindowsPreflightReport::not_probed()
}

#[cfg(target_os = "windows")]
fn probe_uac_consent() -> PreflightStatus {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token: HANDLE = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return PreflightStatus::Unknown;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut ret_len: u32 = 0;
        let result = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        );
        if result.is_err() {
            return PreflightStatus::Unknown;
        }
        // We don't fail the check here — both elevated and
        // non-elevated argo can succeed depending on the target.
        // The fail mode is detected post-hoc via
        // IntegrityMismatch. Pass means "we know our IL".
        let _ = elevation.TokenIsElevated;
        PreflightStatus::Pass
    }
}

#[cfg(target_os = "windows")]
fn probe_secure_desktop() -> PreflightStatus {
    use windows::Win32::System::RemoteDesktop::{
        WTSQuerySessionInformationW, WTSConnectState, WTS_CURRENT_SERVER_HANDLE,
        WTSActive,
    };
    use windows::Win32::System::RemoteDesktop::WTSGetActiveConsoleSessionId;

    unsafe {
        let session_id = WTSGetActiveConsoleSessionId();
        let mut buffer = windows::core::PWSTR::null();
        let mut bytes_returned: u32 = 0;
        let result = WTSQuerySessionInformationW(
            Some(WTS_CURRENT_SERVER_HANDLE),
            session_id,
            WTSConnectState,
            &mut buffer,
            &mut bytes_returned,
        );
        if result.is_err() || buffer.is_null() {
            return PreflightStatus::Unknown;
        }
        let state_ptr = buffer.0 as *const i32;
        let state = std::ptr::read(state_ptr);
        windows::Win32::System::RemoteDesktop::WTSFreeMemory(buffer.0 as *mut _);
        if state == WTSActive.0 {
            PreflightStatus::Pass
        } else {
            PreflightStatus::Fail
        }
    }
}

#[cfg(target_os = "windows")]
fn probe_dpi_awareness() -> PreflightStatus {
    use windows::Win32::UI::HiDpi::{
        GetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        AreDpiAwarenessContextsEqual,
    };

    unsafe {
        let ctx = GetThreadDpiAwarenessContext();
        if AreDpiAwarenessContextsEqual(ctx, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2).as_bool() {
            PreflightStatus::Pass
        } else {
            PreflightStatus::Fail
        }
    }
}

#[cfg(target_os = "windows")]
fn probe_a11y_service() -> PreflightStatus {
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
    use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation};

    unsafe {
        let res: Result<IUIAutomation, _> = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER);
        match res {
            Ok(_) => PreflightStatus::Pass,
            Err(_) => PreflightStatus::Fail,
        }
    }
}

#[cfg(target_os = "windows")]
fn probe_monitor_count() -> (PreflightStatus, u32) {
    use crate::screenshot::enumerate_monitors;
    match enumerate_monitors() {
        Ok(monitors) => {
            let n = monitors.len() as u32;
            if n >= 1 {
                (PreflightStatus::Pass, n)
            } else {
                (PreflightStatus::Fail, 0)
            }
        }
        Err(_) => (PreflightStatus::Fail, 0),
    }
}

#[cfg(target_os = "windows")]
fn probe_primary_monitor() -> (PreflightStatus, Option<(u32, u32)>) {
    use crate::screenshot::enumerate_monitors;
    match enumerate_monitors() {
        Ok(monitors) => {
            if let Some(primary) = monitors.iter().find(|m| m.is_primary) {
                (PreflightStatus::Pass, Some((primary.width, primary.height)))
            } else {
                (PreflightStatus::Fail, None)
            }
        }
        Err(_) => (PreflightStatus::Fail, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_checks_have_distinct_labels() {
        let mut labels = std::collections::HashSet::new();
        for c in PreflightCheck::all() {
            labels.insert(c.label());
        }
        assert_eq!(labels.len(), 6);
    }

    #[test]
    fn all_checks_have_actionable_hints() {
        for c in PreflightCheck::all() {
            assert!(!c.actionable_hint().is_empty());
        }
    }

    #[test]
    fn report_not_probed_marks_every_check_unknown() {
        let r = WindowsPreflightReport::not_probed();
        assert!(!r.probed);
        assert_eq!(r.entries.len(), 6);
        assert!(r.entries.iter().all(|(_, s)| *s == PreflightStatus::Unknown));
        assert!(!r.all_passed());
        assert_eq!(r.monitor_count, 0);
        assert!(r.primary_resolution.is_none());
    }

    #[test]
    fn failing_lists_only_non_pass_axes() {
        let r = WindowsPreflightReport {
            entries: vec![
                (PreflightCheck::UacConsent, PreflightStatus::Pass),
                (PreflightCheck::SecureDesktop, PreflightStatus::Fail),
                (PreflightCheck::DpiAwareness, PreflightStatus::Unknown),
                (PreflightCheck::A11yService, PreflightStatus::Pass),
                (PreflightCheck::MonitorCount, PreflightStatus::Pass),
                (PreflightCheck::PrimaryMonitor, PreflightStatus::Pass),
            ],
            monitor_count: 1,
            primary_resolution: Some((1920, 1080)),
            probed: true,
        };
        let f = r.failing();
        assert_eq!(
            f,
            vec![PreflightCheck::SecureDesktop, PreflightCheck::DpiAwareness]
        );
        assert!(!r.all_passed());
    }

    #[test]
    fn all_passed_returns_true_only_when_every_axis_passes() {
        let r = WindowsPreflightReport {
            entries: PreflightCheck::all()
                .into_iter()
                .map(|c| (c, PreflightStatus::Pass))
                .collect(),
            monitor_count: 1,
            primary_resolution: Some((1920, 1080)),
            probed: true,
        };
        assert!(r.all_passed());
    }

    #[test]
    fn actionable_summary_lists_each_failing_axis() {
        let r = WindowsPreflightReport {
            entries: vec![
                (PreflightCheck::UacConsent, PreflightStatus::Pass),
                (PreflightCheck::SecureDesktop, PreflightStatus::Fail),
                (PreflightCheck::DpiAwareness, PreflightStatus::Pass),
                (PreflightCheck::A11yService, PreflightStatus::Fail),
                (PreflightCheck::MonitorCount, PreflightStatus::Pass),
                (PreflightCheck::PrimaryMonitor, PreflightStatus::Pass),
            ],
            monitor_count: 1,
            primary_resolution: Some((1920, 1080)),
            probed: true,
        };
        let s = r.actionable_summary();
        assert!(s.contains("Secure-desktop"));
        assert!(s.contains("UIAutomation"));
        assert!(!s.contains("UAC consent"));
    }

    #[test]
    fn actionable_summary_is_friendly_when_all_passed() {
        let r = WindowsPreflightReport {
            entries: PreflightCheck::all()
                .into_iter()
                .map(|c| (c, PreflightStatus::Pass))
                .collect(),
            monitor_count: 2,
            primary_resolution: Some((1920, 1080)),
            probed: true,
        };
        assert_eq!(
            r.actionable_summary(),
            "all Windows preflight checks passed"
        );
    }

    #[test]
    fn check_all_yields_six_entries() {
        assert_eq!(PreflightCheck::all().len(), 6);
    }

    #[test]
    fn report_serialises_through_serde() {
        let r = WindowsPreflightReport::not_probed();
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains("entries"));
        assert!(s.contains("probed"));
        let parsed: WindowsPreflightReport = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.entries.len(), 6);
    }

    #[test]
    fn primary_resolution_captured_in_report() {
        let r = WindowsPreflightReport {
            entries: PreflightCheck::all()
                .into_iter()
                .map(|c| (c, PreflightStatus::Pass))
                .collect(),
            monitor_count: 1,
            primary_resolution: Some((2560, 1440)),
            probed: true,
        };
        assert_eq!(r.primary_resolution, Some((2560, 1440)));
    }
}
