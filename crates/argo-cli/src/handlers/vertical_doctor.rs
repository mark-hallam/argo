//! `argo vertical doctor <id>` (#673) — composite "is this
//! vertical OK?" health check.
//!
//! Composes four checks into one report so an operator gets the
//! full picture without running four commands:
//!
//! | Check       | Pass criteria                                       |
//! |-------------|-----------------------------------------------------|
//! | validate    | argo_vertical::validate returns Ok(())              |
//! | eval        | every JSONL suite parses (or no suites)             |
//! | lint        | no severity=error findings                          |
//! | scored_eval | for each suite, if eval/known_good_actuals.jsonl    |
//! |             | exists alongside it, scored mode passes 100%        |
//!
//! Exit code: 1 if any check fails, 0 otherwise. The scored_eval
//! check (#695) is **convention over configuration**: doctor
//! auto-detects `known_good_actuals.jsonl` next to each suite
//! file and runs scored mode against it. When the file is
//! absent the check is reported as `skipped` (not failed) — CI
//! gates that don't curate actuals stay green.
//!
//! No new behaviour for the validate/eval/lint trio — that's
//! pure composition. The constituent commands keep their own
//! dedicated CLI surfaces (`argo vertical validate / eval /
//! lint`) and unit tests; doctor just wires them together.
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::{
    parse_actuals_jsonl, score_suite_jsonl_with_actuals, LoadedVertical, SuiteScore,
};

/// Filename doctor looks for next to each `[evaluation].suites`
/// entry. Convention is fixed (no flag) so CI behaviour stays
/// predictable across verticals.
const ACTUALS_FILENAME: &str = "known_good_actuals.jsonl";

pub async fn run(id: &str, json: bool) -> ArgoResult<()> {
    let dir = super::vertical::resolve_verticals_dir();
    let (registry, _errs) = argo_vertical::load_dir(&dir)?;
    let loaded = registry.get(id).ok_or_else(|| {
        ArgoError::Provider(format!(
            "vertical '{id}' not found in {} — run `argo vertical list` to see what's loaded",
            dir.display()
        ))
    })?;

    // 1. Structural validate
    let validate_err = argo_vertical::validate(&loaded.manifest, &loaded.root)
        .err()
        .map(|e| format!("{e}"));
    let validate_ok = validate_err.is_none();

    // 2. Eval-suite structural check
    let eval_report = super::vertical_eval::collect_eval_report(loaded);

    // 3. Lint
    use super::vertical_lint::{lint_loaded, Severity};
    let lint_findings = lint_loaded(loaded);
    let lint_error_count = lint_findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    let lint_warn_count = lint_findings
        .iter()
        .filter(|f| f.severity == Severity::Warn)
        .count();
    let lint_info_count = lint_findings
        .iter()
        .filter(|f| f.severity == Severity::Info)
        .count();
    let lint_ok = lint_error_count == 0;

    // 4. Scored eval (auto-detect known_good_actuals.jsonl). The
    // helper builds a per-suite outcome list + an aggregate
    // status — `Passed` / `Skipped` / `Failed`. Doctor's `ok`
    // flips false on `Failed` (not on `Skipped`).
    let scored = collect_scored_outcomes(loaded);
    let scored_ok = !matches!(scored.status, ScoredStatus::Failed);

    let any_failed = !validate_ok || !eval_report.ok || !lint_ok || !scored_ok;

    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": !any_failed,
                "id": id,
                "validate": {
                    "ok": validate_ok,
                    "error": validate_err,
                },
                "eval": eval_report.to_json(),
                "lint": {
                    "ok": lint_ok,
                    "error_count": lint_error_count,
                    "warn_count": lint_warn_count,
                    "info_count": lint_info_count,
                    "findings": lint_findings.iter().map(|f| serde_json::json!({
                        "severity": f.severity.label(),
                        "code": f.code,
                        "message": f.message,
                    })).collect::<Vec<_>>(),
                },
                "scored_eval": scored.to_json(),
            }))
            .unwrap_or_default()
        );
    } else {
        println!("vertical '{id}' health check:");
        let validate_marker = if validate_ok { "ok" } else { "FAIL" };
        println!("  [{validate_marker}] validate");
        if let Some(e) = &validate_err {
            println!("    error: {e}");
        }
        let eval_marker = if eval_report.ok { "ok" } else { "FAIL" };
        let suite_count = eval_report.suites.len();
        let total_fixtures: usize = eval_report
            .suites
            .iter()
            .map(|s| s.fixture_count)
            .sum();
        println!(
            "  [{eval_marker}] eval ({suite_count} suite(s), {total_fixtures} fixture(s) total)"
        );
        for s in &eval_report.suites {
            for err in &s.errors {
                println!("    {} → {err}", s.path);
            }
        }
        let lint_marker = if lint_ok { "ok" } else { "FAIL" };
        println!(
            "  [{lint_marker}] lint ({lint_error_count} error, {lint_warn_count} warn, {lint_info_count} info)"
        );
        for f in &lint_findings {
            println!("    [{}] {} — {}", f.severity.label(), f.code, f.message);
        }
        scored.print_text();
        let overall = if any_failed { "FAILED" } else { "PASSED" };
        println!();
        println!("overall: {overall}");
    }

    if any_failed {
        return Err(ArgoError::Provider(format!(
            "vertical '{id}' doctor: one or more checks failed"
        )));
    }
    Ok(())
}

/// Aggregate scored-eval outcome across every suite in the
/// vertical. `Passed` requires at least one suite scored
/// successfully AND no failures; `Skipped` means no actuals
/// files were found anywhere; `Failed` means at least one suite
/// either had failing fixtures, malformed parse, or a malformed
/// actuals file.
///
/// `pub(crate)` since #696 so the gateway doctor handler can
/// reuse the same auto-detection + JSON shape (CLI/gateway
/// symmetry — neither side reimplements the rules).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScoredStatus {
    Passed,
    Skipped,
    Failed,
}

impl ScoredStatus {
    fn label(self) -> &'static str {
        match self {
            ScoredStatus::Passed => "passed",
            ScoredStatus::Skipped => "skipped",
            ScoredStatus::Failed => "failed",
        }
    }
}

#[derive(Debug)]
pub(crate) struct ScoredOutcome {
    pub(crate) status: ScoredStatus,
    suites: Vec<ScoredSuiteOutcome>,
}

#[derive(Debug)]
struct ScoredSuiteOutcome {
    path: String,
    actuals_path: String,
    actuals_present: bool,
    /// Set when the scored run produced a SuiteScore. None when
    /// the actuals file was absent (skipped) OR parsing the
    /// actuals/suite file failed at IO time.
    score: Option<SuiteScore>,
    /// Per-suite fail reason, if any (actuals parse error, IO
    /// error, fixture failure summary). Empty when this suite
    /// passed or was skipped cleanly.
    error: Option<String>,
}

impl ScoredOutcome {
    pub(crate) fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "ok": !matches!(self.status, ScoredStatus::Failed),
            "status": self.status.label(),
            "suites": self.suites.iter().map(|s| {
                let (scored_count, no_actual, malformed, passed, failed, pass_rate) =
                    match &s.score {
                        Some(sc) => (
                            sc.scored_count,
                            sc.no_actual,
                            sc.malformed,
                            sc.passed,
                            sc.failed,
                            round3(sc.pass_rate),
                        ),
                        None => (0, 0, 0, 0, 0, 0.0),
                    };
                serde_json::json!({
                    "path": s.path,
                    "actuals_path": s.actuals_path,
                    "actuals_present": s.actuals_present,
                    "scored_count": scored_count,
                    "no_actual": no_actual,
                    "malformed": malformed,
                    "passed": passed,
                    "failed": failed,
                    "pass_rate": pass_rate,
                    "error": s.error,
                })
            }).collect::<Vec<_>>(),
        })
    }

    fn print_text(&self) {
        match self.status {
            ScoredStatus::Skipped => {
                println!(
                    "  [skip] scored eval — no `{ACTUALS_FILENAME}` alongside any suite (convention-based; create one to enable)"
                );
                return;
            }
            ScoredStatus::Passed => {
                let (passed, scored, rate) = self.totals();
                let pct = (rate * 100.0).round() as i64;
                println!(
                    "  [ok] scored eval — {passed}/{scored} pass, pass_rate {pct}%"
                );
            }
            ScoredStatus::Failed => {
                let (passed, scored, rate) = self.totals();
                let pct = (rate * 100.0).round() as i64;
                println!(
                    "  [FAIL] scored eval — {passed}/{scored} pass, pass_rate {pct}%"
                );
            }
        }
        for s in &self.suites {
            if let Some(err) = &s.error {
                println!("    {} → {err}", s.path);
            }
        }
    }

    /// Aggregate (passed, scored_count, pass_rate) across every
    /// suite that was actually scored (skipped suites contribute 0).
    fn totals(&self) -> (usize, usize, f64) {
        let mut passed = 0_usize;
        let mut scored = 0_usize;
        for s in &self.suites {
            if let Some(sc) = &s.score {
                passed += sc.passed;
                scored += sc.scored_count;
            }
        }
        let rate = if scored == 0 {
            0.0
        } else {
            passed as f64 / scored as f64
        };
        (passed, scored, rate)
    }
}

/// Pulled out of `run` so the gateway handler (`/verticals/<id>
/// /doctor`) can reuse the exact same detection + scoring
/// without copy/paste. CLI/gateway symmetry — fix once,
/// fix everywhere.
pub(crate) fn collect_scored_outcomes(loaded: &LoadedVertical) -> ScoredOutcome {
    let suites_decl = loaded
        .manifest
        .evaluation
        .as_ref()
        .map(|e| e.suites.clone())
        .unwrap_or_default();
    if suites_decl.is_empty() {
        return ScoredOutcome {
            status: ScoredStatus::Skipped,
            suites: Vec::new(),
        };
    }
    let mut outcomes: Vec<ScoredSuiteOutcome> = Vec::new();
    let mut any_present = false;
    let mut any_failed = false;
    for rel in &suites_decl {
        let suite_path = loaded.root.join(rel);
        // Compute the actuals path: same dir as the suite, name
        // is the constant ACTUALS_FILENAME. Mirror the relative
        // path in the JSON for operator clarity.
        let actuals_rel = std::path::Path::new(rel)
            .parent()
            .map(|p| p.join(ACTUALS_FILENAME))
            .unwrap_or_else(|| std::path::PathBuf::from(ACTUALS_FILENAME));
        let actuals_rel_str = actuals_rel.to_string_lossy().into_owned();
        let actuals_path = loaded.root.join(&actuals_rel);
        if !actuals_path.is_file() {
            outcomes.push(ScoredSuiteOutcome {
                path: rel.clone(),
                actuals_path: actuals_rel_str,
                actuals_present: false,
                score: None,
                error: None,
            });
            continue;
        }
        any_present = true;
        // Read the actuals file. IO failures are loud; this is a
        // misconfigured file that doctor should expose.
        let actuals_body = match std::fs::read_to_string(&actuals_path) {
            Ok(b) => b,
            Err(e) => {
                any_failed = true;
                outcomes.push(ScoredSuiteOutcome {
                    path: rel.clone(),
                    actuals_path: actuals_rel_str,
                    actuals_present: true,
                    score: None,
                    error: Some(format!("read actuals: {e}")),
                });
                continue;
            }
        };
        let actuals_map = match parse_actuals_jsonl(&actuals_body) {
            Ok(m) => m,
            Err(errs) => {
                any_failed = true;
                let summary = format!(
                    "{} actuals parse error(s): {}",
                    errs.len(),
                    errs.iter()
                        .take(3)
                        .map(|e| format!("line {}: {}", e.line, e.reason))
                        .collect::<Vec<_>>()
                        .join("; ")
                );
                outcomes.push(ScoredSuiteOutcome {
                    path: rel.clone(),
                    actuals_path: actuals_rel_str,
                    actuals_present: true,
                    score: None,
                    error: Some(summary),
                });
                continue;
            }
        };
        let suite_body = match std::fs::read_to_string(&suite_path) {
            Ok(b) => b,
            Err(e) => {
                any_failed = true;
                outcomes.push(ScoredSuiteOutcome {
                    path: rel.clone(),
                    actuals_path: actuals_rel_str,
                    actuals_present: true,
                    score: None,
                    error: Some(format!("read suite: {e}")),
                });
                continue;
            }
        };
        let (score, _unknown_ids) =
            score_suite_jsonl_with_actuals(&suite_body, &actuals_map);
        let suite_failed = score.failed > 0 || score.malformed > 0;
        if suite_failed {
            any_failed = true;
        }
        let error = if suite_failed {
            Some(format!(
                "{}/{} fixture(s) failed",
                score.failed,
                score.scored_count.max(1)
            ))
        } else {
            None
        };
        outcomes.push(ScoredSuiteOutcome {
            path: rel.clone(),
            actuals_path: actuals_rel_str,
            actuals_present: true,
            score: Some(score),
            error,
        });
    }
    let status = if any_failed {
        ScoredStatus::Failed
    } else if any_present {
        ScoredStatus::Passed
    } else {
        ScoredStatus::Skipped
    };
    ScoredOutcome {
        status,
        suites: outcomes,
    }
}

/// Round to 3 decimal places — same convention as the scored
/// eval handler's JSON shape (#693) so JSON consumers don't see
/// gratuitous precision noise.
fn round3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}
