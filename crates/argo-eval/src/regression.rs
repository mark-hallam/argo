//! Cross-version eval regression detection.
//!
//! The detector compares two eval reports over their shared fixture
//! set. A candidate blocks promotion only when it has a meaningful
//! aggregate pass-rate drop with statistical support, or a large
//! mean-score drop. Per-fixture regressions are always reported for
//! operator review.

use crate::{EvalRunFixture, EvalRunReport};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Regression thresholds for comparing a candidate run to a baseline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EvalRegressionConfig {
    /// Minimum shared fixtures before producing a blocking decision.
    pub min_common_fixtures: usize,
    /// Minimum candidate pass-rate drop that can block promotion.
    pub min_pass_rate_drop: f64,
    /// Minimum mean score drop that can block promotion.
    pub min_mean_score_drop: f64,
    /// Two-proportion z-score threshold. 1.96 is about 95% confidence.
    pub min_z_score: f64,
}

impl Default for EvalRegressionConfig {
    fn default() -> Self {
        Self {
            min_common_fixtures: 20,
            min_pass_rate_drop: 0.05,
            min_mean_score_drop: 0.10,
            min_z_score: 1.96,
        }
    }
}

/// Blocking decision for a candidate eval run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalRegressionDecision {
    /// Too few shared fixtures to make a promotion decision.
    NoBaseline,
    /// Candidate is within configured noise thresholds.
    Stable,
    /// Candidate regressed enough to block promotion.
    Regressed,
}

/// One fixture whose candidate result is worse than the baseline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalFixtureRegression {
    /// Stable fixture id.
    pub fixture: String,
    /// Skill or workflow under test.
    pub skill: String,
    /// Metric name.
    pub metric: String,
    /// Baseline pass flag.
    pub passed_baseline: bool,
    /// Candidate pass flag.
    pub passed_candidate: bool,
    /// Baseline score.
    pub score_baseline: f64,
    /// Candidate score.
    pub score_candidate: f64,
    /// `score_candidate - score_baseline`.
    pub score_delta: f64,
}

/// Aggregate regression report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalRegressionReport {
    /// Baseline eval run id.
    pub baseline_run: String,
    /// Candidate eval run id.
    pub candidate_run: String,
    /// Shared fixture count used for the decision.
    pub common_fixtures: usize,
    /// Baseline pass rate over shared fixtures.
    pub baseline_pass_rate: f64,
    /// Candidate pass rate over shared fixtures.
    pub candidate_pass_rate: f64,
    /// `candidate_pass_rate - baseline_pass_rate`.
    pub pass_rate_delta: f64,
    /// Mean score delta over shared fixtures.
    pub mean_score_delta: f64,
    /// Two-proportion z-score for the pass-rate drop.
    pub z_score: f64,
    /// Blocking decision.
    pub decision: EvalRegressionDecision,
    /// Per-fixture regressions for diagnostics.
    pub fixture_regressions: Vec<EvalFixtureRegression>,
}

impl EvalRegressionReport {
    /// Returns true when the report should block promotion.
    #[must_use]
    pub fn blocks_promotion(&self) -> bool {
        self.decision == EvalRegressionDecision::Regressed
    }
}

/// Compare a candidate eval run against a baseline.
#[must_use]
pub fn detect_eval_regression(
    baseline: &EvalRunReport,
    candidate: &EvalRunReport,
    cfg: EvalRegressionConfig,
) -> EvalRegressionReport {
    let baseline_by_fixture = fixture_map(&baseline.fixtures);
    let candidate_by_fixture = fixture_map(&candidate.fixtures);
    let mut pairs = Vec::new();
    for (fixture, left) in baseline_by_fixture {
        if let Some(right) = candidate_by_fixture.get(fixture) {
            pairs.push((fixture, left, *right));
        }
    }
    let common = pairs.len();
    let baseline_passes = pairs.iter().filter(|(_, b, _)| b.passed).count();
    let candidate_passes = pairs.iter().filter(|(_, _, c)| c.passed).count();
    let baseline_rate = rate(baseline_passes, common);
    let candidate_rate = rate(candidate_passes, common);
    let pass_rate_delta = candidate_rate - baseline_rate;
    let mean_score_delta = mean(pairs.iter().map(|(_, b, c)| c.score - b.score));
    let z_score = pass_rate_z_score(baseline_passes, candidate_passes, common);
    let fixture_regressions = pairs
        .iter()
        .filter(|(_, b, c)| (b.passed && !c.passed) || c.score < b.score)
        .map(|(fixture, b, c)| EvalFixtureRegression {
            fixture: (*fixture).to_string(),
            skill: c.skill.clone(),
            metric: c.metric.clone(),
            passed_baseline: b.passed,
            passed_candidate: c.passed,
            score_baseline: b.score,
            score_candidate: c.score,
            score_delta: c.score - b.score,
        })
        .collect();
    let decision = if common < cfg.min_common_fixtures {
        EvalRegressionDecision::NoBaseline
    } else if (-pass_rate_delta >= cfg.min_pass_rate_drop && z_score >= cfg.min_z_score)
        || -mean_score_delta >= cfg.min_mean_score_drop
    {
        EvalRegressionDecision::Regressed
    } else {
        EvalRegressionDecision::Stable
    };
    EvalRegressionReport {
        baseline_run: baseline.id.clone(),
        candidate_run: candidate.id.clone(),
        common_fixtures: common,
        baseline_pass_rate: baseline_rate,
        candidate_pass_rate: candidate_rate,
        pass_rate_delta,
        mean_score_delta,
        z_score,
        decision,
        fixture_regressions,
    }
}

/// Enforce a regression report as an AB-promotion gate.
///
/// # Errors
/// Returns an error string when `report` blocks promotion and
/// `override_regression` is false.
pub fn ensure_promotion_allowed(
    report: &EvalRegressionReport,
    override_regression: bool,
) -> Result<(), String> {
    if report.blocks_promotion() && !override_regression {
        return Err(format!(
            "eval_regression: candidate {} regressed against {} ({:.1}% pass-rate delta, {:.3} mean-score delta, z={:.2}); pass override to promote anyway",
            report.candidate_run,
            report.baseline_run,
            report.pass_rate_delta * 100.0,
            report.mean_score_delta,
            report.z_score
        ));
    }
    Ok(())
}

fn fixture_map(fixtures: &[EvalRunFixture]) -> BTreeMap<&str, &EvalRunFixture> {
    fixtures.iter().map(|f| (f.fixture.as_str(), f)).collect()
}

fn rate(passes: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        passes as f64 / total as f64
    }
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let mut sum = 0.0;
    let mut count = 0usize;
    for value in values {
        sum += value;
        count += 1;
    }
    if count == 0 {
        0.0
    } else {
        sum / count as f64
    }
}

fn pass_rate_z_score(baseline_passes: usize, candidate_passes: usize, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }
    let p1 = baseline_passes as f64 / total as f64;
    let p2 = candidate_passes as f64 / total as f64;
    let pooled = (baseline_passes + candidate_passes) as f64 / (2 * total) as f64;
    let variance = pooled * (1.0 - pooled) * (2.0 / total as f64);
    if variance <= f64::EPSILON {
        0.0
    } else {
        ((p1 - p2) / variance.sqrt()).max(0.0)
    }
}

#[cfg(test)]
#[path = "regression_tests.rs"]
mod tests;
