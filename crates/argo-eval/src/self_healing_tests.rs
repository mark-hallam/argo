use crate::{
    apply_fixture_update, propose_fixture_update, EvalMetric, EvalRecord, EvalTask,
    FixtureFailureJudgment, OperatorFixtureApproval, SelfHealingConfig,
};

fn task() -> EvalTask {
    EvalTask {
        name: "ui/approve-button".into(),
        input: "find the approval button".into(),
        expected: "Approve".into(),
        metric: EvalMetric::ExactMatch,
    }
}

fn failed_record() -> EvalRecord {
    EvalRecord {
        task: "ui/approve-button".into(),
        score: 0.0,
        passed: false,
        output: "Approve request".into(),
    }
}

#[test]
fn environmental_failure_produces_fixture_update_proposal() {
    let proposal = propose_fixture_update(
        &task(),
        &failed_record(),
        &FixtureFailureJudgment::environmental(
            "the product renamed the button",
            "Approve request",
            0.92,
        ),
        SelfHealingConfig::default(),
    )
    .unwrap();

    assert_eq!(proposal.fixture, "ui/approve-button");
    assert_eq!(proposal.before.expected, "Approve");
    assert_eq!(proposal.after.expected, "Approve request");
}

#[test]
fn operator_approval_applies_update() {
    let proposal = propose_fixture_update(
        &task(),
        &failed_record(),
        &FixtureFailureJudgment::environmental("renamed", "Approve request", 0.95),
        SelfHealingConfig::default(),
    )
    .unwrap();
    let mut tasks = vec![task()];
    let receipt = apply_fixture_update(
        &mut tasks,
        &proposal,
        OperatorFixtureApproval::Approved {
            approver: "operator:mark".into(),
        },
    )
    .unwrap()
    .unwrap();

    assert_eq!(tasks[0].expected, "Approve request");
    assert_eq!(receipt.approver, "operator:mark");
    assert_eq!(receipt.before.expected, "Approve");
}

#[test]
fn logical_failure_does_not_self_heal() {
    let proposal = propose_fixture_update(
        &task(),
        &failed_record(),
        &FixtureFailureJudgment::logical("model ignored instructions", 0.99),
        SelfHealingConfig::default(),
    );

    assert!(proposal.is_none());
}

#[test]
fn low_confidence_environmental_failure_does_not_self_heal() {
    let proposal = propose_fixture_update(
        &task(),
        &failed_record(),
        &FixtureFailureJudgment::environmental("maybe renamed", "Approve request", 0.40),
        SelfHealingConfig::default(),
    );

    assert!(proposal.is_none());
}

#[test]
fn passed_fixture_does_not_self_heal() {
    let mut record = failed_record();
    record.passed = true;
    record.score = 1.0;
    let proposal = propose_fixture_update(
        &task(),
        &record,
        &FixtureFailureJudgment::environmental("renamed", "Approve request", 0.95),
        SelfHealingConfig::default(),
    );

    assert!(proposal.is_none());
}

#[test]
fn substring_metric_updates_required_terms() {
    let task = EvalTask {
        name: "api/status-field".into(),
        input: "read status".into(),
        expected: "status: pending".into(),
        metric: EvalMetric::SubstringContains {
            must_contain: vec!["pending".into()],
        },
    };
    let record = EvalRecord {
        task: "api/status-field".into(),
        score: 0.0,
        passed: false,
        output: "status: awaiting_review".into(),
    };
    let proposal = propose_fixture_update(
        &task,
        &record,
        &FixtureFailureJudgment::environmental("field renamed", "awaiting_review", 0.90),
        SelfHealingConfig::default(),
    )
    .unwrap();

    assert_eq!(proposal.after.expected, "awaiting_review");
    assert_eq!(
        proposal.after.metric,
        EvalMetric::SubstringContains {
            must_contain: vec!["awaiting_review".into()]
        }
    );
}

#[test]
fn rejected_update_leaves_tasks_unchanged() {
    let proposal = propose_fixture_update(
        &task(),
        &failed_record(),
        &FixtureFailureJudgment::environmental("renamed", "Approve request", 0.95),
        SelfHealingConfig::default(),
    )
    .unwrap();
    let mut tasks = vec![task()];
    let receipt = apply_fixture_update(
        &mut tasks,
        &proposal,
        OperatorFixtureApproval::Rejected {
            reason: Some("bad suggestion".into()),
        },
    )
    .unwrap();

    assert!(receipt.is_none());
    assert_eq!(tasks[0].expected, "Approve");
}
