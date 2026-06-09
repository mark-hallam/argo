//! LLM-rubric composition half of the hire gate (Genesis P7, DEC-025 #7).
//!
//! Extracted from `hire_gate.rs` to keep that file under the per-file LOC
//! ceiling. Owns the single additive method [`crate::GateVerdict::with_rubric`]
//! that folds the LLM-graded capability rubric ([`crate::rubric`]) into an
//! already-[`crate::GateVerdict::compose`]d verdict.
//!
//! The rubric is layered ON TOP of the deterministic five-signal gate (rather
//! than threaded as a sixth positional argument to `compose`) so that:
//! - the fail-closed baseline every hire seam already runs keeps a stable
//!   surface (no churn to the existing callers/tests), and
//! - the rubric can ONLY tighten the verdict, never loosen it — a teammate the
//!   deterministic gate rejected can never be rescued here.

use crate::genesis_judge::JudgeInput;
use crate::hire_gate::{GateState, GateVerdict};
use crate::pipeline::GenesisPlan;
use crate::rubric::{RubricOutcome, DIMENSION_FLOOR, RUBRIC_PASS_FLOOR};

/// Project a [`GenesisPlan`] onto the safety judge / rubric [`JudgeInput`].
/// Mirrors the projection `plan_or_broad` already uses so the safety judge AND
/// the LLM rubric feed an identical view of the archetype plan.
#[must_use]
pub fn judge_input_for_plan(plan: &GenesisPlan) -> JudgeInput<'_> {
    // Funnel through the shared projection so the safety judge sees each
    // action's `requires_approval` flag (parallel to its name): an
    // approval-gated send/pay/delete action is safe by construction and must
    // not be blocked, only an autonomous one is. Without the flags the judge
    // false-positive-blocked the onboarding first hire's approval-gated
    // `send_*`/`post_*` actions.
    JudgeInput::from_plan_projection(&plan.charter, &plan.persona, plan.manifest.actions.as_ref())
}

impl GateVerdict {
    /// Fold the LLM-graded capability rubric (Genesis P7, DEC-025 #7) into an
    /// already-composed verdict as an ADDITIVE sixth axis, re-deriving the gate
    /// state under the SAME fail-closed precedence (`Blocked` ▸ `NeedsReview` ▸
    /// `Pass`).
    ///
    /// - **Not applicable** (`outcome.applicable == false`, the flag-gated-off
    ///   default) → no change. The deterministic floor remains the gate.
    /// - **Graded + cleared** → no change (the teammate already passed the
    ///   richer bar; the other axes still decide).
    /// - **Graded + failed** (`applicable && !degraded && !passed_floor`) → a
    ///   hard **`Blocked`** with a per-dimension reason: a genuinely
    ///   thin/incoherent teammate the substring floor missed.
    /// - **Degraded** (`applicable && degraded`) → **`NeedsReview`** (held in
    ///   dry-run, audited). NEVER a silent pass — a rubric that could not run
    ///   must not wave a teammate through, and must not brick hiring either.
    ///
    /// Because a `Block` from any axis already wins, and the rubric only ever
    /// pushes toward `Blocked`/`NeedsReview`, the rubric is strictly a stricter
    /// quality lens, never a bypass of the deterministic gate.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn demo() -> Result<(), argo_genesis::GenesisError> {
    /// use argo_genesis::{
    ///     ComprehensionOutcome, ContractGateInput, EvalFloorOutcome, GateState, GateVerdict,
    ///     RoleContract, RubricOutcome, SafetySignal, SopComprehensionOutcome,
    /// };
    /// let plan = argo_genesis::plan("be my marketing intern, run weekly campaigns", "maya")?;
    /// let digest = RoleContract::derive_from_manifest(&plan.manifest).digest();
    /// // A teammate that would PASS the deterministic gate...
    /// let base = GateVerdict::compose(
    ///     EvalFloorOutcome::from_counts(10, 10),
    ///     ComprehensionOutcome::from_counts(5, 5),
    ///     SopComprehensionOutcome::not_applicable(),
    ///     SafetySignal::Allow,
    ///     ContractGateInput::from_plan(&plan, Some(&digest)),
    /// );
    /// assert_eq!(base.state, GateState::Pass);
    /// // ...is held for review if the rubric could not run (degrade fail-safe).
    /// let held = base.with_rubric(RubricOutcome::unavailable("provider down"));
    /// assert_eq!(held.state, GateState::NeedsReview);
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub fn with_rubric(mut self, outcome: RubricOutcome) -> Self {
        self.rubric_applicable = outcome.applicable;
        self.rubric_degraded = outcome.degraded;
        self.rubric_score = outcome.weighted_score;
        self.rubric_min_dimension = outcome.min_dimension;

        if !outcome.applicable {
            // Flag-gated off → vacuous pass, the deterministic gate is unchanged.
            self.rubric_verdict = "not_applicable".to_string();
            return self;
        }

        // Re-derive the two failure flags from the current state so the rubric
        // composes under the identical precedence as the five base axes.
        let mut blocked = self.state == GateState::Blocked;
        let mut needs_review = self.state == GateState::NeedsReview;

        if outcome.degraded {
            needs_review = true;
            self.rubric_verdict = "unavailable".to_string();
            self.reasons.push(format!(
                "LLM capability rubric unavailable (degraded → needs_review, held in \
                 dry-run): {} — the deterministic eval floor still applied",
                outcome.detail
            ));
        } else if !outcome.passed_floor {
            blocked = true;
            self.rubric_verdict = "block".to_string();
            self.reasons.push(format!(
                "LLM capability rubric failed: weighted score {:.2} (floor {:.2}), \
                 weakest dimension {:.2} (floor {:.2}) — the teammate is not coherently \
                 capable for its role{}",
                outcome.weighted_score,
                RUBRIC_PASS_FLOOR,
                outcome.min_dimension,
                DIMENSION_FLOOR,
                if outcome.detail.is_empty() {
                    String::new()
                } else {
                    format!(": {}", outcome.detail)
                },
            ));
        } else {
            self.rubric_verdict = "pass".to_string();
        }

        self.state = if blocked {
            GateState::Blocked
        } else if needs_review {
            GateState::NeedsReview
        } else {
            GateState::Pass
        };
        self
    }
}

#[cfg(test)]
#[path = "hire_gate_rubric_tests.rs"]
mod tests;
