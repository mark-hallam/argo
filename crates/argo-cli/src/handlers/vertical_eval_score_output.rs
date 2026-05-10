//! Output rendering for `argo vertical eval <id> --score` —
//! per-suite JSON + text emitters split out from
//! `vertical_eval_score.rs` to keep that file under the 500-LOC
//! budget. The data carrier `SuiteOutcome` lives here too so
//! the renderer owns its own input shape; the runner imports it
//! back via `super::vertical_eval_score_output::SuiteOutcome`.
use argo_vertical::{FixtureStatus, JudgeVerdict, SuiteScore};

/// One suite's outcome bundle: relative path, the score rollup,
/// any per-suite IO errors, the unknown-id warning list (for
/// suites scored against an external actuals file), and an
/// optional per-fixture judge verdict map (#698).
pub(super) struct SuiteOutcome {
    pub(super) path: String,
    pub(super) score: SuiteScore,
    pub(super) errors: Vec<String>,
    pub(super) unknown_actual_ids: Vec<String>,
    /// Indexed parallel to `score.fixtures`. `None` when no
    /// judge was configured (`--judge` not set) or the fixture
    /// wasn't scored (no actual). Always-present + always
    /// `Some` for `Scored` fixtures when a judge is active.
    pub(super) judge_verdicts: Vec<Option<JudgeVerdict>>,
}

pub(super) fn emit_json(
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
                                    .unwrap_or(serde_json::Value::Null),
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

pub(super) fn emit_text(id: &str, per_suite: &[SuiteOutcome], judge_systemic_failure: bool) {
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

/// Round to 3 decimal places — keeps machine-readable output
/// stable across runs without dragging in a formatting crate.
fn round3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}
