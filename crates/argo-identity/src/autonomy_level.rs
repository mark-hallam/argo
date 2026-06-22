//! Teammate autonomy levels (DEC-060) and the auto-approval spend
//! envelope each level grants.
//!
//! A teammate's autonomy is a level the workspace owner picks. Each
//! level maps to an [`AutoApprovalPolicy`] (the existing per-worker
//! spend gate): [`AutonomyLevel::AlwaysAsk`] disables auto-approval so
//! every scheduled fire is held, and the higher rungs widen the spend
//! envelope. The action-RISK ceiling is enforced separately by the
//! duty gate — only `low`-risk actions ever auto-settle (DEC-028 /
//! DEC-041) — so a higher level grants MORE BUDGET, never a riskier
//! action class, and the irreducible approval floor holds at every
//! rung including [`AutonomyLevel::Bypass`].
//!
//! Backing labels reuse the existing `conservative | balanced |
//! aggressive` enum (display-renamed per DEC-060) plus `always_ask`
//! for the new safe default, so no stored value migrates.

use crate::AutoApprovalPolicy;

/// One rung of the DEC-060 autonomy ladder, chosen per-teammate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutonomyLevel {
    /// Hold every scheduled fire for approval (default; no auto-spend).
    AlwaysAsk,
    /// Auto-settle low-risk actions within a tight envelope.
    AcceptSafe,
    /// Auto-settle low-risk actions within a wider envelope.
    Auto,
    /// Auto-settle low-risk actions within the widest envelope.
    /// Skips the approval QUEUE, never the irreducible floor.
    Bypass,
}

impl AutonomyLevel {
    /// Parse a stored or display label. Accepts the back-compat
    /// `conservative | balanced | aggressive` storage labels and the
    /// DEC-060 display labels; `none` / `always_ask` →
    /// [`AutonomyLevel::AlwaysAsk`]. Returns `None` for anything else.
    ///
    /// ```
    /// use argo_identity::autonomy_level::AutonomyLevel;
    /// assert_eq!(AutonomyLevel::from_label("conservative"), Some(AutonomyLevel::AcceptSafe));
    /// assert_eq!(AutonomyLevel::from_label("bypass"), Some(AutonomyLevel::Bypass));
    /// assert_eq!(AutonomyLevel::from_label("nonsense"), None);
    /// ```
    #[must_use]
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "always_ask" | "none" => Some(Self::AlwaysAsk),
            "conservative" | "accept_safe" => Some(Self::AcceptSafe),
            "balanced" | "auto" => Some(Self::Auto),
            "aggressive" | "bypass" => Some(Self::Bypass),
            _ => None,
        }
    }

    /// The canonical stored label for this level.
    ///
    /// ```
    /// use argo_identity::autonomy_level::AutonomyLevel;
    /// assert_eq!(AutonomyLevel::Bypass.label(), "aggressive");
    /// ```
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::AlwaysAsk => "always_ask",
            Self::AcceptSafe => "conservative",
            Self::Auto => "balanced",
            Self::Bypass => "aggressive",
        }
    }

    /// The auto-approval spend envelope this level grants.
    /// [`AutonomyLevel::AlwaysAsk`] disables auto-approval entirely
    /// (every fire held); the higher rungs widen the per-action +
    /// daily ceilings. Risk class is NOT set here — the duty gate caps
    /// auto-settle at `low` risk regardless of level (DEC-028).
    ///
    /// ```
    /// use argo_identity::autonomy_level::AutonomyLevel;
    /// assert!(!AutonomyLevel::AlwaysAsk.policy().enabled);
    /// assert!(AutonomyLevel::AcceptSafe.policy().enabled);
    /// ```
    #[must_use]
    pub fn policy(self) -> AutoApprovalPolicy {
        let (max, daily) = match self {
            Self::AlwaysAsk => {
                return AutoApprovalPolicy {
                    enabled: false,
                    ..AutoApprovalPolicy::default()
                };
            }
            Self::AcceptSafe => (200_000, 2_000_000), // $0.20 / action, $2 / day
            Self::Auto => (1_000_000, 10_000_000),    // $1 / action, $10 / day
            Self::Bypass => (5_000_000, 50_000_000),  // $5 / action, $50 / day
        };
        AutoApprovalPolicy {
            enabled: true,
            max_usd_micros_per_action: Some(max),
            daily_budget_usd_micros: Some(daily),
            ..AutoApprovalPolicy::default()
        }
    }
}

#[cfg(test)]
#[path = "autonomy_level_tests.rs"]
mod tests;
