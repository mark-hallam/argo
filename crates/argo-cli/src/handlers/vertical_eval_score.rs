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
    parse_actuals_jsonl, score_suite_jsonl, score_suite_jsonl_with_actuals, ActualsMap,
    FixtureStatus, LoadedVertical, SuiteScore,
};

/// One suite's outcome bundle: relative path, the score rollup,
/// any per-suite IO errors, and the unknown-id warning list (for
/// suites that were scored against an external actuals file).
struct SuiteOutcome {
    path: String,
    score: SuiteScore,
    errors: Vec<String>,
    unknown_actual_ids: Vec<String>,
}

pub(super) async fn run_scored(
    id: &str,
    loaded: &LoadedVertical,
    json: bool,
    actuals_path: Option<&str>,
) -> ArgoResult<()> {
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
                });
                any_failure = true;
                continue;
            }
        };
        let (score, unknown_ids) = match &actuals {
            Some(map) => score_suite_jsonl_with_actuals(&body, map),
            None => (score_suite_jsonl(&body), Vec::new()),
        };
        if score.failed > 0 || score.malformed > 0 {
            any_failure = true;
        }
        per_suite.push(SuiteOutcome {
            path: rel.clone(),
            score,
            errors,
            unknown_actual_ids: unknown_ids,
        });
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
                .map(|o| o.score.failed + o.score.malformed)
                .sum::<usize>(),
            per_suite.len()
        )))
    } else {
        Ok(())
    }
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

fn emit_json(id: &str, per_suite: &[SuiteOutcome], ok: bool) {
    let suites_json: Vec<serde_json::Value> = per_suite
        .iter()
        .map(|o| {
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
                "fixtures": &o.score.fixtures,
                "errors": &o.errors,
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

fn emit_text(id: &str, per_suite: &[SuiteOutcome]) {
    println!("vertical '{id}': scored eval report");
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
        for fx in &o.score.fixtures {
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
