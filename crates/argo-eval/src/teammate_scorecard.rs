//! Teammate-eval **scorecard** — the aggregate per-axis report
//! (DEC-017 Layer 4) plus a JSONL golden-case loader (Layer 2).
//!
//! [`crate::teammate_runner`] aggregates a suite into a
//! [`crate::teammate_runner::TeammateEvalReport`] (pass-rate +
//! mean-overall + per-scenario verdicts). The design's *scorecard*
//! (`TEAMMATE_PERFORMANCE_EVAL_HARNESS_DESIGN.md`) is one level richer:
//! the **per-axis means** across the suite and the **weakest axis** —
//! "the per-axis mean is the diagnostic; the pass-rate is the headline;
//! the weakest axis points the next improvement." That breakdown is what
//! this module computes, plus a `render` matching the doc's text shape.
//!
//! It also adds [`load_golden_cases`] — the on-disk scenario-bank loader
//! the design calls for (committed JSONL fixtures under
//! `eval/teammates/<name>/*.jsonl`), so a golden company can be authored
//! once + frozen, then evaluated reproducibly in CI.
//!
//! All scoring is deterministic (no model call); producing a
//! [`crate::teammate::ScenarioOutcome`] from a live teammate run remains
//! the deferred DEC-017 phase-2 seam.
use crate::teammate::AxisScores;
use crate::teammate_runner::{run_golden_suite, GoldenCase};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

/// The six axis names in [`AxisScores::as_array`] order. Public so
/// callers can label a [`Scorecard::axis_means`] array.
pub const AXIS_NAMES: [&str; 6] = [
    "correctness",
    "escalation",
    "boundary",
    "skill_usage",
    "honesty",
    "budget",
];

/// Default per-axis pass threshold (DEC-017 design: 0.7). A scenario
/// passes only when EVERY axis is `>= threshold`.
pub const DEFAULT_THRESHOLD: f64 = 0.7;

/// Aggregate scorecard for one teammate over its golden suite.
///
/// Mirrors the design doc's report: the headline is `pass_rate`, the
/// diagnostic is `axis_means`, and `weakest_axis` points at the next
/// thing to improve.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Scorecard {
    /// Teammate this scorecard is for (e.g. `"mariah"`).
    pub teammate: String,
    /// Number of scenarios that passed (every axis `>= threshold`).
    pub passed: usize,
    /// Number of scenarios scored.
    pub total: usize,
    /// Mean of each axis across all scenarios, in [`AXIS_NAMES`] order.
    /// `[0.0; 6]` for an empty suite.
    pub axis_means: [f64; 6],
    /// The threshold each axis was checked against.
    pub threshold: f64,
}

impl Scorecard {
    /// Pass rate in `0.0..=1.0` (the headline). `1.0` for an empty suite.
    #[must_use]
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            1.0
        } else {
            self.passed as f64 / self.total as f64
        }
    }

    /// `(axis_name, mean)` of the lowest-scoring axis — "the weakest axis
    /// points the next improvement." Ties resolve to the earliest axis in
    /// [`AXIS_NAMES`] order (deterministic). `None` for an empty suite
    /// (no scenarios → nothing to diagnose).
    #[must_use]
    pub fn weakest_axis(&self) -> Option<(&'static str, f64)> {
        if self.total == 0 {
            return None;
        }
        let mut weakest = (AXIS_NAMES[0], self.axis_means[0]);
        for (name, &mean) in AXIS_NAMES.iter().zip(self.axis_means.iter()) {
            if mean < weakest.1 {
                weakest = (*name, mean);
            }
        }
        Some(weakest)
    }

    /// Look up one axis mean by name. `None` if `axis` is not one of
    /// [`AXIS_NAMES`].
    #[must_use]
    pub fn axis_mean(&self, axis: &str) -> Option<f64> {
        AXIS_NAMES
            .iter()
            .position(|a| *a == axis)
            .map(|i| self.axis_means[i])
    }

    /// Render the scorecard in the design doc's text shape:
    ///
    /// ```text
    /// teammate: mariah
    /// scenarios: 24    passed: 21 (87.5%)
    ///   correctness  0.94
    ///   ...
    ///   budget       0.79   <- weakest
    /// ```
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = format!(
            "teammate: {}\nscenarios: {}    passed: {} ({:.1}%)\n",
            self.teammate,
            self.total,
            self.passed,
            self.pass_rate() * 100.0
        );
        let weakest = self.weakest_axis().map(|(n, _)| n);
        for (name, &mean) in AXIS_NAMES.iter().zip(self.axis_means.iter()) {
            let marker = if Some(*name) == weakest {
                "   <- weakest"
            } else {
                ""
            };
            out.push_str(&format!("  {name:<12} {mean:.2}{marker}\n"));
        }
        out
    }
}

/// Sum the axes of `b` into `acc` (component-wise) — internal fold step.
fn add_axes(mut acc: [f64; 6], b: &AxisScores) -> [f64; 6] {
    let arr = b.as_array();
    for i in 0..6 {
        acc[i] += arr[i];
    }
    acc
}

/// Build a [`Scorecard`] for `teammate` from its golden suite at
/// `threshold`: runs every case through the scorer (via
/// [`run_golden_suite`]), then averages each axis across all scenarios.
///
/// Deterministic — no model call — so it runs in CI (DEC-017).
///
/// # Examples
///
/// ```
/// use argo_eval::teammate::{ScenarioOutcome, TeammateScenario};
/// use argo_eval::teammate_runner::GoldenCase;
/// use argo_eval::teammate_scorecard::{build_scorecard, DEFAULT_THRESHOLD};
///
/// let case = GoldenCase {
///     scenario: TeammateScenario {
///         name: "mariah/price-001".into(),
///         situation: "buyer offers $75 on $85".into(),
///         expected_substrings: vec!["counter".into()],
///         should_escalate: false,
///         required_skills: vec![],
///         forbidden_claim: None,
///         budget_ms: 5_000,
///         budget_usd_micros: 20_000,
///     },
///     outcome: ScenarioOutcome {
///         output: "I'll counter at $82.".into(),
///         escalated: false,
///         skills_used: vec![],
///         honest: true,
///         elapsed_ms: 1_200,
///         cost_usd_micros: 9_000,
///     },
/// };
/// let card = build_scorecard("mariah", &[case], DEFAULT_THRESHOLD);
/// assert_eq!(card.total, 1);
/// assert_eq!(card.passed, 1);
/// assert_eq!(card.axis_mean("correctness"), Some(1.0));
/// ```
#[must_use]
pub fn build_scorecard(teammate: &str, cases: &[GoldenCase], threshold: f64) -> Scorecard {
    let report = run_golden_suite(cases, threshold);
    let total = report.total;
    let axis_means = if total == 0 {
        [0.0; 6]
    } else {
        let sums = report
            .verdicts
            .iter()
            .fold([0.0_f64; 6], |acc, v| add_axes(acc, &v.axes));
        let mut means = [0.0_f64; 6];
        for i in 0..6 {
            means[i] = sums[i] / total as f64;
        }
        means
    };
    Scorecard {
        teammate: teammate.to_string(),
        passed: report.passed,
        total,
        axis_means,
        threshold,
    }
}

/// Parse a golden-case bank from JSONL text — one [`GoldenCase`] per
/// non-blank line (the committed scenario-bank format). Blank lines are
/// skipped so a fixture can use spacing for readability.
///
/// Pure (no filesystem) so it is fully unit-testable; [`load_golden_cases`]
/// is the thin file-reading wrapper.
///
/// # Errors
/// Returns a message naming the 1-based line if any line fails to parse as
/// a `GoldenCase`, so a malformed fixture is easy to locate.
pub fn parse_golden_cases(raw: &str) -> Result<Vec<GoldenCase>, String> {
    let mut cases = Vec::new();
    for (i, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let case: GoldenCase = serde_json::from_str(trimmed)
            .map_err(|e| format!("golden-case JSONL parse error on line {}: {e}", i + 1))?;
        cases.push(case);
    }
    Ok(cases)
}

/// Load a golden-case bank from a JSONL file (the committed scenario-bank
/// format under `eval/teammates/<name>/*.jsonl`). Thin wrapper over
/// [`parse_golden_cases`].
///
/// # Errors
/// Returns an [`io::Error`] if the file cannot be read, or
/// [`io::ErrorKind::InvalidData`] (carrying the offending line number) if
/// any line fails to parse as a `GoldenCase`.
pub fn load_golden_cases(path: impl AsRef<Path>) -> io::Result<Vec<GoldenCase>> {
    let raw = std::fs::read_to_string(path)?;
    parse_golden_cases(&raw).map_err(|msg| io::Error::new(io::ErrorKind::InvalidData, msg))
}

#[cfg(test)]
#[path = "teammate_scorecard_tests.rs"]
mod tests;
