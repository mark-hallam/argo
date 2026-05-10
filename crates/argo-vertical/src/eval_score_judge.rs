//! Optional judge layer for scored evals (#698).
//!
//! Two-mode design:
//!
//! - **`Heuristic`** — pure, deterministic, no LLM dependency.
//!   Takes the existing per-rule outcomes from the rule engine
//!   (#693) and aggregates them into a single
//!   `JudgeVerdict { passed, reason }`. Useful as a CI gate
//!   that wants ONE pass/fail per fixture instead of N rule
//!   checks. Also catches "rules disagree" cases (e.g. some
//!   pass, some fail with low-confidence markers).
//!
//! - **`Live { provider, model }`** — placeholder for the
//!   real LLM judge (PR #698b). Today, the verdict surfaces a
//!   `not_yet_wired` reason explaining what's needed (transport
//!   setup + provider API key) so operators don't see silent
//!   failures. The seam is in place; the live wiring is a
//!   bounded follow-up.
//!
//! No LLM dependency in argo-vertical. The CLI handler in
//! `vertical_eval_score` is the only place that would actually
//! invoke a transport once #698b lands — keeps argo-vertical
//! pure (no network deps).
//!
//! ## File layout
//!
//! - This file: types (`JudgeVerdict`, `JudgePolicy`),
//!   `parse_judge_spec`, the dispatcher `judge_fixture`, and
//!   the heuristic aggregator.
//! - Sibling `eval_score_judge_live.rs`: pure helpers for the
//!   live LLM judge (`build_judge_prompt`, `parse_judge_response`).
//!   Re-exported below via `pub(crate) use` so callers continue
//!   to see one judge-layer surface.
use serde::Serialize;

use crate::eval_score::FixtureScore;

pub use crate::eval_score_judge_live::{build_judge_prompt, parse_judge_response};

/// One judge outcome per scored fixture. `passed` is the
/// machine-readable pass/fail bit; `reason` is the operator-
/// facing explanation; `source` records which judge produced
/// the verdict so audit + debugging can trace it.
///
/// `score` and `failed_criteria` (#701) extend the verdict for
/// the live LLM judge — graded scores and per-criterion
/// detail. Heuristic verdicts populate `score` as 0.0 / 1.0
/// (mirrors `passed`) and leave `failed_criteria` empty.
#[derive(Debug, Clone, Serialize)]
pub struct JudgeVerdict {
    pub passed: bool,
    pub reason: String,
    /// `"heuristic"` for the rule-aggregating judge,
    /// `"live:<provider>/<model>"` for the live LLM judge,
    /// `"not_wired:<provider>/<model>"` for the placeholder.
    pub source: String,
    /// Graded score in [0.0, 1.0]. Heuristic + not_wired
    /// verdicts use `1.0` for pass and `0.0` for fail. Live
    /// judges may use intermediate values; "uncertain"
    /// verdicts typically land in [0.4, 0.6].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    /// Names of criteria that failed (e.g.
    /// `"must_include[role-based agents]"`,
    /// `"personalization_quality"`). Empty when the verdict
    /// passed; populated by the live judge to localise
    /// failures so operators don't have to re-read the prose
    /// reason.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed_criteria: Vec<String>,
}

/// Parse a `--judge <spec>` value into the matching policy.
/// Returns `Err` for an empty / malformed spec so the CLI
/// surfaces the problem before iterating fixtures.
///
/// Accepted shapes (case-sensitive):
/// - `"heuristic"` → [`JudgePolicy::Heuristic`]
/// - `"<provider>/<model>"` → [`JudgePolicy::Live`]
///   (provider + model both non-empty, no extra slashes)
///
/// # Errors
/// Empty input, missing slash, empty provider, empty model,
/// or more than one slash all produce a clear message.
pub fn parse_judge_spec(spec: &str) -> Result<JudgePolicy, String> {
    let s = spec.trim();
    if s.is_empty() {
        return Err("--judge value is empty".into());
    }
    if s == "heuristic" {
        return Ok(JudgePolicy::Heuristic);
    }
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() != 2 {
        return Err(format!(
            "--judge value `{s}` must be `heuristic` or `<provider>/<model>` (got {} segment(s))",
            parts.len()
        ));
    }
    let provider = parts[0].trim();
    let model = parts[1].trim();
    if provider.is_empty() || model.is_empty() {
        return Err(format!(
            "--judge value `{s}` has empty provider or model — expected `<provider>/<model>`"
        ));
    }
    Ok(JudgePolicy::Live {
        provider: provider.to_string(),
        model: model.to_string(),
    })
}

/// What kind of judge to run. Constructed by [`parse_judge_spec`]
/// so the CLI can validate before scoring starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JudgePolicy {
    /// Pure deterministic aggregator — no LLM, no API key.
    Heuristic,
    /// Live LLM judge, gated behind a transport setup that
    /// hasn't shipped yet (PR #698b).
    Live { provider: String, model: String },
}

/// Apply a policy to one scored fixture, producing a verdict.
/// Sync helpers only — the live LLM path is async and lives in
/// the CLI handler `vertical_eval_judge_live` so this crate
/// stays free of network dependencies. Live mode here returns
/// a `not_wired` placeholder; the CLI dispatcher overrides it
/// with the real verdict from a live transport call.
#[must_use]
pub fn judge_fixture(policy: &JudgePolicy, score: &FixtureScore) -> JudgeVerdict {
    match policy {
        JudgePolicy::Heuristic => heuristic_verdict(score),
        JudgePolicy::Live { provider, model } => JudgeVerdict {
            passed: score.passed,
            reason: format!(
                "live LLM judge for `{provider}/{model}` not invoked at this layer; CLI handler overrides via the live transport seam (#701). Placeholder mirrors rule-engine outcome (passed={})",
                score.passed
            ),
            source: format!("not_wired:{provider}/{model}"),
            score: Some(if score.passed { 1.0 } else { 0.0 }),
            failed_criteria: Vec::new(),
        },
    }
}

/// Aggregate the rule checks into a single verdict. Rules:
///
/// - If any rule failed → judge fails with a summary of the
///   first three failures (more truncated for output sanity).
/// - If every rule passed AND there were ≥1 checks → judge
///   passes with `"all N rule check(s) passed"`.
/// - If there were zero checks (e.g. fixture had no scoring
///   keys) → judge passes with `"no rule checks ran (vacuous)"`.
fn heuristic_verdict(score: &FixtureScore) -> JudgeVerdict {
    let total = score.checks.len();
    if total == 0 {
        return JudgeVerdict {
            passed: true,
            reason: "no rule checks ran (vacuous pass — fixture has no scoring keys)".into(),
            source: "heuristic".into(),
            score: Some(1.0),
            failed_criteria: Vec::new(),
        };
    }
    let failures: Vec<&str> = score
        .fail_reasons
        .iter()
        .map(String::as_str)
        .take(3)
        .collect();
    // Each ScoreCheck carries its own name; build the
    // failed_criteria list from the names rather than
    // re-parsing reasons. Heuristic verdicts surface the same
    // information the live judge reports.
    let failed_criteria: Vec<String> = score
        .checks
        .iter()
        .filter(|c| !c.passed)
        .map(|c| c.name.clone())
        .collect();
    if failures.is_empty() {
        JudgeVerdict {
            passed: true,
            reason: format!("all {total} rule check(s) passed"),
            source: "heuristic".into(),
            score: Some(1.0),
            failed_criteria: Vec::new(),
        }
    } else {
        let summary = failures.join("; ");
        let more = if score.fail_reasons.len() > 3 {
            format!(" (+{} more)", score.fail_reasons.len() - 3)
        } else {
            String::new()
        };
        JudgeVerdict {
            passed: false,
            reason: format!("{}/{total} rule check(s) failed: {summary}{more}", score.fail_reasons.len()),
            source: "heuristic".into(),
            score: Some(0.0),
            failed_criteria,
        }
    }
}

#[cfg(test)]
#[path = "eval_score_judge_tests.rs"]
mod tests;
