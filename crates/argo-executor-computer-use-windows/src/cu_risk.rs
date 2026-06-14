//! Risk-tiered computer-use floor (DEC-051, operator-locked) — Windows.
//!
//! Mirror of `argo-executor-computer-use-macos/src/cu_risk.rs` (the classifier
//! is platform-neutral but, like macOS, lives per-crate until it is promoted to
//! the shared `argo-executor-computer-use` crate). Binds each CU primitive's
//! autonomy to whether it COMMITS an action: AUTONOMOUS ops (observe / navigate
//! / STAGE input incl `type`) run autonomously; ACT ops (click, `key`, drag,
//! navigate, app launch) are held for operator approval at the first occurrence.
//! Fail closed: any op not on the autonomous allowlist — including unrecognised
//! vocabulary — classifies as ACT (DEC-041 boundary 4).

use crate::events::action_kind;
use argo_executor_computer_use::e2b::CuAction;

/// Reversibility tier the executor floor binds autonomy to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CuRiskTier {
    /// Reversible / read-only. Runs autonomously, no approval row.
    Observe,
    /// Mutating or host-controlling. Held for operator approval.
    Act,
}

impl CuRiskTier {
    /// True when the op injects committing input or mutates host state.
    #[must_use]
    pub fn is_act(self) -> bool {
        matches!(self, CuRiskTier::Act)
    }
}

/// Classify a typed [`CuAction`] via the canonical [`action_kind`] label.
///
/// ```
/// use argo_executor_computer_use_windows::cu_risk::{classify, CuRiskTier};
/// use argo_executor_computer_use::e2b::CuAction;
/// assert_eq!(classify(&CuAction::Click { x: 1, y: 2 }), CuRiskTier::Act);
/// assert_eq!(classify(&CuAction::Type { text: "x".into() }), CuRiskTier::Observe);
/// ```
#[must_use]
pub fn classify(action: &CuAction) -> CuRiskTier {
    classify_kind(action_kind(action))
}

/// Classify by the Anthropic CU action-kind label. Autonomous allowlist is the
/// source of truth; everything else (committing + unknown) is ACT (fail closed).
#[must_use]
pub fn classify_kind(kind: &str) -> CuRiskTier {
    match kind {
        "screenshot" | "cursor_position" | "wait" | "screen" | "screen_read" | "read" | "ocr"
        | "enumerate" | "mouse_move" | "scroll" | "type" => CuRiskTier::Observe,
        _ => CuRiskTier::Act,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committing_primitives_are_act_staging_primitives_are_autonomous() {
        for action in [
            CuAction::Click { x: 1, y: 2 },
            CuAction::DoubleClick { x: 1, y: 2 },
            CuAction::Key {
                keys: vec!["ctrl".into(), "n".into()],
            },
            CuAction::Navigate {
                url: "https://x".into(),
            },
        ] {
            assert!(classify(&action).is_act(), "{} commits → ACT", action.summary());
        }
        for action in [
            CuAction::Type { text: "x".into() },
            CuAction::Scroll { x: 0, y: 0, dy: 3 },
            CuAction::MouseMove { x: 5, y: 5 },
        ] {
            assert_eq!(
                classify(&action),
                CuRiskTier::Observe,
                "{} stages/navigates → autonomous",
                action.summary()
            );
        }
    }

    #[test]
    fn unknown_kind_fails_closed_to_act() {
        assert_eq!(classify_kind("totally_unrecognised_op"), CuRiskTier::Act);
        assert_eq!(classify_kind(""), CuRiskTier::Act);
        assert!(classify_kind("left_click_drag").is_act());
        assert!(classify_kind("hold_key").is_act());
    }
}
