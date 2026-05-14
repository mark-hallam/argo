//! Unit tests for eval regression detection.

use super::*;

fn fixture(id: usize, passed: bool, score: f64) -> EvalRunFixture {
    EvalRunFixture {
        fixture: format!("f-{id:03}"),
        skill: "invoice-extract".into(),
        metric: "exact".into(),
        passed,
        score,
    }
}

fn run(id: &str, pass_until: usize, total: usize, score: f64) -> EvalRunReport {
    EvalRunReport {
        id: id.into(),
        vertical: "ap".into(),
        created_at: "2026-05-14T00:00:00Z".into(),
        fixtures: (0..total)
            .map(|i| fixture(i, i < pass_until, score))
            .collect(),
    }
}

#[test]
fn deliberate_pass_rate_regression_blocks() {
    let baseline = run("v1", 90, 100, 0.90);
    let candidate = run("v2", 60, 100, 0.60);
    let report = detect_eval_regression(&baseline, &candidate, EvalRegressionConfig::default());
    assert_eq!(report.decision, EvalRegressionDecision::Regressed);
    assert!(report.blocks_promotion());
    let pass_flips = report
        .fixture_regressions
        .iter()
        .filter(|r| r.passed_baseline && !r.passed_candidate)
        .count();
    assert_eq!(pass_flips, 30);
    assert!(report.z_score >= 1.96);
}

#[test]
fn noise_level_variation_does_not_block() {
    let baseline = run("v1", 90, 100, 0.90);
    let candidate = run("v2", 88, 100, 0.88);
    let report = detect_eval_regression(&baseline, &candidate, EvalRegressionConfig::default());
    assert_eq!(report.decision, EvalRegressionDecision::Stable);
    assert!(!report.blocks_promotion());
}

#[test]
fn score_only_regression_blocks() {
    let baseline = run("v1", 100, 100, 0.96);
    let candidate = run("v2", 100, 100, 0.81);
    let report = detect_eval_regression(&baseline, &candidate, EvalRegressionConfig::default());
    assert_eq!(report.decision, EvalRegressionDecision::Regressed);
    assert!(report.pass_rate_delta.abs() < f64::EPSILON);
    assert!(report.mean_score_delta < -0.10);
}

#[test]
fn too_few_common_fixtures_yields_no_baseline() {
    let baseline = run("v1", 5, 5, 0.90);
    let candidate = run("v2", 0, 5, 0.20);
    let report = detect_eval_regression(&baseline, &candidate, EvalRegressionConfig::default());
    assert_eq!(report.decision, EvalRegressionDecision::NoBaseline);
    assert!(!report.blocks_promotion());
}

#[test]
fn compares_only_shared_fixture_ids() {
    let baseline = run("v1", 25, 30, 0.90);
    let mut candidate = run("v2", 20, 20, 0.90);
    candidate.fixtures.push(EvalRunFixture {
        fixture: "candidate-only".into(),
        skill: "invoice-extract".into(),
        metric: "exact".into(),
        passed: false,
        score: 0.0,
    });
    let report = detect_eval_regression(&baseline, &candidate, EvalRegressionConfig::default());
    assert_eq!(report.common_fixtures, 20);
    assert_eq!(report.decision, EvalRegressionDecision::Stable);
}

#[test]
fn promotion_gate_requires_override_for_regression() {
    let baseline = run("v1", 90, 100, 0.90);
    let candidate = run("v2", 60, 100, 0.60);
    let report = detect_eval_regression(&baseline, &candidate, EvalRegressionConfig::default());
    let err = ensure_promotion_allowed(&report, false).unwrap_err();
    assert!(err.contains("eval_regression"));
    assert!(ensure_promotion_allowed(&report, true).is_ok());
}

#[test]
fn promotion_gate_allows_stable_report_without_override() {
    let baseline = run("v1", 90, 100, 0.90);
    let candidate = run("v2", 90, 100, 0.91);
    let report = detect_eval_regression(&baseline, &candidate, EvalRegressionConfig::default());
    assert_eq!(report.decision, EvalRegressionDecision::Stable);
    assert!(ensure_promotion_allowed(&report, false).is_ok());
}
