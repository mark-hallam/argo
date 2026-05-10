//! Scored-eval check for `argo vertical doctor` (#695). Auto-detects
//! `known_good_actuals.jsonl` next to each suite file and runs scored
//! mode against it. When the file is absent the check is reported as
//! `skipped` (not failed) — CI gates that don't curate actuals stay
//! green.
//!
//! Split out of `vertical_doctor.rs` to keep that file under the
//! 500-LOC ceiling. Gateway handler reuses `ScoredStatus` /
//! `ScoredOutcome` / `collect_scored_outcomes` via re-exports from
//! `vertical_doctor` (CLI/gateway symmetry).
use argo_vertical::{
    parse_actuals_jsonl, score_suite_jsonl_with_actuals, LoadedVertical, SuiteScore,
};

/// Filename doctor looks for next to each `[evaluation].suites`
/// entry. Convention is fixed (no flag) so CI behaviour stays
/// predictable across verticals.
pub(crate) const ACTUALS_FILENAME: &str = "known_good_actuals.jsonl";

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

    pub(crate) fn print_text(&self) {
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
