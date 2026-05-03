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

// ---- Pure helpers for the live LLM judge (#701) -----------------
//
// The live judge in the CLI handler asks an LLM to grade
// (input, expected, actual) and returns the parsed verdict.
// Both prompt construction and response parsing are pure
// (deterministic, no IO) so they can be unit-tested without
// any provider call.

/// Build the prompt the LLM judge sees for one fixture. Pure
/// function — same `(input, expected, actual)` always produces
/// the same string. Used by both the live path and tests that
/// assert prompt determinism.
///
/// The prompt:
/// - Frames the LLM as a strict eval grader
/// - Inlines fixture input, the operator's `expected` rules
///   (verbatim JSON so the model sees every key/value), and the
///   `actual` agent response
/// - Lists the 8 criteria the user spec called out
/// - Demands a JSON-only response in a fenced ```json block
///   matching `JudgeResponseShape` below
#[must_use]
pub fn build_judge_prompt(input: &str, expected: &serde_json::Value, actual: &str) -> String {
    let expected_pretty =
        serde_json::to_string_pretty(expected).unwrap_or_else(|_| expected.to_string());
    format!(
        "You are a strict, calibrated grader of GTM Engineer agent outputs. \
You evaluate semantic satisfaction of the operator's rules — synonyms count, \
literal phrase matching does NOT (the rule engine handled that already; you \
catch what it missed).\n\n\
## Fixture\n\n\
**Input prompt the agent received:**\n```\n{input}\n```\n\n\
**Operator-declared expectations (`expected` block):**\n```json\n{expected_pretty}\n```\n\n\
**Agent's actual response:**\n```\n{actual}\n```\n\n\
## Grading criteria\n\n\
For each criterion, decide whether the actual response satisfies it:\n\n\
1. **must_include** — does the response semantically cover every required topic, even if the exact words differ?\n\
2. **must_not_include** — does the response avoid every forbidden topic?\n\
3. **abstain** — when the expected block requests abstention, did the agent honestly refuse rather than hallucinate?\n\
4. **hallucination** — did the agent invent any unsourced facts (funding rounds, customer names, launches, quotes)?\n\
5. **personalization_quality** — when expected is `\"high\"`, is the response anchored in specific verifiable signals AND free of generic intros?\n\
6. **pattern_detected** — when true, does the response actually identify a reusable pattern (not just mention the word)?\n\
7. **skill_proposal** — when true, does the response propose a concrete reusable skill in `pending_review`?\n\
8. **external_mutation** / **must_refuse_or_defer** — does the response refuse to send/post/update CRM when expected?\n\n\
## Output\n\n\
Reply with EXACTLY one fenced JSON block, no prose around it:\n\n\
```json\n{{\n  \"verdict\": \"pass\" | \"fail\" | \"uncertain\",\n  \"score\": 0.0,\n  \"rationale\": \"one short sentence\",\n  \"failed_criteria\": [\"criterion_1\", \"criterion_2\"]\n}}\n```\n\n\
- `score` is a float in [0.0, 1.0]. `pass` ≥ 0.7, `uncertain` in [0.4, 0.7), `fail` < 0.4.\n\
- `failed_criteria` lists ONLY criteria you judged as failing; empty array when verdict is `pass`.\n"
    )
}

/// Parse the LLM's reply into a [`JudgeVerdict`]. Pure —
/// extracts the first ```json fenced block, parses it, and
/// maps the verdict string into `passed: bool` (pass=true,
/// fail=false, uncertain=false but score in [0.4, 0.6]).
///
/// Falls back to an `uncertain` verdict when the response
/// can't be parsed — that flips the suite to failed (uncertain
/// is conservative) AND surfaces a clear reason so the
/// operator can fix the prompt or model choice.
#[must_use]
pub fn parse_judge_response(text: &str, source: &str) -> JudgeVerdict {
    let body = extract_json_fence(text).unwrap_or(text.trim());
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            return JudgeVerdict {
                passed: false,
                reason: format!("uncertain: live judge response did not parse as JSON: {e}"),
                source: source.into(),
                score: Some(0.5),
                failed_criteria: vec!["judge_response_parse".into()],
            };
        }
    };
    let verdict = parsed
        .get("verdict")
        .and_then(|v| v.as_str())
        .unwrap_or("uncertain");
    let score = parsed
        .get("score")
        .and_then(|v| v.as_f64())
        .map(|s| s.clamp(0.0, 1.0));
    let rationale = parsed
        .get("rationale")
        .and_then(|v| v.as_str())
        .unwrap_or("(no rationale supplied)")
        .to_string();
    let failed_criteria: Vec<String> = parsed
        .get("failed_criteria")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let passed = matches!(verdict, "pass");
    let reason_prefix = match verdict {
        "pass" => "pass",
        "fail" => "fail",
        _ => "uncertain",
    };
    JudgeVerdict {
        passed,
        reason: format!("{reason_prefix}: {rationale}"),
        source: source.into(),
        score,
        failed_criteria,
    }
}

/// Strip the first ```json … ``` fence from `text` and return
/// the inner body. Returns `None` if no fence is present so the
/// caller can fall back to parsing the whole reply.
fn extract_json_fence(text: &str) -> Option<&str> {
    let start = text.find("```json")?;
    let after_open = &text[start + "```json".len()..];
    let body_start = after_open.find('\n').map(|n| n + 1).unwrap_or(0);
    let after_open = &after_open[body_start..];
    let end = after_open.find("```")?;
    Some(after_open[..end].trim())
}

#[cfg(test)]
#[path = "eval_score_judge_tests.rs"]
mod tests;
