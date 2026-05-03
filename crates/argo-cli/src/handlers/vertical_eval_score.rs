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
use argo_vertical::{score_suite_jsonl, FixtureStatus, LoadedVertical, SuiteScore};

pub(super) async fn run_scored(
    id: &str,
    loaded: &LoadedVertical,
    json: bool,
) -> ArgoResult<()> {
    let suites = loaded
        .manifest
        .evaluation
        .as_ref()
        .map(|e| e.suites.clone())
        .unwrap_or_default();

    let mut per_suite: Vec<(String, SuiteScore, Vec<String>)> = Vec::new();
    let mut any_failure = false;
    for rel in &suites {
        let path = loaded.root.join(rel);
        let mut errors: Vec<String> = Vec::new();
        let body = match std::fs::read_to_string(&path) {
            Ok(b) => b,
            Err(e) => {
                errors.push(format!("read {}: {e}", path.display()));
                let empty = score_suite_jsonl("");
                per_suite.push((rel.clone(), empty, errors));
                any_failure = true;
                continue;
            }
        };
        let score = score_suite_jsonl(&body);
        if score.failed > 0 || score.malformed > 0 {
            any_failure = true;
        }
        per_suite.push((rel.clone(), score, errors));
    }

    if json {
        emit_json(id, &per_suite, !any_failure);
    } else {
        emit_text(id, &per_suite);
    }

    if any_failure {
        Err(ArgoError::Schema(format!(
            "vertical '{id}': scored eval has {} failure(s) across {} suite(s)",
            per_suite
                .iter()
                .map(|(_, s, _)| s.failed + s.malformed)
                .sum::<usize>(),
            per_suite.len()
        )))
    } else {
        Ok(())
    }
}

fn emit_json(id: &str, per_suite: &[(String, SuiteScore, Vec<String>)], ok: bool) {
    let suites_json: Vec<serde_json::Value> = per_suite
        .iter()
        .map(|(path, score, errors)| {
            serde_json::json!({
                "path": path,
                "fixture_count": score.fixture_count,
                "scored_count": score.scored_count,
                "no_actual": score.no_actual,
                "malformed": score.malformed,
                "passed": score.passed,
                "failed": score.failed,
                "pass_rate": round3(score.pass_rate),
                "fixtures": &score.fixtures,
                "errors": errors,
            })
        })
        .collect();
    let payload = serde_json::json!({
        "id": id,
        "mode": "scored",
        "ok": ok,
        "suites": suites_json,
    });
    println!(
        "{}",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into())
    );
}

fn emit_text(id: &str, per_suite: &[(String, SuiteScore, Vec<String>)]) {
    println!("vertical '{id}': scored eval report");
    if per_suite.is_empty() {
        println!("  (manifest declares no [evaluation].suites)");
        return;
    }
    for (path, score, errors) in per_suite {
        let pct = (round3(score.pass_rate) * 100.0).round() as i64;
        println!(
            "  [{}] {path} — {} scored ({} pass / {} fail), {} no-actual, {} malformed (pass_rate {pct}%)",
            if score.failed == 0 && score.malformed == 0 && errors.is_empty() {
                "ok"
            } else {
                "FAIL"
            },
            score.scored_count,
            score.passed,
            score.failed,
            score.no_actual,
            score.malformed,
        );
        for err in errors {
            println!("        IO: {err}");
        }
        for fx in &score.fixtures {
            if !matches!(fx.status, FixtureStatus::Scored) {
                continue;
            }
            if fx.passed {
                continue;
            }
            println!("        FAIL `{}`:", fx.id);
            for r in &fx.fail_reasons {
                println!("          - {r}");
            }
        }
    }
}

/// Round to 3 decimal places — keeps machine-readable output
/// stable across runs without dragging in a formatting crate.
fn round3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}
