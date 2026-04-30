//! Iteration-budget extension policy (Amendment 9, §6.6).
//!
//! When `IterationBudget::charge` returns `Err` because the loop hit
//! `max_iterations`, this module decides what happens next:
//!
//! - **`Halt`** (default) — the turn ends; the user sees a "max
//!   iterations reached" surface.
//! - **`PromptUser`** — the loop pauses; the daemon asks the user
//!   "the agent reached its iteration cap; extend by N more?".
//! - **`AutoContinue`** — the loop transparently extends by
//!   `extension_size`, but **never silently** beyond
//!   `max_extensions × extension_size + initial`.
//!
//! The hard ceiling is the spec invariant: `decide` returns `Halt`
//! once `extensions_used >= max_extensions`, regardless of policy.
//! That's the "the loop never silently extends beyond ..." guarantee.
use serde::{Deserialize, Serialize};

/// Policy on hitting `max_iterations`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IterationOnMax {
    /// Terminate the turn.
    #[default]
    Halt,
    /// Pause; surface to the user; act on their answer.
    PromptUser,
    /// Auto-extend by `extension_size`, capped by
    /// `max_extensions`.
    AutoContinue,
}

/// Tunables governing the extension loop.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ExtensionLimits {
    /// Hard cap on how many auto-extensions are allowed.
    pub max_extensions: u32,
    /// Iterations granted per extension.
    pub extension_size: u32,
}

impl Default for ExtensionLimits {
    fn default() -> Self {
        // Spec defaults from `[budget]`.
        Self {
            max_extensions: 3,
            extension_size: 30,
        }
    }
}

/// Decision the loop should act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionDecision {
    /// Terminate the turn.
    Halt,
    /// Wait for user input; on approval, callers re-invoke with
    /// `extensions_used` decremented if the user denies, or take
    /// `AutoExtend { new_max }` semantics if they accept.
    PromptUser {
        /// What the new `max_iterations` would be after granting.
        new_max_if_approved: u32,
    },
    /// Auto-extend; the loop should set `max_iterations = new_max`.
    AutoExtend {
        /// New `max_iterations` after this extension.
        new_max: u32,
    },
}

/// Decide what to do when the loop hits its current cap.
///
/// `current_max` is the existing iteration cap (initial + any prior
/// extensions); `extensions_used` counts extensions already
/// applied. The hard ceiling kicks in when `extensions_used >=
/// limits.max_extensions` regardless of policy.
#[must_use]
pub fn decide(
    policy: IterationOnMax,
    current_max: u32,
    extensions_used: u32,
    limits: ExtensionLimits,
) -> ExtensionDecision {
    if extensions_used >= limits.max_extensions {
        return ExtensionDecision::Halt;
    }
    let new_max = current_max.saturating_add(limits.extension_size);
    match policy {
        IterationOnMax::Halt => ExtensionDecision::Halt,
        IterationOnMax::PromptUser => ExtensionDecision::PromptUser {
            new_max_if_approved: new_max,
        },
        IterationOnMax::AutoContinue => ExtensionDecision::AutoExtend { new_max },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> ExtensionLimits {
        ExtensionLimits {
            max_extensions: 3,
            extension_size: 30,
        }
    }

    #[test]
    fn halt_policy_always_halts() {
        let d = decide(IterationOnMax::Halt, 60, 0, limits());
        assert_eq!(d, ExtensionDecision::Halt);
    }

    #[test]
    fn prompt_user_policy_proposes_new_max() {
        let d = decide(IterationOnMax::PromptUser, 60, 0, limits());
        assert_eq!(
            d,
            ExtensionDecision::PromptUser {
                new_max_if_approved: 90,
            }
        );
    }

    #[test]
    fn auto_continue_extends_by_extension_size() {
        let d = decide(IterationOnMax::AutoContinue, 60, 0, limits());
        assert_eq!(d, ExtensionDecision::AutoExtend { new_max: 90 });
    }

    #[test]
    fn auto_continue_halts_at_hard_ceiling() {
        // §6.6: "The loop never silently extends beyond
        // max_extensions × extension_size + initial."
        let d = decide(IterationOnMax::AutoContinue, 150, 3, limits());
        assert_eq!(d, ExtensionDecision::Halt);
    }

    #[test]
    fn prompt_user_also_halts_at_hard_ceiling() {
        // The hard ceiling overrides every policy.
        let d = decide(IterationOnMax::PromptUser, 150, 3, limits());
        assert_eq!(d, ExtensionDecision::Halt);
    }

    #[test]
    fn nth_auto_extension_chains_correctly() {
        // 60 + 30 + 30 + 30 = 150 across 3 extensions; 4th extension
        // is blocked by the hard ceiling.
        let mut current = 60;
        for used in 0..3 {
            match decide(IterationOnMax::AutoContinue, current, used, limits()) {
                ExtensionDecision::AutoExtend { new_max } => current = new_max,
                other => panic!("unexpected decision at used={used}: {other:?}"),
            }
        }
        assert_eq!(current, 150);
        let final_d = decide(IterationOnMax::AutoContinue, current, 3, limits());
        assert_eq!(final_d, ExtensionDecision::Halt);
    }

    #[test]
    fn default_policy_is_halt() {
        // Spec invariant: "Default off." A fresh-config user must
        // not silently auto-continue.
        assert_eq!(IterationOnMax::default(), IterationOnMax::Halt);
    }
}
