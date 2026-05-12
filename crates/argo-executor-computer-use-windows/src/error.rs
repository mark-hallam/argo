//! Error variants for the native Windows computer-use executor.
//!
//! Mirrors the structured-error pattern of
//! [`argo_executor_computer_use_macos::MacosCuError`] and
//! [`argo_executor_computer_use::CuError`] so the dispatcher can
//! decide handoff vs hard-fail per-variant.

use thiserror::Error;

/// Top-level error for the Windows executor.
#[derive(Debug, Error)]
pub enum WindowsCuError {
    /// Crate was called on a non-Windows host. Compiles on every
    /// platform; the runtime guard surfaces here when the executor
    /// is constructed or its methods are called from a Linux /
    /// macOS test runner.
    #[error("argo-executor-computer-use-windows requires Windows (target_os = \"windows\")")]
    NotWindows,

    /// A Win32 / COM call returned a non-success HRESULT.
    #[error("Win32 error in `{ctx}`: HRESULT {hresult:#010x}")]
    Win32Error {
        /// Hex-formatted HRESULT.
        hresult: u32,
        /// Operator-facing context string (e.g., "SendInput",
        /// "BitBlt", "IUIAutomation::ElementFromPoint").
        ctx: String,
    },

    /// UIAutomation could not find an element matching the
    /// `role` + `name` selector.
    #[error("UIAutomation lookup failed for {selector}")]
    A11yLookupFailed {
        /// Description of the selector used (role, name, automation_id).
        selector: String,
    },

    /// Screenshot capture (BitBlt or EnumDisplayMonitors) failed.
    #[error("screenshot failed: {reason}")]
    ScreenshotFailed {
        /// Reason the capture aborted (e.g., "no monitors found",
        /// "BitBlt returned 0", "session locked").
        reason: String,
    },

    /// SendInput silently no-op'd because the target session is
    /// in the secure-desktop state (UAC prompt visible, lock
    /// screen, Ctrl-Alt-Del menu). The OS routes input to the
    /// secure desktop and rejects user-mode injection.
    #[error("input blocked: secure desktop or UAC prompt is active")]
    InputBlocked,

    /// `EnumDisplayMonitors` returned zero monitors. Most often
    /// indicates a headless / disconnected RDP session, or a
    /// driver mis-configuration where the primary display has
    /// detached.
    #[error("no monitors enumerated by the OS")]
    MonitorEnumerationFailed,

    /// The target app's window class / process name is not in
    /// [`crate::WindowsCuConfig::allowed_apps`]. Defends against
    /// the model trying to escape the sandbox by activating an
    /// unrelated app.
    #[error("target app `{app}` is not in the allowed_apps list")]
    AppNotAllowed {
        /// The app id the executor refused (window class or
        /// process basename).
        app: String,
    },

    /// The CU model emitted an action the active config does not
    /// allow (e.g., destructive `key` combo).
    #[error("CU action `{kind}` is denied by the action allowlist for this app")]
    ActionNotAllowed {
        /// Short action kind ("type" / "click" / "key" / ...).
        kind: &'static str,
    },

    /// The active foreground window does not belong to the
    /// target app. Something stole focus between steps (UAC
    /// prompt, balloon notification, popup).
    #[error("expected foreground app `{expected}` but found `{actual}`")]
    FrontAppMismatch {
        /// App id expected to be foreground.
        expected: String,
        /// App id actually foreground.
        actual: String,
    },

    /// UAC elevation parity mismatch: target process is
    /// elevated, ours is not (or vice-versa). Windows blocks
    /// cross-IL SendInput silently.
    #[error("UAC integrity mismatch — argo runs at IL `{ours}`, target at IL `{target}`")]
    IntegrityMismatch {
        /// Integrity level of argo's own process.
        ours: String,
        /// Integrity level of the target process.
        target: String,
    },

    /// IO error from std::io.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    /// Anthropic CU client error bubbled up from the shared
    /// `argo-executor-computer-use::anthropic` module.
    #[error("anthropic: {0}")]
    Anthropic(String),

    /// Per-action wallclock or token budget exhausted. Caller
    /// translates to `Outcome::HumanHandoff`.
    #[error("budget exhausted: {reason}")]
    BudgetExhausted {
        /// Plain-English explanation for the audit log.
        reason: String,
    },

    /// Generic protocol / parse error (config validation, bad
    /// action shape, unparseable result).
    #[error("protocol: {0}")]
    Protocol(String),
}

impl WindowsCuError {
    /// Whether the dispatch path should map this to
    /// `Outcome::HumanHandoff` rather than fail the action.
    /// Mirrors `CuError::is_recoverable_to_handoff` from the E2B
    /// crate.
    #[must_use]
    pub fn is_recoverable_to_handoff(&self) -> bool {
        matches!(
            self,
            Self::FrontAppMismatch { .. }
                | Self::BudgetExhausted { .. }
                | Self::InputBlocked
                | Self::IntegrityMismatch { .. }
                | Self::Anthropic(_)
        )
    }

    /// Build a `Win32Error` from a raw HRESULT and a context
    /// label. The HRESULT is recorded as a `u32` (cast from `i32`)
    /// so it round-trips through error logs cleanly.
    #[must_use]
    pub fn win32(hresult: i32, ctx: impl Into<String>) -> Self {
        Self::Win32Error {
            hresult: hresult as u32,
            ctx: ctx.into(),
        }
    }
}

#[cfg(target_os = "windows")]
impl From<windows::core::Error> for WindowsCuError {
    fn from(err: windows::core::Error) -> Self {
        Self::Win32Error {
            hresult: err.code().0 as u32,
            ctx: err.message().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handoff_classification_matches_brief() {
        assert!(WindowsCuError::BudgetExhausted {
            reason: "tok".into()
        }
        .is_recoverable_to_handoff());
        assert!(WindowsCuError::FrontAppMismatch {
            expected: "a".into(),
            actual: "b".into()
        }
        .is_recoverable_to_handoff());
        assert!(WindowsCuError::InputBlocked.is_recoverable_to_handoff());
        assert!(WindowsCuError::IntegrityMismatch {
            ours: "medium".into(),
            target: "high".into()
        }
        .is_recoverable_to_handoff());
        assert!(!WindowsCuError::NotWindows.is_recoverable_to_handoff());
        assert!(!WindowsCuError::AppNotAllowed {
            app: "x".into()
        }
        .is_recoverable_to_handoff());
    }

    #[test]
    fn win32_helper_formats_hresult_hex() {
        let err = WindowsCuError::win32(-2147024891_i32, "SendInput");
        let s = err.to_string();
        assert!(s.contains("SendInput"));
        assert!(s.contains("0x"));
    }

    #[test]
    fn win32_helper_records_ctx_string() {
        let err = WindowsCuError::win32(0x0, "BitBlt");
        match err {
            WindowsCuError::Win32Error { ctx, .. } => assert_eq!(ctx, "BitBlt"),
            other => panic!("expected Win32Error, got {other:?}"),
        }
    }

    #[test]
    fn screenshot_failed_carries_reason() {
        let err = WindowsCuError::ScreenshotFailed {
            reason: "session locked".into(),
        };
        let s = err.to_string();
        assert!(s.contains("session locked"));
    }

    #[test]
    fn a11y_lookup_failed_carries_selector() {
        let err = WindowsCuError::A11yLookupFailed {
            selector: "role=Button name=Save".into(),
        };
        let s = err.to_string();
        assert!(s.contains("role=Button"));
        assert!(s.contains("Save"));
    }
}
