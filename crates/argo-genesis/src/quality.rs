//! Quality gates for generated verticals.
//!
//! A `GenesisPlan` produced by [`crate::pipeline::plan`] is the
//! input to disk write. Without a quality bar, Genesis happily
//! emits a vertical with 1 action, a 50-word prompt, and a
//! single eval seed — technically valid, practically useless.
//!
//! This module exposes a structured check + a `require_pass`
//! converter so callers (CLI, tests) can either surface the
//! report to the operator OR refuse to materialise the vertical.
//!
//! v0 brief deviations called out so the next reader knows:
//! - The brief refers to a `GeneratedVertical` type — actual
//!   pipeline returns `GenesisPlan`. This module operates on
//!   the `GenesisPlan`.
//! - The brief calls for `ArgoError::Validation` — that variant
//!   doesn't exist; the closest fit is `ArgoError::Schema`,
//!   used here.
//! - `pipeline.rs` invokes `check_plan` after generation but
//!   does NOT call `require_pass` — current `eval_seed_rows`
//!   tops out at 2 seeds per archetype, so unconditional
//!   gating would break Genesis. The CLI follow-up brief wires
//!   the `--allow-low-quality` flag and inverts the default.

use crate::pipeline::GenesisPlan;
use crate::prompt_writer::{build_eval_seed, build_memory_schema, build_system_prompt};
use argo_types::{ArgoError, ArgoResult};

/// Minimum action vocabulary entries on `actions`. Anything
/// below this and the worker has no useful repertoire.
pub const ACTION_MIN: usize = 3;

/// Minimum eval seed rows. Sub-10 is "I tested two cases" —
/// not enough to trust regression catching.
pub const EVAL_SEED_MIN: usize = 10;

/// Minimum memory schema namespace count. ≥1 means the worker
/// has somewhere to write.
pub const MEMORY_LAYER_MIN: usize = 1;

/// Minimum whitespace-tokenized word count on the rendered
/// system prompt. 200 is the bar for "actually says something."
pub const PROMPT_WORD_MIN: usize = 200;

/// Outcome of a single check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    /// Stable identifier for the check (snake_case).
    pub name: String,
    /// `true` iff this check met its minimum.
    pub passed: bool,
    /// What we observed (e.g. `"2 actions"`).
    pub observed: String,
    /// The threshold that was required (e.g. `">= 3 actions"`).
    pub expected: String,
}

/// Aggregated outcome for one plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualityReport {
    /// `true` iff every individual check passed.
    pub passed: bool,
    /// Per-check breakdown in the order the checks ran.
    pub checks: Vec<CheckResult>,
}

impl QualityReport {
    /// Render only the failed checks, one per line, for use in
    /// error messages or operator-facing summaries.
    #[must_use]
    pub fn format_failures(&self) -> String {
        let mut out = String::new();
        for c in &self.checks {
            if c.passed {
                continue;
            }
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&format!(
                "- {}: observed {}, expected {}",
                c.name, c.observed, c.expected
            ));
        }
        out
    }
}

/// Run every quality check against `plan` and return a
/// structured report. Pure function — never errors.
#[must_use]
pub fn check_plan(plan: &GenesisPlan) -> QualityReport {
    let action_count = plan
        .manifest
        .actions
        .as_ref()
        .map_or(0, |a| a.actions.len());
    let eval_seed_count = build_eval_seed(plan.archetype, &plan.persona)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count();
    let memory_layer_count = count_memory_namespaces(&build_memory_schema());
    let prompt_word_count = build_system_prompt(&plan.charter, plan.archetype, &plan.persona)
        .split_whitespace()
        .count();
    let checks = vec![
        threshold_check(
            "action_vocabulary_min",
            action_count,
            ACTION_MIN,
            "actions",
        ),
        threshold_check("eval_seed_min", eval_seed_count, EVAL_SEED_MIN, "seeds"),
        threshold_check(
            "memory_layers_min",
            memory_layer_count,
            MEMORY_LAYER_MIN,
            "layers",
        ),
        threshold_check(
            "prompt_word_min",
            prompt_word_count,
            PROMPT_WORD_MIN,
            "words",
        ),
    ];
    let passed = checks.iter().all(|c| c.passed);
    QualityReport { passed, checks }
}

/// Convert a failing report into [`ArgoError::Schema`]; pass
/// means [`Ok`]. Used by callers that want a hard gate on
/// emission.
pub fn require_pass(report: &QualityReport) -> ArgoResult<()> {
    if report.passed {
        return Ok(());
    }
    Err(ArgoError::Schema(format!(
        "genesis quality gate failed:\n{}",
        report.format_failures()
    )))
}

/// Count `[namespaces.*]` rows in a memory schema TOML body.
fn count_memory_namespaces(schema_toml: &str) -> usize {
    schema_toml
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with("[namespaces.") && t.trim_end().ends_with(']')
        })
        .count()
}

fn threshold_check(name: &str, observed: usize, threshold: usize, unit: &str) -> CheckResult {
    CheckResult {
        name: name.to_string(),
        passed: observed >= threshold,
        observed: format!("{observed} {unit}"),
        expected: format!(">= {threshold} {unit}"),
    }
}

#[cfg(test)]
#[path = "quality_tests.rs"]
mod tests;
