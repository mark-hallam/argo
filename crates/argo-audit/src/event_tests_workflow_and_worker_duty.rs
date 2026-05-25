//! Workflow, journey, and worker-duty audit-kind tests for
//! [`crate::event`]. Included via `#[path]` from `event.rs`.
use super::AuditKind;

#[test]
fn workflow_step_executed_and_failed_round_trip_and_listed() {
    for (label, kind) in [
        ("workflow.step.executed", AuditKind::WorkflowStepExecuted),
        ("workflow.step.failed", AuditKind::WorkflowStepFailed),
    ] {
        assert_eq!(AuditKind::from_label(label), Some(kind));
        assert_eq!(kind.label(), label);
    }
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"workflow.step.executed"));
    assert!(labels.contains(&"workflow.step.failed"));
}

#[test]
fn worker_duty_kinds_round_trip_and_listed() {
    for (label, kind) in [
        ("worker_duty.fired", AuditKind::WorkerDutyFired),
        ("worker_duty.failed", AuditKind::WorkerDutyFailed),
    ] {
        assert_eq!(AuditKind::from_label(label), Some(kind));
        assert_eq!(kind.label(), label);
    }
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"worker_duty.fired"));
    assert!(labels.contains(&"worker_duty.failed"));
}

#[test]
fn workflow_definition_kinds_round_trip_and_listed() {
    for (label, kind) in [
        (
            "workflow_definition_created",
            AuditKind::WorkflowDefinitionCreated,
        ),
        ("workflow_head_changed", AuditKind::WorkflowHeadChanged),
        (
            "workflow_version_archived",
            AuditKind::WorkflowVersionArchived,
        ),
        ("workflow_rolled_back", AuditKind::WorkflowRolledBack),
    ] {
        assert_eq!(AuditKind::from_label(label), Some(kind));
        assert_eq!(kind.label(), label);
        assert_eq!(
            serde_json::to_string(&kind).unwrap(),
            format!("\"{label}\"")
        );
    }
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"workflow_definition_created"));
    assert!(labels.contains(&"workflow_head_changed"));
    assert!(labels.contains(&"workflow_version_archived"));
    assert!(labels.contains(&"workflow_rolled_back"));
}

#[test]
fn workflow_step_kinds_round_trip_and_listed() {
    for (label, kind) in [
        ("workflow.step.started", AuditKind::WorkflowStepStarted),
        ("workflow.step.completed", AuditKind::WorkflowStepCompleted),
    ] {
        assert_eq!(AuditKind::from_label(label), Some(kind));
        assert_eq!(kind.label(), label);
    }
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"workflow.step.started"));
    assert!(labels.contains(&"workflow.step.completed"));
}

#[test]
fn journey_event_kinds_round_trip_and_listed() {
    // Stream `web-audit-kinds-for-journey-events` — operator audit
    // feed promotes worker-hire / vertical-run / cron rows from the
    // generic `HttpRequest` bucket to named domain events. Mixed
    // label style (dot for vertical/worker, underscore for cron)
    // mirrors the existing precedent — `workflow.step.started`
    // uses dots; `workflow_definition_created` uses underscores.
    let kinds = [
        ("worker.hired", AuditKind::WorkerHired),
        ("vertical.run.requested", AuditKind::VerticalRunRequested),
        ("vertical.run.completed", AuditKind::VerticalRunCompleted),
        ("cron_job_created", AuditKind::CronJobCreated),
        ("cron_job_deleted", AuditKind::CronJobDeleted),
        ("cron_job_fired", AuditKind::CronJobFired),
    ];
    for (label, kind) in kinds {
        assert_eq!(AuditKind::from_label(label), Some(kind));
        assert_eq!(kind.label(), label);
    }
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    for (label, _) in kinds {
        assert!(labels.contains(&label), "missing {label}: {labels:?}");
    }
}

#[test]
fn workflow_test_run_persisted_round_trips_and_listed() {
    // Stream `claude-followup-pr-2021-test-run-gateway-persist` —
    // re-author of closed PR #2021. The builder's "Test run" overlay
    // POSTs each completed run to the gateway, which emits this row
    // alongside the existing `HttpRequest` so audit consumers can
    // filter on a first-class kind.
    let kind = AuditKind::WorkflowTestRunPersisted;
    assert_eq!(
        AuditKind::from_label("workflow_test_run.persisted"),
        Some(kind)
    );
    assert_eq!(kind.label(), "workflow_test_run.persisted");
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"workflow_test_run.persisted"));
    assert_eq!(
        serde_json::to_string(&kind).unwrap(),
        "\"workflow_test_run_persisted\""
    );
}

#[test]
fn workflow_execute_dispatched_round_trips_and_listed() {
    let kind = AuditKind::WorkflowExecuteDispatched;
    assert_eq!(
        AuditKind::from_label("workflow.execute.dispatched"),
        Some(kind)
    );
    assert_eq!(kind.label(), "workflow.execute.dispatched");
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"workflow.execute.dispatched"));
    assert_eq!(
        serde_json::to_string(&kind).unwrap(),
        "\"workflow_execute_dispatched\""
    );
}
