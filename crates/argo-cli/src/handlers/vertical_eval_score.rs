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

/// One suite's outcome bundle: relative path, the score rollup,
/// any per-suite IO errors, the unknown-id warning list (for
/// suites scored against an external actuals file), and an
/// optional per-fixture judge verdict map (#698).
struct SuiteOutcome {
    path: String,
    score: SuiteScore,
    errors: Vec<String>,
    unknown_actual_ids: Vec<String>,
    /// Indexed parallel to `score.fixtures`. `None` when no
    /// judge was configured (`--judge` not set) or the fixture
    /// wasn't scored (no actual). Always-present + always
    /// `Some` for `Scored` fixtures when a judge is active.
    judge_verdicts: Vec<Option<JudgeVerdict>>,
}

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
        for jv in &o.judge_verdicts {
            if let Some(v) = jv {
                total_judged += 1;
                if v.failed_criteria.iter().any(|c| c == "judge_call_failed") {
                    judge_call_failed += 1;
                }
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

fn emit_json(
    id: &str,
    per_suite: &[SuiteOutcome],
    ok: bool,
    judge_systemic_failure: bool,
) {
    let suites_json: Vec<serde_json::Value> = per_suite
        .iter()
        .map(|o| {
            // #698: pair each fixture with its (optional) judge
            // verdict by index. Skip-if-none in the JSON so
            // unscored mode JSON shape stays unchanged.
            let fixtures_json: Vec<serde_json::Value> = o
                .score
                .fixtures
                .iter()
                .zip(o.judge_verdicts.iter())
                .map(|(fx, jv)| {
                    let mut v = serde_json::to_value(fx).unwrap_or_else(|_| serde_json::json!({}));
                    if let Some(verdict) = jv {
                        if let Some(obj) = v.as_object_mut() {
                            obj.insert(
                                "judge".to_string(),
                                serde_json::to_value(verdict)
                                    .unwrap_or_else(|_| serde_json::Value::Null),
                            );
                        }
                    }
                    v
                })
                .collect();
            serde_json::json!({
                "path": o.path,
                "fixture_count": o.score.fixture_count,
                "scored_count": o.score.scored_count,
                "no_actual": o.score.no_actual,
                "malformed": o.score.malformed,
                "passed": o.score.passed,
                "failed": o.score.failed,
                "pass_rate": round3(o.score.pass_rate),
                "unknown_actual_ids": &o.unknown_actual_ids,
                "fixtures": fixtures_json,
                "errors": &o.errors,
            })
        })
        .collect();
    // #701c: surface the systemic-failure signal at the top
    // level so consumers don't need to scan per-fixture
    // failed_criteria. `false` is the common case and stays
    // unchanged from #698's shape.
    let payload = serde_json::json!({
        "id": id,
        "mode": "scored",
        "ok": ok,
        "judge_systemic_failure": judge_systemic_failure,
        "suites": suites_json,
    });
    println!(
        "{}",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into())
    );
}

fn emit_text(id: &str, per_suite: &[SuiteOutcome], judge_systemic_failure: bool) {
    println!("vertical '{id}': scored eval report");
    if judge_systemic_failure {
        // Print the warning ABOVE the per-suite detail so
        // operators see it first. The message is written to
        // be actionable: route to the offline fallback +
        // point at where the underlying error lives.
        println!();
        println!(
            "  WARN: live judge appears misconfigured — every scored fixture's judge call failed."
        );
        println!(
            "        Re-run with `--judge heuristic` to confirm rules still work, then read"
        );
        println!(
            "        per-fixture `judge.reason` strings below for the underlying error."
        );
        println!();
    }
    if per_suite.is_empty() {
        println!("  (manifest declares no [evaluation].suites)");
        return;
    }
    for o in per_suite {
        let pct = (round3(o.score.pass_rate) * 100.0).round() as i64;
        let marker = if o.score.failed == 0 && o.score.malformed == 0 && o.errors.is_empty() {
            "ok"
        } else {
            "FAIL"
        };
        println!(
            "  [{marker}] {} — {} scored ({} pass / {} fail), {} no-actual, {} malformed (pass_rate {pct}%)",
            o.path,
            o.score.scored_count,
            o.score.passed,
            o.score.failed,
            o.score.no_actual,
            o.score.malformed,
        );
        for err in &o.errors {
            println!("        IO: {err}");
        }
        if !o.unknown_actual_ids.is_empty() {
            // Warning, not failure: actuals file contains rows
            // for ids that no fixture in the suite uses. Helps
            // catch typos / drift between transcript and suite.
            let mut ids = o.unknown_actual_ids.clone();
            ids.sort();
            println!(
                "        warn: {} unknown actuals id(s) ignored: {}",
                ids.len(),
                ids.join(", ")
            );
        }
        for (fx, jv) in o.score.fixtures.iter().zip(o.judge_verdicts.iter()) {
            if !matches!(fx.status, FixtureStatus::Scored) {
                continue;
            }
            let judge_failed = matches!(jv, Some(v) if !v.passed);
            if fx.passed && !judge_failed {
                continue;
            }
            println!("        FAIL `{}`:", fx.id);
            for r in &fx.fail_reasons {
                println!("          - {r}");
            }
            if let Some(v) = jv {
                let marker = if v.passed { "ok" } else { "FAIL" };
                println!("          - judge[{marker}] ({}): {}", v.source, v.reason);
            }
        }
    }
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
    let backend =
        super::vertical_eval_judge_live::build_anthropic_backend(provider, model)?;
    let inputs = collect_fixture_inputs(suite_body, actuals);
    Ok(super::vertical_eval_judge_live::judge_all_fixtures(
        &backend,
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

/// Round to 3 decimal places — keeps machine-readable output
/// stable across runs without dragging in a formatting crate.
fn round3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}
