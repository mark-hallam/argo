//! Stream `claude-skill-success-metric` — structured score for
//! the skill-learning proposal gate.
//!
//! H-4c shipped a binary metric (`1.0` for completion, `0.0`
//! for handoff). That tells you "did the trajectory finish?"
//! but loses signal about *how* it finished — a one-step quick
//! win is much more proposal-worthy than a 30-step grind that
//! also succeeded.
//!
//! This module replaces the binary with a per-dimension
//! [`SuccessMetric`] + a [`ProposalThreshold`] so callers can
//! gate on confidence + length + duration + final-state score
//! independently. Defaults are intentionally conservative so
//! operators don't drown in low-value proposals.
//!
//! The previous `should_propose(f64, f64)` shim in
//! [`crate::learn`] is preserved as a deprecated thin wrapper
//! that builds a [`SuccessMetric`] with synthetic values, so
//! existing call sites compile unchanged for one release.

use serde::{Deserialize, Serialize};

/// Structured success score for a learned skill proposal.
/// Built by the executor at end-of-run from the actual
/// trajectory + timing + (optional) per-vertical fixture
/// score. Higher = more proposal-worthy along that dimension.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SuccessMetric {
    /// 0.0..=1.0 overall confidence the trajectory succeeded.
    /// Today's heuristic: `1.0` when the CU loop emitted
    /// `stop_success`, `0.0` otherwise. Future iterations
    /// will read it from a model-confidence field on the CU
    /// outcome.
    pub confidence: f64,
    /// Number of executor steps. Lower = simpler + more
    /// reusable path.
    pub trajectory_length: u32,
    /// Walltime in seconds.
    pub duration_seconds: f64,
    /// Optional final-state score (e.g. URL match against the
    /// vertical's declared success fixture). `None` when the
    /// vertical doesn't define one.
    #[serde(default)]
    pub final_state_score: Option<f64>,
}

impl SuccessMetric {
    /// Builder for the binary case the deprecated `f64` API
    /// passes through. Synthesises a length of 1 + duration of
    /// 0.0 so the legacy callers always clear the
    /// trajectory_length + duration floors.
    #[must_use]
    pub fn from_legacy_confidence(confidence: f64) -> Self {
        Self {
            confidence,
            trajectory_length: 1,
            duration_seconds: 0.0,
            final_state_score: None,
        }
    }

    /// Construct a metric directly. Used by the CU executor at
    /// end-of-run when it has all four signals.
    #[must_use]
    pub fn new(
        confidence: f64,
        trajectory_length: u32,
        duration_seconds: f64,
        final_state_score: Option<f64>,
    ) -> Self {
        Self {
            confidence,
            trajectory_length,
            duration_seconds,
            final_state_score,
        }
    }
}

/// Per-dimension floors the proposal gate enforces. A metric
/// must clear EVERY floor to propose. `final_state_score` is
/// optional — when present on the metric, it must clear
/// `min_final_state_score` (when both are set); otherwise it's
/// ignored.
///
/// Defaults match the brief: confidence ≥ 0.8, trajectory ≤ 25
/// steps, duration ≤ 600s.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProposalThreshold {
    /// Minimum confidence score (0.0..=1.0) to propose.
    pub min_confidence: f64,
    /// Maximum trajectory length to propose (longer = more
    /// noise, less reusable).
    pub max_trajectory_length: u32,
    /// Maximum walltime in seconds. Slow runs are filtered.
    pub max_duration_seconds: f64,
    /// Floor applied only when the metric carries a
    /// `final_state_score`; `None` skips the check.
    #[serde(default)]
    pub min_final_state_score: Option<f64>,
}

impl Default for ProposalThreshold {
    fn default() -> Self {
        Self {
            min_confidence: 0.8,
            max_trajectory_length: 25,
            max_duration_seconds: 600.0,
            min_final_state_score: None,
        }
    }
}

impl ProposalThreshold {
    /// Static default — `Default::default()` exposed as a
    /// constant builder for callers that want the brief's
    /// defaults inline.
    #[must_use]
    pub fn defaults() -> Self {
        Self::default()
    }

    /// Returns the dimension(s) the metric failed, if any. An
    /// empty Vec means the metric proposes; a non-empty Vec
    /// gives reviewers a precise reason it didn't.
    #[must_use]
    pub fn failures(&self, metric: &SuccessMetric) -> Vec<&'static str> {
        let mut out = Vec::new();
        if metric.confidence < self.min_confidence {
            out.push("confidence");
        }
        if metric.trajectory_length > self.max_trajectory_length {
            out.push("trajectory_length");
        }
        if metric.duration_seconds > self.max_duration_seconds {
            out.push("duration_seconds");
        }
        if let (Some(floor), Some(score)) =
            (self.min_final_state_score, metric.final_state_score)
        {
            if score < floor {
                out.push("final_state_score");
            }
        }
        out
    }

    /// True when the metric clears every dimension's floor.
    /// Pure function; used by the runtime's proposal gate.
    #[must_use]
    pub fn meets(&self, metric: &SuccessMetric) -> bool {
        self.failures(metric).is_empty()
    }
}

/// Structured proposal check — replaces the binary
/// `should_propose(f64, f64)`. Returns `true` when every floor
/// in `threshold` clears.
///
/// ```
/// use argo_skill::success_metric::{should_propose_metric, ProposalThreshold, SuccessMetric};
/// let metric = SuccessMetric::new(0.95, 5, 12.0, None);
/// assert!(should_propose_metric(&metric, &ProposalThreshold::defaults()));
/// ```
#[must_use]
pub fn should_propose_metric(metric: &SuccessMetric, threshold: &ProposalThreshold) -> bool {
    threshold.meets(metric)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good() -> SuccessMetric {
        SuccessMetric::new(0.95, 5, 30.0, Some(0.9))
    }

    #[test]
    fn defaults_match_brief() {
        let t = ProposalThreshold::defaults();
        assert!((t.min_confidence - 0.8).abs() < f64::EPSILON);
        assert_eq!(t.max_trajectory_length, 25);
        assert!((t.max_duration_seconds - 600.0).abs() < f64::EPSILON);
        assert!(t.min_final_state_score.is_none());
    }

    #[test]
    fn good_metric_clears_default_threshold() {
        assert!(should_propose_metric(&good(), &ProposalThreshold::defaults()));
    }

    #[test]
    fn confidence_floor_blocks_low_confidence() {
        let m = SuccessMetric::new(0.5, 5, 30.0, None);
        let failures = ProposalThreshold::defaults().failures(&m);
        assert_eq!(failures, vec!["confidence"]);
    }

    #[test]
    fn trajectory_length_floor_blocks_long_path() {
        let m = SuccessMetric::new(0.95, 50, 30.0, None);
        let failures = ProposalThreshold::defaults().failures(&m);
        assert_eq!(failures, vec!["trajectory_length"]);
    }

    #[test]
    fn duration_floor_blocks_slow_runs() {
        let m = SuccessMetric::new(0.95, 5, 700.0, None);
        let failures = ProposalThreshold::defaults().failures(&m);
        assert_eq!(failures, vec!["duration_seconds"]);
    }

    #[test]
    fn multiple_failures_are_reported_in_order() {
        let m = SuccessMetric::new(0.5, 100, 700.0, None);
        let failures = ProposalThreshold::defaults().failures(&m);
        assert_eq!(
            failures,
            vec!["confidence", "trajectory_length", "duration_seconds"]
        );
    }

    #[test]
    fn final_state_score_floor_only_checked_when_set() {
        let m_ok = SuccessMetric::new(0.95, 5, 30.0, Some(0.85));
        let m_low = SuccessMetric::new(0.95, 5, 30.0, Some(0.10));
        let m_none = SuccessMetric::new(0.95, 5, 30.0, None);
        let mut t = ProposalThreshold::defaults();
        // Default threshold ignores final_state_score.
        assert!(t.meets(&m_low));
        // Set a floor: m_ok clears, m_low fails, m_none ignored.
        t.min_final_state_score = Some(0.5);
        assert!(t.meets(&m_ok));
        assert!(!t.meets(&m_low));
        assert!(t.meets(&m_none));
    }

    #[test]
    fn from_legacy_confidence_synthesises_clearing_metric() {
        let m = SuccessMetric::from_legacy_confidence(0.9);
        assert!((m.confidence - 0.9).abs() < f64::EPSILON);
        assert_eq!(m.trajectory_length, 1);
        assert!((m.duration_seconds - 0.0).abs() < f64::EPSILON);
        assert!(m.final_state_score.is_none());
        // Clears the default threshold.
        assert!(should_propose_metric(&m, &ProposalThreshold::defaults()));
    }

    #[test]
    fn boundary_at_min_confidence_passes() {
        // Floor uses `<` so equal-to-floor passes.
        let m = SuccessMetric::new(0.8, 5, 30.0, None);
        assert!(should_propose_metric(&m, &ProposalThreshold::defaults()));
    }

    #[test]
    fn boundary_at_max_trajectory_passes() {
        let m = SuccessMetric::new(0.95, 25, 30.0, None);
        assert!(should_propose_metric(&m, &ProposalThreshold::defaults()));
    }

    #[test]
    fn serde_round_trips_metric_and_threshold() {
        let m = good();
        let json = serde_json::to_string(&m).unwrap();
        let parsed: SuccessMetric = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, m);
        let t = ProposalThreshold::defaults();
        let json = serde_json::to_string(&t).unwrap();
        let parsed: ProposalThreshold = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, t);
    }
}
