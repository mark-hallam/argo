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
use serde::Serialize;

use crate::eval_score::FixtureScore;

/// One judge outcome per scored fixture. `passed` is the
/// machine-readable pass/fail bit; `reason` is the operator-
/// facing explanation; `source` records which judge produced
/// the verdict so audit + debugging can trace it.
#[derive(Debug, Clone, Serialize)]
pub struct JudgeVerdict {
    pub passed: bool,
    pub reason: String,
    /// `"heuristic"` for the rule-aggregating judge,
    /// `"live:<provider>/<model>"` for the live LLM judge,
    /// `"not_wired:<provider>/<model>"` for the placeholder.
    pub source: String,
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
/// Heuristic mode aggregates the rule checks; Live mode emits
/// a `not_yet_wired` placeholder so operators see exactly which
/// fixture would be live-judged once #698b lands.
#[must_use]
pub fn judge_fixture(policy: &JudgePolicy, score: &FixtureScore) -> JudgeVerdict {
    match policy {
        JudgePolicy::Heuristic => heuristic_verdict(score),
        JudgePolicy::Live { provider, model } => JudgeVerdict {
            passed: score.passed, // mirror the rule engine until live judge ships
            reason: format!(
                "live LLM judge for `{provider}/{model}` is not yet wired (PR #698b); placeholder mirrors rule-engine outcome (passed={})",
                score.passed
            ),
            source: format!("not_wired:{provider}/{model}"),
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
///   Distinct enough that operators can spot the corner.
fn heuristic_verdict(score: &FixtureScore) -> JudgeVerdict {
    let total = score.checks.len();
    if total == 0 {
        return JudgeVerdict {
            passed: true,
            reason: "no rule checks ran (vacuous pass — fixture has no scoring keys)".into(),
            source: "heuristic".into(),
        };
    }
    let failures: Vec<&str> = score
        .fail_reasons
        .iter()
        .map(String::as_str)
        .take(3)
        .collect();
    if failures.is_empty() {
        JudgeVerdict {
            passed: true,
            reason: format!("all {total} rule check(s) passed"),
            source: "heuristic".into(),
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
        }
    }
}

#[cfg(test)]
#[path = "eval_score_judge_tests.rs"]
mod tests;
