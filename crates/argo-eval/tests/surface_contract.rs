//! Public-surface contract tests for `argo-eval`.
//!
//! These tests intentionally import through `argo_eval::*` so the
//! shared eval harness API used by Forge, regression gates, honesty
//! diagnostics, streaming output, and fixture self-healing cannot drift
//! silently behind private module tests.

use argo_eval::*;
use argo_types::{ArgoError, ArgoResult, SessionId};
use async_trait::async_trait;
use serde_json::{json, Value};

const EXPECTED_PUBLIC_MODULES: &[&str] = &["honesty", "regression", "self_healing"];

const EXPECTED_REEXPORTS: &[&str] = &[
    "apply_fixture_update",
    "compare_runs",
    "detect_eval_regression",
    "ensure_promotion_allowed",
    "format_simple_line",
    "format_summary_line",
    "format_verbose_line",
    "load_jsonl",
    "propose_fixture_update",
    "run_eval",
    "run_honesty_test",
    "AuditSampler",
    "ComparisonGroup",
    "ComparisonItem",
    "ComparisonReport",
    "EvalFixtureRegression",
    "EvalMetric",
    "EvalRecord",
    "EvalRegressionConfig",
    "EvalRegressionDecision",
    "EvalRegressionReport",
    "EvalRunFixture",
    "EvalRunReport",
    "EvalSubject",
    "EvalSummary",
    "EvalTask",
    "FixtureFailureJudgment",
    "FixtureFailureKind",
    "FixtureUpdateProposal",
    "FixtureUpdateReceipt",
    "GroupComparison",
    "HonestyConfig",
    "HonestyResult",
    "OperatorFixtureApproval",
    "SampleVerdict",
    "SelfHealingConfig",
    "SkillSample",
    "StreamMode",
];

#[test]
fn lib_public_surface_inventory_is_intentional() {
    let lib = include_str!("../src/lib.rs");
    assert_eq!(public_modules(lib), EXPECTED_PUBLIC_MODULES);
    let mut expected_reexports: Vec<String> = EXPECTED_REEXPORTS
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    expected_reexports.sort();
    assert_eq!(public_reexports(lib), expected_reexports);
}

#[test]
fn eval_metric_scoring_contracts_are_stable() {
    let exact = task("exact", "approved", EvalMetric::ExactMatch);
    assert_eq!(exact.score("approved"), 1.0);
    assert_eq!(exact.score("Approved"), 0.0);

    let contains = task(
        "contains",
        "ignored",
        EvalMetric::SubstringContains {
            must_contain: vec!["tenant".into(), "isolated".into()],
        },
    );
    assert_eq!(contains.score("tenant rows stay isolated"), 1.0);
    assert_eq!(contains.score("tenant rows leak"), 0.0);

    let jaccard = task("jaccard", "alpha gamma", EvalMetric::JaccardTokens);
    assert!((jaccard.score("alpha beta") - (1.0 / 3.0)).abs() < f64::EPSILON);
    assert_eq!(jaccard.score("delta epsilon"), 0.0);
}

#[tokio::test]
async fn run_eval_records_subject_successes_and_errors() {
    let tasks = vec![
        task("pass-a", "one", EvalMetric::ExactMatch),
        task("pass-b", "two", EvalMetric::ExactMatch),
        task("error", "three", EvalMetric::ExactMatch),
    ];
    let (records, summary) = run_eval(&ScriptedSubject, &tasks).await;

    assert_eq!(records.len(), 3);
    assert_eq!(records[0].output, "one");
    assert_eq!(records[2].task, "error");
    assert_eq!(records[2].output, "");
    assert!(!records[2].passed);

    assert_eq!(summary.n, 3);
    assert_eq!(summary.errors, 1);
    assert!((summary.mean_score - (2.0 / 3.0)).abs() < f64::EPSILON);
    assert!((summary.pass_rate - (2.0 / 3.0)).abs() < f64::EPSILON);
}

#[test]
fn regression_gate_allows_stable_and_blocks_real_drops() {
    let stable = detect_eval_regression(
        &run_report("base", 95, 100, 0.95),
        &run_report("candidate", 94, 100, 0.94),
        EvalRegressionConfig::default(),
    );
    assert_eq!(stable.decision, EvalRegressionDecision::Stable);
    assert!(ensure_promotion_allowed(&stable, false).is_ok());

    let regressed = detect_eval_regression(
        &run_report("base", 95, 100, 0.95),
        &run_report("candidate", 70, 100, 0.70),
        EvalRegressionConfig::default(),
    );
    assert_eq!(regressed.decision, EvalRegressionDecision::Regressed);
    assert!(regressed.blocks_promotion());
    assert!(ensure_promotion_allowed(&regressed, false)
        .expect_err("regression should require an override")
        .contains("eval_regression"));
    assert!(ensure_promotion_allowed(&regressed, true).is_ok());
}

#[test]
fn honesty_verdict_serde_shape_is_stable() {
    let verdict = HonestyResult::Suspicious {
        self_reported: 1.0,
        observed: 0.0,
        delta: 1.0,
        sample_size: 1,
        breakdown: vec![SampleVerdict {
            session_id: SessionId::new(),
            self_reported_success: true,
            judge_observed_success: false,
            disagreement: true,
        }],
    };
    let json = serde_json::to_value(&verdict).expect("honesty verdict serializes");

    assert_eq!(json["kind"], "suspicious");
    assert_eq!(json["self_reported"], 1.0);
    assert_eq!(json["observed"], 0.0);
    assert_eq!(json["delta"], 1.0);
    assert_eq!(json["sample_size"], 1);
    assert_eq!(json["breakdown"][0]["self_reported_success"], true);
    assert_eq!(json["breakdown"][0]["judge_observed_success"], false);
    assert_eq!(json["breakdown"][0]["disagreement"], true);
}

#[test]
fn streaming_line_formats_are_stable() {
    assert_eq!(
        format_simple_line(3, 12, "verify_price", true, Some(172), Some(0.0021)),
        "[3/12] verify_price: PASS (172ms, $0.0021)"
    );

    let verbose: Value = serde_json::from_str(&format_verbose_line(
        2,
        5,
        "fixture_b",
        false,
        json!({"score": 0.25, "id": "spoof"}),
    ))
    .expect("verbose line is valid JSON");
    assert_eq!(verbose["type"], "fixture");
    assert_eq!(verbose["idx"], 2);
    assert_eq!(verbose["total"], 5);
    assert_eq!(verbose["id"], "fixture_b");
    assert_eq!(verbose["passed"], false);
    assert_eq!(verbose["score"], 0.25);

    assert_eq!(
        format_summary_line(StreamMode::Simple, 4, 1, Some(1234)),
        "summary: 4/5 passed in 1234ms"
    );
    let summary: Value =
        serde_json::from_str(&format_summary_line(StreamMode::Verbose, 4, 1, Some(1234)))
            .expect("summary line is valid JSON");
    assert_eq!(summary["type"], "summary");
    assert_eq!(summary["passed"], 4);
    assert_eq!(summary["failed"], 1);
    assert_eq!(summary["total_ms"], 1234);
    assert_eq!(format_summary_line(StreamMode::Off, 4, 1, Some(1234)), "");
}

#[test]
fn self_healing_requires_environmental_failure_and_operator_approval() {
    let original = task("ui/approve-button", "Approve", EvalMetric::ExactMatch);
    let record = EvalRecord {
        task: original.name.clone(),
        score: 0.0,
        passed: false,
        output: "Approve request".into(),
    };
    let proposal = propose_fixture_update(
        &original,
        &record,
        &FixtureFailureJudgment::environmental("button label changed", "Approve request", 0.91),
        SelfHealingConfig::default(),
    )
    .expect("confident environmental failure proposes a fixture update");
    assert_eq!(proposal.before.expected, "Approve");
    assert_eq!(proposal.after.expected, "Approve request");

    let mut rejected = vec![original.clone()];
    let receipt = apply_fixture_update(
        &mut rejected,
        &proposal,
        OperatorFixtureApproval::Rejected {
            reason: Some("not an environment change".into()),
        },
    )
    .expect("rejected updates are not errors");
    assert!(receipt.is_none());
    assert_eq!(rejected[0].expected, "Approve");

    let mut approved = vec![original];
    let receipt = apply_fixture_update(
        &mut approved,
        &proposal,
        OperatorFixtureApproval::Approved {
            approver: "operator:mark".into(),
        },
    )
    .expect("approved update applies")
    .expect("approved update returns a receipt");
    assert_eq!(approved[0].expected, "Approve request");
    assert_eq!(receipt.approver, "operator:mark");
}

struct ScriptedSubject;

#[async_trait]
impl EvalSubject for ScriptedSubject {
    async fn run(&self, task: &EvalTask) -> ArgoResult<String> {
        if task.name == "error" {
            Err(ArgoError::Provider("synthetic subject error".into()))
        } else {
            Ok(task.expected.clone())
        }
    }
}

fn task(name: &str, expected: &str, metric: EvalMetric) -> EvalTask {
    EvalTask {
        name: name.into(),
        input: format!("input for {name}"),
        expected: expected.into(),
        metric,
    }
}

fn run_report(id: &str, pass_until: usize, total: usize, score: f64) -> EvalRunReport {
    EvalRunReport {
        id: id.into(),
        vertical: "support".into(),
        created_at: "2026-05-24T00:00:00Z".into(),
        fixtures: (0..total)
            .map(|i| EvalRunFixture {
                fixture: format!("fixture-{i:03}"),
                skill: "answer-ticket".into(),
                metric: "exact".into(),
                passed: i < pass_until,
                score,
            })
            .collect(),
    }
}

fn public_modules(lib: &str) -> Vec<&str> {
    lib.lines()
        .filter_map(|line| line.trim().strip_prefix("pub mod "))
        .map(|rest| rest.trim_end_matches(';'))
        .collect()
}

fn public_reexports(lib: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut block = String::new();
    for line in lib.lines().map(str::trim) {
        if line.starts_with("pub use ") || !block.is_empty() {
            block.push_str(line);
            block.push(' ');
            if line.ends_with(';') {
                names.extend(reexport_names(&block));
                block.clear();
            }
        }
    }
    names.sort();
    names
}

fn reexport_names(block: &str) -> Vec<String> {
    let rest = block
        .trim()
        .strip_prefix("pub use ")
        .expect("called only for pub use blocks")
        .trim_end_matches(';')
        .trim();
    let inner = rest
        .split_once('{')
        .and_then(|(_, tail)| tail.rsplit_once('}').map(|(names, _)| names))
        .unwrap_or(rest);
    inner
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| name.rsplit("::").next().unwrap_or(name).to_string())
        .collect()
}
