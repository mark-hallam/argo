//! Stream `claude-skill-success-metric` — heuristic computation
//! of the structured `argo_skill::SuccessMetric` from a CU
//! [`Outcome`].
//!
//! The pre-existing binary metric in [`crate::learn`] stays
//! around for the deprecated shim path. This module is the
//! richer compute the brief asks for: it inspects the
//! trajectory length + the run's wallclock + (optionally) a
//! per-vertical fixture score and packs them into the four-field
//! struct that the proposal gate then evaluates against per-
//! dimension floors.

use crate::executor::Outcome;
use argo_skill::SuccessMetric;

/// Build a structured [`SuccessMetric`] from a CU outcome.
///
/// - `confidence`: `1.0` when the outcome is `Completed`, `0.0`
///   when it's `HumanHandoff`. (Future iteration: read the
///   model's confidence field once the CU loop emits one.)
/// - `trajectory_length`: number of CU steps actually taken.
/// - `duration_seconds`: caller supplies (recorded by the
///   executor at start + end of the run).
/// - `final_state_score`: caller-supplied; `None` when the
///   vertical doesn't define a fixture.
#[must_use]
pub fn metric_from_outcome(
    outcome: &Outcome,
    duration_seconds: f64,
    final_state_score: Option<f64>,
) -> SuccessMetric {
    let (confidence, trajectory_length) = match outcome {
        Outcome::Completed { result, .. } => (1.0_f64, result.actions_taken.len() as u32),
        Outcome::HumanHandoff { .. } => (0.0_f64, 0_u32),
    };
    SuccessMetric::new(
        confidence,
        trajectory_length,
        duration_seconds,
        final_state_score,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::BudgetSnapshot;
    use crate::executor::RunResult;
    use std::path::PathBuf;

    fn budget_zero() -> BudgetSnapshot {
        BudgetSnapshot {
            tokens_used: 0,
            tokens_cap: 100,
            steps_used: 0,
            steps_cap: 5,
            wallclock_seconds_used: 0,
            wallclock_seconds_cap: 60,
        }
    }

    #[test]
    fn completed_outcome_yields_confidence_one_and_real_trajectory_length() {
        let outcome = Outcome::Completed {
            result: RunResult {
                final_text: None,
                actions_taken: vec!["a".into(), "b".into(), "c".into()],
                tokens_used: 100,
            },
            bundle_dir: PathBuf::from("/tmp/x"),
            budget: budget_zero(),
        };
        let m = metric_from_outcome(&outcome, 12.5, None);
        assert!((m.confidence - 1.0).abs() < f64::EPSILON);
        assert_eq!(m.trajectory_length, 3);
        assert!((m.duration_seconds - 12.5).abs() < f64::EPSILON);
        assert!(m.final_state_score.is_none());
    }

    #[test]
    fn handoff_outcome_yields_confidence_zero_and_zero_length() {
        let outcome = Outcome::HumanHandoff {
            reason: "test".into(),
            bundle_dir: PathBuf::from("/tmp/x"),
            budget: budget_zero(),
        };
        let m = metric_from_outcome(&outcome, 8.0, None);
        assert!((m.confidence - 0.0).abs() < f64::EPSILON);
        assert_eq!(m.trajectory_length, 0);
    }

    #[test]
    fn final_state_score_passes_through_when_supplied() {
        let outcome = Outcome::Completed {
            result: RunResult {
                final_text: None,
                actions_taken: vec![],
                tokens_used: 0,
            },
            bundle_dir: PathBuf::from("/tmp/x"),
            budget: budget_zero(),
        };
        let m = metric_from_outcome(&outcome, 0.0, Some(0.85));
        assert_eq!(m.final_state_score, Some(0.85));
    }
}
