//! Risk-tiered computer-use floor (DEC-051, operator-locked).
//!
//! Binds each computer-use primitive's autonomy to whether it COMMITS an
//! action, enforced in the executor — independent of the manifest's
//! self-asserted `requires_approval` / `risk`. AUTONOMOUS operations
//! (screenshot, cursor read, wait, OCR, enumerate, cursor/mouse move, scroll,
//! and `type`) observe, navigate, or STAGE input and run autonomously; ACT
//! operations (click, `key`/hold_key — incl Enter/Return-as-submit, drag,
//! navigate, app launch, mutating osascript) commit/trigger a real effect and
//! are held for operator approval at the first occurrence in a run. Typing is
//! autonomous ONLY because the SUBMIT (`key`/click) stays ACT — typed text is
//! staged + reviewable before the gated commit (operator tiers, 2026-06-11).
//!
//! Fail closed: any primitive not explicitly on the autonomous allowlist —
//! including a vocabulary the run loop does not yet recognise —
//! classifies as ACT (DEC-041 boundary 4). This closes the cu3/#2816
//! self-assertion hole: a manifest can no longer stamp a clicking action
//! `risk=low, requires_approval=false` and earn autonomous host control once
//! `ARGO_EXECUTION_GRADUATION` flips — the executor floor holds regardless.

use crate::events::action_kind;
use argo_executor_computer_use::e2b::CuAction;

/// Reversibility tier the executor floor binds autonomy to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CuRiskTier {
    /// Reversible / read-only. Runs autonomously, no approval row.
    Observe,
    /// Mutating or host-controlling. Held for operator approval at the
    /// first occurrence in a run.
    Act,
}

impl CuRiskTier {
    /// True when the operation injects input or mutates host state and
    /// therefore requires the operator's approval before it runs.
    #[must_use]
    pub fn is_act(self) -> bool {
        matches!(self, CuRiskTier::Act)
    }
}

/// Classify a typed [`CuAction`].
///
/// Delegates to [`classify_kind`] through the canonical [`action_kind`]
/// label so the typed and string entry points share one source of truth
/// — there is no parallel allowlist to drift out of sync.
///
/// ```
/// use argo_executor_computer_use_macos::cu_risk::{classify, CuRiskTier};
/// use argo_executor_computer_use::e2b::CuAction;
/// // Every model-issued macOS CU primitive injects input or mutates
/// // host state, so all classify as ACT (held for approval).
/// assert_eq!(classify(&CuAction::Click { x: 1, y: 2 }), CuRiskTier::Act);
/// ```
#[must_use]
pub fn classify(action: &CuAction) -> CuRiskTier {
    classify_kind(action_kind(action))
}

/// Classify by the Anthropic CU action-kind label the run loop parses
/// (see [`action_kind`]).
///
/// The autonomous (OBSERVE) allowlist is the floor's source of truth;
/// everything else — committing primitives and any unrecognised op — is ACT.
/// Fail closed (DEC-041 boundary 4).
///
/// ```
/// use argo_executor_computer_use_macos::cu_risk::{classify_kind, CuRiskTier};
/// assert_eq!(classify_kind("screenshot"), CuRiskTier::Observe);
/// // DEC-051 operator tiers: typing/scroll/cursor-move are autonomous (staged);
/// // the SUBMIT (`key`/click) is the gated ACT.
/// assert_eq!(classify_kind("type"), CuRiskTier::Observe);
/// assert_eq!(classify_kind("key"), CuRiskTier::Act);
/// assert_eq!(classify_kind("click"), CuRiskTier::Act);
/// // Unknown vocabulary fails closed to ACT.
/// assert_eq!(classify_kind("some_new_primitive"), CuRiskTier::Act);
/// ```
#[must_use]
pub fn classify_kind(kind: &str) -> CuRiskTier {
    match kind {
        // AUTONOMOUS tier (DEC-051) — observe / navigate / STAGE input. None
        // commit a consequential or irreversible effect on their own:
        //  - observe: screenshot, cursor_position, wait, screen read/OCR, enumerate
        //  - navigate (reversible, side-effect-free): mouse_move, scroll
        //  - stage input: `type` — entering text into a field changes nothing
        //    irreversibly. Typing is autonomous ONLY because the operation that
        //    SUBMITS typed text — a `key` press (Enter/Return) or a `click` —
        //    stays in the ACT tier below (operator call, 2026-06-11). Typed text
        //    is staged + reviewable before the gated commit.
        "screenshot" | "cursor_position" | "wait" | "screen" | "screen_read" | "read" | "ocr"
        | "enumerate" | "mouse_move" | "scroll" | "type" => CuRiskTier::Observe,
        // ACT tier — operations that COMMIT/trigger an action: click,
        // double_click, `key`/hold_key (Enter/Return/Tab = submit), drag,
        // navigate, mouse-button input, app launch, mutating osascript, and any
        // unknown op. Fail closed (DEC-041 boundary 4).
        _ => CuRiskTier::Act,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committing_primitives_are_act_staging_primitives_are_autonomous() {
        // DEC-051 (classifier half, operator tiers 2026-06-11): a primitive that
        // COMMITS/triggers an action is ACT (held); one that only observes,
        // navigates, or STAGES input is autonomous.
        for action in [
            CuAction::Click { x: 1, y: 2 },
            CuAction::DoubleClick { x: 1, y: 2 },
            CuAction::Key {
                keys: vec!["cmd".into(), "n".into()],
            },
            CuAction::Navigate {
                url: "https://x".into(),
            },
        ] {
            assert!(
                classify(&action).is_act(),
                "{} commits → must be ACT",
                action.summary()
            );
        }
        // Staging / navigating: typing into a field, scrolling, moving the
        // cursor — none commit on their own (the submit `key`/`click` is ACT).
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
    fn observe_kinds_classify_as_observe() {
        for kind in [
            "screenshot",
            "cursor_position",
            "wait",
            "screen",
            "screen_read",
            "read",
            "ocr",
            "enumerate",
            // DEC-051 operator tiers: navigation + staged input join autonomous.
            "mouse_move",
            "scroll",
            "type",
        ] {
            assert_eq!(classify_kind(kind), CuRiskTier::Observe, "{kind} is observe");
            assert!(!classify_kind(kind).is_act());
        }
    }

    #[test]
    fn unknown_kind_fails_closed_to_act() {
        // DEC-051 acceptance #3: an unknown / unclassified op → ACT → held.
        assert_eq!(classify_kind("totally_unrecognised_op"), CuRiskTier::Act);
        assert_eq!(classify_kind(""), CuRiskTier::Act);
        // A real Anthropic primitive the loop does not model yet still
        // fails closed rather than slipping through as autonomous.
        assert!(classify_kind("left_click_drag").is_act());
        assert!(classify_kind("hold_key").is_act());
    }
}
