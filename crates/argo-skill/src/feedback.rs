//! Operator-feedback scoring helpers (Stream
//! `claude-operator-feedback-channel`). Sits between the
//! `success_metric` thresholding and the CU executor's
//! `maybe_propose` call: one extra signal that the operator has
//! ground-truthed the action.
//!
//! Concrete behaviour (mirrors the brief):
//!   * `Vote::Up`   → propose unconditionally (operator says yes)
//!   * `Vote::Down` → blacklist (operator says no, never propose)
//!   * `None`       → fall back to the metric-based threshold check
//!
//! Storage of the raw feedback rows lives in
//! `argo-store::feedback` so this crate stays DB-free.

use crate::success_metric::{should_propose_metric, ProposalThreshold, SuccessMetric};
use serde::{Deserialize, Serialize};

/// Operator vote on a single action. Mirrors the storage-layer
/// `Vote` enum but kept independent so `argo-skill` doesn't
/// depend on `argo-store`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackVote {
    /// Operator approved this action.
    Up,
    /// Operator rejected this action.
    Down,
}

/// Should the skill-learning loop emit a candidate for this
/// action? Combines the metric-based threshold with the
/// operator's vote, with the vote winning when present.
#[must_use]
pub fn should_propose_with_feedback(
    metric: &SuccessMetric,
    threshold: &ProposalThreshold,
    feedback: Option<FeedbackVote>,
) -> bool {
    match feedback {
        Some(FeedbackVote::Down) => false,
        Some(FeedbackVote::Up) => true,
        None => should_propose_metric(metric, threshold),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good() -> SuccessMetric {
        SuccessMetric::new(0.9, 10, 30.0, None)
    }

    fn marginal() -> SuccessMetric {
        // Below the default 0.8 confidence floor.
        SuccessMetric::new(0.5, 30, 800.0, None)
    }

    #[test]
    fn vote_up_overrides_marginal_metric() {
        // Marginal metric would normally fail the threshold...
        assert!(!should_propose_metric(
            &marginal(),
            &ProposalThreshold::defaults()
        ));
        // ...but an operator up-vote forces a propose.
        assert!(should_propose_with_feedback(
            &marginal(),
            &ProposalThreshold::defaults(),
            Some(FeedbackVote::Up),
        ));
    }

    #[test]
    fn vote_down_overrides_passing_metric() {
        // Good metric would normally pass...
        assert!(should_propose_metric(&good(), &ProposalThreshold::defaults()));
        // ...but an operator down-vote forces a blacklist.
        assert!(!should_propose_with_feedback(
            &good(),
            &ProposalThreshold::defaults(),
            Some(FeedbackVote::Down),
        ));
    }

    #[test]
    fn no_feedback_falls_back_to_metric_check() {
        let t = ProposalThreshold::defaults();
        assert!(should_propose_with_feedback(&good(), &t, None));
        assert!(!should_propose_with_feedback(&marginal(), &t, None));
    }

    #[test]
    fn feedback_vote_serde_round_trip() {
        for vote in [FeedbackVote::Up, FeedbackVote::Down] {
            let json = serde_json::to_string(&vote).unwrap();
            let parsed: FeedbackVote = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, vote);
        }
    }
}
