//! Permission preflight: check the four macOS permissions the
//! executor needs before any session starts.
//!
//! The four are documented in
//! [`docs/skycot/COMPUTER_USE_NATIVE_MACOS.md`] §4. CI runners
//! grant none of them by default; the smoke test gates on a
//! successful preflight to skip rather than fail in unprivileged
//! environments.

use serde::{Deserialize, Serialize};

/// One of the four macOS permissions the executor needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PermissionKind {
    /// `CGWindowListCreateImage` / `screencapture` — taking
    /// screenshots.
    ScreenRecording,
    /// AXUIElement* + posting CGEvent — accessibility framework.
    Accessibility,
    /// Some `CGEvent` keystroke flavors — input monitoring.
    InputMonitoring,
    /// AppleScript control of target apps — automation.
    Automation,
}

impl PermissionKind {
    /// All four permissions in deterministic order. Iteration
    /// helper for preflight + reporting.
    #[must_use]
    pub fn all() -> [Self; 4] {
        [
            Self::ScreenRecording,
            Self::Accessibility,
            Self::InputMonitoring,
            Self::Automation,
        ]
    }

    /// Human-readable name as it appears in System Settings.
    #[must_use]
    pub fn display_name(self) -> &'static str {
        match self {
            Self::ScreenRecording => "Screen Recording",
            Self::Accessibility => "Accessibility",
            Self::InputMonitoring => "Input Monitoring",
            Self::Automation => "Automation",
        }
    }

    /// Apple System Settings deep-link to the relevant pane.
    #[must_use]
    pub fn settings_deep_link(self) -> &'static str {
        match self {
            Self::ScreenRecording => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"
            }
            Self::Accessibility => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
            }
            Self::InputMonitoring => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"
            }
            Self::Automation => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Automation"
            }
        }
    }

    /// Operator-actionable hint for granting this permission.
    #[must_use]
    pub fn actionable_hint(self) -> String {
        format!(
            "open `{}` and add the binary running argo to the {} list",
            self.settings_deep_link(),
            self.display_name()
        )
    }
}

/// Status of one permission at preflight time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionStatus {
    /// Permission is granted (best-effort detection).
    Granted,
    /// Permission is missing.
    Missing,
    /// Could not determine (e.g., probing the API failed for an
    /// unrelated reason). Treated as "missing" for the
    /// `all_granted` check but reported separately.
    Unknown,
}

/// Snapshot of all four permissions at preflight time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionReport {
    /// One entry per permission in [`PermissionKind::all`] order.
    pub entries: Vec<(PermissionKind, PermissionStatus)>,
    /// Whether the preflight was even attempted (false on
    /// non-macOS hosts).
    pub probed: bool,
}

impl PermissionReport {
    /// Empty report for the "we did not actually probe" case
    /// (non-macOS hosts).
    #[must_use]
    pub fn not_probed() -> Self {
        Self {
            entries: PermissionKind::all()
                .into_iter()
                .map(|k| (k, PermissionStatus::Unknown))
                .collect(),
            probed: false,
        }
    }

    /// Whether every permission is granted. False if any is
    /// `Missing` or `Unknown`.
    #[must_use]
    pub fn all_granted(&self) -> bool {
        self.entries
            .iter()
            .all(|(_, s)| *s == PermissionStatus::Granted)
    }

    /// Sub-list of missing permissions for operator-facing
    /// reporting.
    #[must_use]
    pub fn missing(&self) -> Vec<PermissionKind> {
        self.entries
            .iter()
            .filter(|(_, s)| *s != PermissionStatus::Granted)
            .map(|(k, _)| *k)
            .collect()
    }

    /// Newline-separated, operator-facing summary listing every
    /// missing permission and its actionable hint.
    #[must_use]
    pub fn actionable_summary(&self) -> String {
        if self.all_granted() {
            return "all macOS permissions granted".to_string();
        }
        let mut out = String::new();
        if !self.probed {
            out.push_str(
                "preflight not attempted (non-macOS host or runtime guard fired)\n",
            );
        }
        for kind in self.missing() {
            out.push_str(&format!("- {}: {}\n", kind.display_name(), kind.actionable_hint()));
        }
        out
    }
}

/// Best-effort permission probe. Returns `not_probed()` on
/// non-macOS hosts; on macOS, probes each permission via a
/// dry-run subprocess invocation (e.g., `screencapture -t png
/// -x /dev/null` to test Screen Recording).
///
/// The probe is **best-effort** — macOS doesn't expose a clean
/// "is permission granted" API, so we infer from probe success.
/// False negatives are possible if the operator already saw the
/// permission prompt during a previous run.
#[cfg(target_os = "macos")]
pub fn probe() -> PermissionReport {
    let mut entries = Vec::with_capacity(4);
    for kind in PermissionKind::all() {
        let status = match kind {
            PermissionKind::ScreenRecording => probe_screen_recording(),
            PermissionKind::Accessibility => probe_accessibility(),
            PermissionKind::InputMonitoring => probe_input_monitoring(),
            PermissionKind::Automation => probe_automation(),
        };
        entries.push((kind, status));
    }
    PermissionReport {
        entries,
        probed: true,
    }
}

/// Non-macOS placeholder so the workspace `cargo check` stays
/// green.
#[cfg(not(target_os = "macos"))]
pub fn probe() -> PermissionReport {
    PermissionReport::not_probed()
}

#[cfg(target_os = "macos")]
fn probe_screen_recording() -> PermissionStatus {
    // Best-effort: try to capture one PNG to /dev/null. If
    // `screencapture` exits 0, we can record. -x silences shutter
    // sound, -t png picks the format.
    use std::process::Command;
    match Command::new("screencapture")
        .args(["-x", "-t", "png", "/dev/null"])
        .status()
    {
        Ok(status) if status.success() => PermissionStatus::Granted,
        Ok(_) => PermissionStatus::Missing,
        Err(_) => PermissionStatus::Unknown,
    }
}

#[cfg(target_os = "macos")]
fn probe_accessibility() -> PermissionStatus {
    // Best-effort: ask System Events to list its own processes.
    // If accessibility is denied, osascript exits 1 with `not
    // authorised to send Apple events to System Events`.
    use std::process::Command;
    match Command::new("osascript")
        .args(["-e", "tell application \"System Events\" to count processes"])
        .output()
    {
        Ok(out) if out.status.success() => PermissionStatus::Granted,
        Ok(_) => PermissionStatus::Missing,
        Err(_) => PermissionStatus::Unknown,
    }
}

#[cfg(target_os = "macos")]
fn probe_input_monitoring() -> PermissionStatus {
    // Input Monitoring isn't probable from script-land — there's
    // no benign keystroke we can post that surfaces a status.
    // Report Unknown so `actionable_summary` still tells the
    // operator to grant it.
    PermissionStatus::Unknown
}

#[cfg(target_os = "macos")]
fn probe_automation() -> PermissionStatus {
    // Best-effort: ask Notes for its process id. If automation
    // for Notes is denied, osascript exits 1.
    use std::process::Command;
    match Command::new("osascript")
        .args(["-e", "tell application id \"com.apple.Notes\" to get its name"])
        .output()
    {
        Ok(out) if out.status.success() => PermissionStatus::Granted,
        Ok(_) => PermissionStatus::Missing,
        Err(_) => PermissionStatus::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_kinds_have_distinct_settings_links() {
        let mut links = std::collections::HashSet::new();
        for kind in PermissionKind::all() {
            links.insert(kind.settings_deep_link());
        }
        assert_eq!(links.len(), 4);
    }

    #[test]
    fn all_kinds_have_distinct_display_names() {
        let mut names = std::collections::HashSet::new();
        for kind in PermissionKind::all() {
            names.insert(kind.display_name());
        }
        assert_eq!(names.len(), 4);
    }

    #[test]
    fn report_not_probed_marks_every_kind_unknown() {
        let r = PermissionReport::not_probed();
        assert!(!r.probed);
        assert_eq!(r.entries.len(), 4);
        assert!(r.entries.iter().all(|(_, s)| *s == PermissionStatus::Unknown));
        assert!(!r.all_granted());
    }

    #[test]
    fn missing_lists_only_non_granted() {
        let r = PermissionReport {
            entries: vec![
                (PermissionKind::ScreenRecording, PermissionStatus::Granted),
                (PermissionKind::Accessibility, PermissionStatus::Missing),
                (PermissionKind::InputMonitoring, PermissionStatus::Unknown),
                (PermissionKind::Automation, PermissionStatus::Granted),
            ],
            probed: true,
        };
        let m = r.missing();
        assert_eq!(m, vec![PermissionKind::Accessibility, PermissionKind::InputMonitoring]);
        assert!(!r.all_granted());
    }

    #[test]
    fn all_granted_returns_true_only_when_all_granted() {
        let r = PermissionReport {
            entries: PermissionKind::all()
                .into_iter()
                .map(|k| (k, PermissionStatus::Granted))
                .collect(),
            probed: true,
        };
        assert!(r.all_granted());
    }

    #[test]
    fn actionable_summary_lists_each_missing_kind() {
        let r = PermissionReport {
            entries: vec![
                (PermissionKind::ScreenRecording, PermissionStatus::Missing),
                (PermissionKind::Accessibility, PermissionStatus::Granted),
                (PermissionKind::InputMonitoring, PermissionStatus::Missing),
                (PermissionKind::Automation, PermissionStatus::Granted),
            ],
            probed: true,
        };
        let s = r.actionable_summary();
        assert!(s.contains("Screen Recording"));
        assert!(s.contains("Input Monitoring"));
        assert!(!s.contains("Accessibility"));
        assert!(!s.contains("Automation"));
    }

    #[test]
    fn actionable_summary_is_friendly_when_all_granted() {
        let r = PermissionReport {
            entries: PermissionKind::all()
                .into_iter()
                .map(|k| (k, PermissionStatus::Granted))
                .collect(),
            probed: true,
        };
        assert_eq!(r.actionable_summary(), "all macOS permissions granted");
    }

    #[test]
    fn settings_deep_links_use_apple_url_scheme() {
        for kind in PermissionKind::all() {
            assert!(kind.settings_deep_link().starts_with("x-apple.systempreferences:"));
        }
    }
}
