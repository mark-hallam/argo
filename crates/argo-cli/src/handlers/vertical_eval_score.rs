//! `argo vertical eval <id> --score` — deterministic rule-based
//! scoring for eval suites (#693). Sibling of `vertical_eval`
//! so that file stays under the 500-LOC budget.
//!
//! Walks every `[evaluation].suites` JSONL file the manifest
//! declares, runs `argo_vertical::eval_score::score_suite_jsonl`
//! against each, and emits a per-suite + per-fixture report.
//!
//! - JSON shape:
//!   ```json
//!   {
//!     "id": "<vertical id>",
//!     "mode": "scored",
//!     "ok": true|false,
//!     "suites": [
//!       {
//!         "path": "eval/...jsonl",
//!         "fixture_count": 10,
//!         "scored_count": 7,
//!         "no_actual": 3,
//!         "malformed": 0,
//!         "passed": 6,
//!         "failed": 1,
//!         "pass_rate": 0.857,
//!         "fixtures": [
//!           {"id":"…","status":"scored","passed":true,"checks":[…],"fail_reasons":[]}
//!         ]
//!       }
//!     ]
//!   }
//!   ```
//! - Text shape: per-suite headline + per-failed-fixture detail
//!   so the operator gets actionable output without `--json`.
//!
//! Exit code: non-zero when ANY scored fixture fails. Suites
//! with only `no_actual` rows do not fail the run (the operator
//! is signalling "rules are wired but actuals aren't captured
//! yet" — a coverage warning, not a regression).
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::{
    judge_fixture, parse_actuals_jsonl, parse_judge_spec, score_suite_jsonl,
    score_suite_jsonl_with_actuals, ActualsMap, FixtureStatus, JudgePolicy, JudgeVerdict,
    LoadedVertical, SuiteScore,
};

use super::vertical_eval_score_output::{emit_json, emit_text, SuiteOutcome};

pub(super) async fn run_scored(
    id: &str,
    loaded: &LoadedVertical,
    json: bool,
    actuals_path: Option<&str>,
    judge_spec: Option<&str>,
) -> ArgoResult<()> {
    // Parse the judge spec up front so a malformed value fails
    // loudly before any IO. None = no judge (default).
    let judge_policy: Option<JudgePolicy> = match judge_spec {
        Some(spec) => Some(parse_judge_spec(spec).map_err(ArgoError::Provider)?),
        None => None,
    };
    // Load the actuals map up front. Failures here abort the
    // run — a bad actuals file means scoring would silently
    // miss rows, which defeats the point of the file.
    let actuals: Option<ActualsMap> = match actuals_path {
        Some(p) => Some(load_actuals(p)?),
        None => None,
    };

    let suites = loaded
        .manifest
        .evaluation
        .as_ref()
        .map(|e| e.suites.clone())
        .unwrap_or_default();

    let mut per_suite: Vec<SuiteOutcome> = Vec::new();
    let mut any_failure = false;
    for rel in &suites {
        let path = loaded.root.join(rel);
        let mut errors: Vec<String> = Vec::new();
        let body = match std::fs::read_to_string(&path) {
            Ok(b) => b,
            Err(e) => {
                errors.push(format!("read {}: {e}", path.display()));
                per_suite.push(SuiteOutcome {
                    path: rel.clone(),
                    score: score_suite_jsonl(""),
                    errors,
                    unknown_actual_ids: Vec::new(),
                    judge_verdicts: Vec::new(),
                });
                any_failure = true;
                continue;
            }
        };
        let (score, unknown_ids) = match &actuals {
            Some(map) => score_suite_jsonl_with_actuals(&body, map),
            None => (score_suite_jsonl(&body), Vec::new()),
        };
        // #698: per-fixture judge pass — only for Scored
        // fixtures (no_actual / malformed don't get judged).
        // When the judge says "fail" but the rule engine said
        // "pass", we DO flip the suite to failure so the
        // operator sees the disagreement; symmetrically a judge
        // "pass" never rescues a rule-engine fail.
        //
        // #701: when the policy is `Live`, replace the
        // placeholder verdicts with real LLM verdicts via the
        // live backend. Heuristic mode keeps the pure helper.
        let judge_verdicts: Vec<Option<JudgeVerdict>> = match &judge_policy {
            None => score.fixtures.iter().map(|_| None).collect(),
            Some(JudgePolicy::Heuristic) => score
                .fixtures
                .iter()
                .map(|fx| match fx.status {
                    FixtureStatus::Scored => {
                        Some(judge_fixture(&JudgePolicy::Heuristic, fx))
                    }
                    _ => None,
                })
                .collect(),
            Some(JudgePolicy::Live { provider, model }) => {
                live_judge_for_suite(provider, model, &score, &body, actuals.as_ref())
                    .await?
            }
        };
        let judge_failed = judge_verdicts
            .iter()
            .any(|v| matches!(v, Some(jv) if !jv.passed));
        if score.failed > 0 || score.malformed > 0 || judge_failed {
            any_failure = true;
        }
        per_suite.push(SuiteOutcome {
            path: rel.clone(),
            score,
            errors,
            unknown_actual_ids: unknown_ids,
            judge_verdicts,
        });
    }

    // #701c: systemic-failure detector. If EVERY scored
    // fixture's judge errored out (judge_call_failed), that's
    // a configuration problem (manifest, key, network), not a
    // GTM-quality regression. Surface it as a distinct error
    // so operators don't read "all 4 fixtures failed" as
    // "the agent's outputs are bad".
    let systemic = detect_systemic_judge_failure(&per_suite);

    if json {
        emit_json(id, &per_suite, !any_failure, systemic);
    } else {
        emit_text(id, &per_suite, systemic);
    }

    if systemic {
        return Err(ArgoError::Provider(format!(
            "vertical '{id}': live judge appears misconfigured — every scored fixture's judge call failed (e.g. provider 400, network error, missing manifest field). \
             Re-run with --judge heuristic to confirm the rule engine still works, then check the per-fixture `judge.reason` strings for the underlying error."
        )));
    }
    if any_failure {
        Err(ArgoError::Schema(format!(
            "vertical '{id}': scored eval has {} failure(s) across {} suite(s)",
            per_suite
                .iter()
                .map(|o| o.score.failed + o.score.malformed)
                .sum::<usize>(),
            per_suite.len()
        )))
    } else {
        Ok(())
    }
}

/// `true` when EVERY scored fixture across all suites had its
/// judge call fail (recognised by the
/// `failed_criteria.contains("judge_call_failed")` marker
/// emitted by `vertical_eval_judge_live::grade_fixture` on
/// transport / parse errors). Returns `false` when there are
/// no scored fixtures at all (no judge ran → nothing to detect).
fn detect_systemic_judge_failure(per_suite: &[SuiteOutcome]) -> bool {
    let mut total_judged = 0_usize;
    let mut judge_call_failed = 0_usize;
    for o in per_suite {
        for v in o.judge_verdicts.iter().flatten() {
            total_judged += 1;
            if v.failed_criteria.iter().any(|c| c == "judge_call_failed") {
                judge_call_failed += 1;
            }
        }
    }
    total_judged > 0 && judge_call_failed == total_judged
}

/// Read + parse the actuals file. All errors fail loud — the
/// operator gets one CI message with every line number that
/// needs fixing.
fn load_actuals(path: &str) -> ArgoResult<ActualsMap> {
    let body = std::fs::read_to_string(path)
        .map_err(|e| ArgoError::Io(format!("read actuals file {path}: {e}")))?;
    parse_actuals_jsonl(&body).map_err(|errs| {
        let mut msg = format!("actuals file {path} has {} error(s):", errs.len());
        for e in errs {
            msg.push_str(&format!("\n  line {}: {}", e.line, e.reason));
        }
        ArgoError::Schema(msg)
    })
}

/// Build a live judge backend, re-parse the suite to extract
/// per-fixture (input, expected, actual) triples, and grade
/// every Scored fixture (#701). Returns one `Option<JudgeVerdict>`
/// per `score.fixtures` entry, indexed parallel.
///
/// Errors propagate from backend construction (missing manifest,
/// unsupported provider family, missing API key) — those are
/// configuration problems the operator must fix before any
/// fixture gets graded.
async fn live_judge_for_suite(
    provider: &str,
    model: &str,
    score: &SuiteScore,
    suite_body: &str,
    actuals: Option<&ActualsMap>,
) -> ArgoResult<Vec<Option<JudgeVerdict>>> {
    // Cross-family dispatch (#701b): pick the right backend based on
    // the manifest's `[provider].family`. Pre-#701b this hard-coded
    // the Anthropic builder and 400'd for any other family at the
    // dispatcher; now OpenAI Compatible / OpenAI Responses / Bedrock
    // all work, with per-family temperature + max_tokens policy.
    let backend =
        super::vertical_eval_judge_live::build_judge_backend(provider, model).await?;
    let inputs = collect_fixture_inputs(suite_body, actuals);
    Ok(super::vertical_eval_judge_live::judge_all_fixtures(
        backend.as_ref(),
        &score.fixtures,
        &inputs,
    )
    .await)
}

/// Re-parse the suite body into `(id, input, expected, actual)`
/// tuples for the live judge. Applies the same actuals overlay
/// the rule engine uses so the live judge sees identical
/// content. Skips rows that don't parse as JSON objects (the
/// rule engine already surfaced those as Malformed).
fn collect_fixture_inputs(
    body: &str,
    actuals: Option<&ActualsMap>,
) -> Vec<(String, String, serde_json::Value, String)> {
    let mut out: Vec<(String, String, serde_json::Value, String)> = Vec::new();
    for raw in body.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut v: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let Some(id) = v.get("id").and_then(|x| x.as_str()).map(String::from) else {
            continue;
        };
        // Apply external actuals overlay (file wins over row-
        // embedded actual, same precedence as the rule engine).
        if let Some(map) = actuals {
            if let Some(a) = map.get(&id) {
                if let Some(obj) = v.as_object_mut() {
                    obj.insert(
                        "actual".to_string(),
                        serde_json::Value::String(a.clone()),
                    );
                }
            }
        }
        let input = v
            .get("input")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let expected = v.get("expected").cloned().unwrap_or(serde_json::Value::Null);
        let Some(actual) = v.get("actual").and_then(|x| x.as_str()).map(String::from) else {
            // No actual → won't be Scored, so the live judge
            // will skip it. Still emit the tuple so id matching
            // doesn't false-negative.
            continue;
        };
        out.push((id, input, expected, actual));
    }
    out
}
