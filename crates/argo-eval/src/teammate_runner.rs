//! Teammate-eval **runner** — drives the [`crate::teammate`] 6-axis
//! scorer over a golden-company suite and aggregates a pass/fail report
//! (DEC-017).
//!
//! `teammate.rs` is the scorer substrate: [`score_scenario`] turns one
//! `(scenario, outcome)` pair into six [`AxisScores`]. DEC-017 calls for
//! the next piece — *"a runner [that] drives it over the golden set …
//! deterministic … runs in CI"* — which is what this module adds. Until
//! now the scorer had **no consumer** (FEATURE_MATRIX "Teammate-eval" =
//! 🟧 Scaffold); this is that consumer, as a library so CI can call it
//! without a model.
//!
//! ## What this is / is not
//!
//! - **Is:** a deterministic (no model call) evaluator that turns
//!   `(scenario, outcome)` pairs into per-scenario pass/fail verdicts +
//!   an aggregate report, runnable in CI against committed golden cases.
//! - **Is not:** a live teammate driver. Producing a [`ScenarioOutcome`]
//!   from a real teammate run (or historical replay) is the separate,
//!   later seam DEC-017 defers (synthetic golden companies first).
use crate::teammate::{score_scenario, AxisScores, ScenarioOutcome, TeammateScenario};
use serde::{Deserialize, Serialize};

/// One golden-suite case: a scenario's behavioural rubric paired with the
/// teammate's observed outcome for that scenario.
///
/// Serializable so a golden set can be authored on disk (JSONL) and
/// loaded into CI without this crate taking a filesystem dependency.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GoldenCase {
    /// The expected-behaviour rubric (situation + per-axis expectations).
    pub scenario: TeammateScenario,
    /// What the teammate actually did on this scenario.
    pub outcome: ScenarioOutcome,
}

/// Verdict for one scenario: the per-axis scores plus whether every axis
/// met `threshold`, with the names of any axes that fell short.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScenarioVerdict {
    /// The scenario this verdict is for ([`TeammateScenario::name`]).
    pub scenario_name: String,
    /// The six axis scores the runner computed.
    pub axes: AxisScores,
    /// Unweighted mean across the six axes ([`AxisScores::overall`]).
    pub overall: f64,
    /// `true` iff every axis is `>= threshold`.
    pub passed: bool,
    /// Names of axes whose score fell below `threshold`, sorted for
    /// deterministic output.
    pub failed_axes: Vec<String>,
}

/// The six axis names in [`AxisScores::as_array`] order. Used to label
/// which axes fell below threshold in a [`ScenarioVerdict`].
const AXIS_NAMES: [&str; 6] = [
    "correctness",
    "escalation",
    "boundary",
    "skill_usage",
    "honesty",
    "budget",
];

/// Evaluate one golden case against `threshold`: score the outcome, then
/// flag every axis below the threshold. An axis passes when its score is
/// **>=** `threshold` (inclusive, matching [`AxisScores::passed`]). The
/// scenario passes iff every axis passes.
///
/// # Examples
///
/// ```
/// use argo_eval::teammate::{ScenarioOutcome, TeammateScenario};
/// use argo_eval::teammate_runner::{evaluate_case, GoldenCase};
///
/// let scenario = TeammateScenario {
///     name: "marketplace/price-drop-001".into(),
///     situation: "A buyer offers $75 on an $85 listing.".into(),
///     expected_substrings: vec!["counter".into()],
///     should_escalate: false,
///     required_skills: vec![],
///     forbidden_claim: None,
///     budget_ms: 5_000,
///     budget_usd_micros: 20_000,
/// };
/// let outcome = ScenarioOutcome {
///     output: "I'll counter at $82.".into(),
///     escalated: false,
///     skills_used: vec![],
///     honest: true,
///     elapsed_ms: 1_200,
///     cost_usd_micros: 9_000,
/// };
/// let verdict = evaluate_case(&GoldenCase { scenario, outcome }, 0.5);
/// assert!(verdict.passed);
/// assert!(verdict.failed_axes.is_empty());
/// ```
#[must_use]
pub fn evaluate_case(case: &GoldenCase, threshold: f64) -> ScenarioVerdict {
    let axes = score_scenario(&case.scenario, &case.outcome);
    let scores = axes.as_array();
    let mut failed_axes: Vec<String> = AXIS_NAMES
        .iter()
        .zip(scores.iter())
        .filter(|(_, &score)| score < threshold)
        .map(|(name, _)| (*name).to_string())
        .collect();
    failed_axes.sort();
    ScenarioVerdict {
        scenario_name: case.scenario.name.clone(),
        overall: axes.overall(),
        passed: failed_axes.is_empty(),
        failed_axes,
        axes,
    }
}

/// Aggregate report over a whole golden suite.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TeammateEvalReport {
    /// Per-scenario verdicts, in input order.
    pub verdicts: Vec<ScenarioVerdict>,
    /// How many scenarios passed.
    pub passed: usize,
    /// How many scenarios ran.
    pub total: usize,
    /// Mean of the per-scenario `overall` scores (0.0 for an empty suite).
    pub mean_overall: f64,
}

impl TeammateEvalReport {
    /// `true` iff every scenario passed. Vacuously `true` for an empty
    /// suite (no failures to report).
    #[must_use]
    pub fn all_passed(&self) -> bool {
        self.passed == self.total
    }

    /// Pass rate in `0.0..=1.0`. `1.0` for an empty suite.
    #[must_use]
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            1.0
        } else {
            self.passed as f64 / self.total as f64
        }
    }
}

/// Drive the 6-axis scorer over a golden suite at `threshold`, producing
/// per-scenario verdicts + an aggregate report. Deterministic — no model
/// call — so it runs in CI (DEC-017).
///
/// # Examples
///
/// ```
/// use argo_eval::teammate::{ScenarioOutcome, TeammateScenario};
/// use argo_eval::teammate_runner::{run_golden_suite, GoldenCase};
///
/// fn case(name: &str, has_counter: bool) -> GoldenCase {
///     GoldenCase {
///         scenario: TeammateScenario {
///             name: name.into(),
///             situation: "offer".into(),
///             expected_substrings: vec!["counter".into()],
///             should_escalate: false,
///             required_skills: vec![],
///             forbidden_claim: None,
///             budget_ms: 5_000,
///             budget_usd_micros: 20_000,
///         },
///         outcome: ScenarioOutcome {
///             output: if has_counter { "counter at $82".into() } else { "ok deal".into() },
///             escalated: false,
///             skills_used: vec![],
///             honest: true,
///             elapsed_ms: 100,
///             cost_usd_micros: 100,
///         },
///     }
/// }
///
/// let report = run_golden_suite(&[case("a", true), case("b", false)], 0.5);
/// assert_eq!(report.total, 2);
/// assert_eq!(report.passed, 1);
/// assert!(!report.all_passed());
/// ```
#[must_use]
pub fn run_golden_suite(cases: &[GoldenCase], threshold: f64) -> TeammateEvalReport {
    let verdicts: Vec<ScenarioVerdict> = cases
        .iter()
        .map(|c| evaluate_case(c, threshold))
        .collect();
    let passed = verdicts.iter().filter(|v| v.passed).count();
    let total = verdicts.len();
    let mean_overall = if total == 0 {
        0.0
    } else {
        verdicts.iter().map(|v| v.overall).sum::<f64>() / total as f64
    };
    TeammateEvalReport {
        verdicts,
        passed,
        total,
        mean_overall,
    }
}

#[cfg(test)]
#[path = "teammate_runner_tests.rs"]
mod tests;
