//! Error variants for the macOS computer-use executor.
//!
//! Mirrors the structured-error pattern of
//! [`argo_executor_computer_use::CuError`] so the dispatcher can
//! decide handoff vs hard-fail per-variant.

use thiserror::Error;

/// Top-level error for the macOS executor.
#[derive(Debug, Error)]
pub enum MacosCuError {
    /// Crate was called on a non-macOS host. Compiles on every
    /// platform; the runtime guard surfaces here when the executor
    /// is constructed or its methods are called from a Linux /
    /// Windows test runner.
    #[error("argo-executor-computer-use-macos requires macOS (target_os = \"macos\")")]
    NotMacos,

    /// One of the four required macOS permissions is not granted.
    /// `kind` is the System Settings panel name; `hint` is the
    /// operator-actionable next step.
    #[error("macOS permission denied — {kind}: {hint}")]
    PermissionDenied {
        /// Permission name as it appears in System Settings.
        kind: &'static str,
        /// Actionable next step for the operator.
        hint: String,
    },

    /// The target app's bundle id is not in
    /// [`crate::MacosCuConfig::allowed_apps`]. Defends against the
    /// model trying to escape the sandbox by activating an
    /// unrelated app.
    #[error("target app `{bundle_id}` is not in the allowed_apps list")]
    AppNotAllowed {
        /// The bundle id the executor refused.
        bundle_id: String,
    },

    /// The CU model emitted an action the active config does not
    /// allow (e.g., `Cmd+Q` or `MouseDrag`).
    #[error("CU action `{kind}` is denied by the action allowlist for this app")]
    ActionNotAllowed {
        /// Short action kind ("type" / "click" / "key" / ...).
        kind: &'static str,
    },

    /// The active front app is not the target. Either Spotlight,
    /// Notification Center, or another window grabbed focus
    /// between steps.
    #[error("expected front app `{expected}` but found `{actual}`")]
    FrontAppMismatch {
        /// Bundle id we expected to be active.
        expected: String,
        /// Bundle id that was actually active.
        actual: String,
    },

    /// A subprocess invocation (`screencapture`, `osascript`)
    /// exited non-zero. `command` is the failing CLI; `stderr` is
    /// the captured error text (truncated to 512 bytes).
    #[error("`{command}` failed: {stderr}")]
    Subprocess {
        /// CLI that failed.
        command: String,
        /// Stderr (truncated).
        stderr: String,
    },

    /// IO error from std::process::Command spawn or wait.
    #[error("subprocess io: {0}")]
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

    /// Action result couldn't be parsed (e.g., osascript returned
    /// unexpected text).
    #[error("protocol: {0}")]
    Protocol(String),
}

impl MacosCuError {
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
                | Self::Subprocess { .. }
                | Self::Anthropic(_)
        )
    }
}

/// Truncate a `stderr` capture to 512 bytes for the error message.
pub(crate) fn truncate_stderr(raw: &[u8]) -> String {
    let s = String::from_utf8_lossy(raw);
    if s.len() <= 512 {
        s.into_owned()
    } else {
        let mut out = s.chars().take(508).collect::<String>();
        out.push_str(" ...");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_keeps_short_strings_verbatim() {
        let s = truncate_stderr(b"oh no");
        assert_eq!(s, "oh no");
    }

    #[test]
    fn truncate_caps_long_strings() {
        let raw = "x".repeat(1024).into_bytes();
        let s = truncate_stderr(&raw);
        assert!(s.len() <= 512);
        assert!(s.ends_with(" ..."));
    }

    #[test]
    fn handoff_classification_matches_brief() {
        assert!(MacosCuError::BudgetExhausted {
            reason: "tok".into()
        }
        .is_recoverable_to_handoff());
        assert!(MacosCuError::FrontAppMismatch {
            expected: "a".into(),
            actual: "b".into()
        }
        .is_recoverable_to_handoff());
        assert!(!MacosCuError::NotMacos.is_recoverable_to_handoff());
        assert!(!MacosCuError::AppNotAllowed {
            bundle_id: "x".into()
        }
        .is_recoverable_to_handoff());
    }
}
