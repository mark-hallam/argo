//! `argo eval skill-ab` — the closed self-improvement loop (DEC-023 / P4).
//!
//! Runs TWO versions of one skill through the REAL eval scorer
//! ([`argo_skill::run_ab_eval`]) and reports the measured pass-rate delta.
//! With `--record`, persists the candidate's result into
//! `skill_reeval_history` so the worker Evals tab reflects it.
//!
//! No fabricated numbers: the body-aware deterministic executor returns
//! each version's body, so a `contains` predicate genuinely passes only
//! when that body carries the required guidance. An older body that lacks
//! a fix fails; an improved body that adds it passes. The delta is a true
//! measurement.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_skill::{record_candidate_reeval, run_ab_eval, SkillAbEvalReport, SkillStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use serde_json::json;
use std::sync::Arc;

/// `argo eval skill-ab <skill> --baseline vN --candidate vM` entry point.
pub async fn run(
    skill: &str,
    baseline: &str,
    candidate: &str,
    record: bool,
    json_out: bool,
) -> ArgoResult<()> {
    let skills = open_skill_store().await?;
    let report = run_ab_eval(&skills, skill, baseline, candidate).await?;
    let recorded = if record {
        Some(record_candidate_reeval(&skills, &report).await?)
    } else {
        None
    };
    if json_out {
        print_json(&report, recorded.as_deref())?;
    } else {
        print_human(&report, recorded.as_deref());
    }
    Ok(())
}

fn print_json(report: &SkillAbEvalReport, recorded: Option<&str>) -> ArgoResult<()> {
    let body = json!({
        "ok": true,
        "report": report,
        "recorded_eval_run_id": recorded,
    });
    let s = serde_json::to_string(&body)
        .map_err(|e| ArgoError::Provider(format!("serialize skill-ab output: {e}")))?;
    println!("{s}");
    Ok(())
}

fn print_human(report: &SkillAbEvalReport, recorded: Option<&str>) {
    println!("skill-ab '{}' — eval-measured A/B", report.skill_name);
    println!("  cases:     {}", report.case_count);
    println!(
        "  baseline   {}: {}/{} passed ({})",
        report.baseline.version,
        report.baseline.pass,
        report.baseline.pass + report.baseline.fail,
        pct(report.baseline.pass_rate)
    );
    println!(
        "  candidate  {}: {}/{} passed ({})",
        report.candidate.version,
        report.candidate.pass,
        report.candidate.pass + report.candidate.fail,
        pct(report.candidate.pass_rate)
    );
    println!(
        "  delta:     {} ({})",
        signed_pct(report.pass_rate_delta),
        verdict(report)
    );
    if let Some(id) = recorded {
        println!("  recorded:  skill_reeval_history eval_run_id={id}");
    }
}

fn verdict(report: &SkillAbEvalReport) -> &'static str {
    if report.improved {
        "improved"
    } else if report.pass_rate_delta < 0.0 {
        "regressed"
    } else {
        "no change"
    }
}

fn pct(rate: f32) -> String {
    format!("{:.0}%", rate * 100.0)
}

fn signed_pct(delta: f32) -> String {
    format!("{:+.0}pp", delta * 100.0)
}

async fn open_skill_store() -> ArgoResult<SkillStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    Ok(SkillStore::new(Arc::new(store)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pct_formats_whole_percent() {
        assert_eq!(pct(0.0), "0%");
        assert_eq!(pct(1.0), "100%");
        assert_eq!(pct(0.8), "80%");
    }

    #[test]
    fn signed_pct_shows_sign() {
        assert_eq!(signed_pct(0.5), "+50pp");
        assert_eq!(signed_pct(-0.25), "-25pp");
        assert_eq!(signed_pct(0.0), "+0pp");
    }

    fn report(base: f32, cand: f32) -> SkillAbEvalReport {
        use argo_skill::VariantEvalResult;
        SkillAbEvalReport {
            skill_name: "s".into(),
            case_count: 4,
            baseline: VariantEvalResult {
                version: "v1".into(),
                pass: 0,
                fail: 4,
                pass_rate: base,
            },
            candidate: VariantEvalResult {
                version: "v2".into(),
                pass: 4,
                fail: 0,
                pass_rate: cand,
            },
            pass_rate_delta: cand - base,
            improved: cand > base,
        }
    }

    #[test]
    fn verdict_branches() {
        assert_eq!(verdict(&report(0.0, 1.0)), "improved");
        assert_eq!(verdict(&report(1.0, 0.5)), "regressed");
        assert_eq!(verdict(&report(0.5, 0.5)), "no change");
    }
}
