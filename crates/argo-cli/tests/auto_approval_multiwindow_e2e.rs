//! Multi-window auto-approval + channel fallback E2E.
//!
//! This extends `auto_approval_budget_e2e.rs` with the missing
//! scenario-4 coverage: per-action caps, daily caps, weekday business
//! windows, and a deterministic fallback chain when a human Ask is
//! required.

use argo_channels::fallback::{AttemptOutcome, DeliveryAttempt, DeliveryReport, FallbackChain};
use argo_channels::{Notification, OperatorTarget, RiskLevel};
use argo_identity::auto_approval::{
    dollars_to_micros, evaluate, AutoApprovalDecision, PendingAction, SpendSnapshot, TimeWindow,
};
use argo_identity::worker_budget::{record_charge, spend_today_for_worker};
use argo_identity::{AutoApprovalPolicy, NotifyChannelRef};
use argo_store::SessionStore;
use chrono::{DateTime, Duration as ChronoDuration, NaiveTime, TimeZone, Utc, Weekday};
use std::collections::HashMap;

const WORKER_ID: &str = "worker-multiwindow-e2e";

#[derive(Debug, Clone, Copy)]
struct Candidate {
    id: &'static str,
    estimated_usd: f64,
    fired_at: DateTime<Utc>,
}

fn business_policy() -> AutoApprovalPolicy {
    AutoApprovalPolicy {
        enabled: true,
        max_usd_micros_per_action: Some(dollars_to_micros(5.0)),
        daily_budget_usd_micros: Some(dollars_to_micros(50.0)),
        time_windows: vec![TimeWindow {
            days: vec![
                Weekday::Mon,
                Weekday::Tue,
                Weekday::Wed,
                Weekday::Thu,
                Weekday::Fri,
            ],
            start: NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(21, 0, 0).unwrap(),
        }],
        wake_action_types: Vec::new(),
    }
}

fn pending_for(candidate: &Candidate) -> PendingAction<'static> {
    PendingAction {
        risk_level: "low",
        requires_approval: true,
        estimated_usd_micros: Some(dollars_to_micros(candidate.estimated_usd)),
        action_type: "send_invoice",
    }
}

fn inside_window() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 13, 18, 0, 0).unwrap()
}

fn outside_window() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 16, 15, 0, 0).unwrap()
}

#[test]
fn per_action_ceiling_routes_to_manual_approval() {
    let candidate = Candidate {
        id: "act-per-action",
        estimated_usd: 6.0,
        fired_at: inside_window(),
    };
    let decision = evaluate(
        &business_policy(),
        &pending_for(&candidate),
        SpendSnapshot::zero(),
        candidate.fired_at,
    );
    assert!(matches!(decision, AutoApprovalDecision::Ask { .. }));
    assert!(
        decision.reason().contains("per_action_cap"),
        "unexpected reason: {}",
        decision.reason(),
    );
}

#[test]
fn daily_ceiling_blocks_after_prior_spend() {
    let home = tempfile::tempdir().expect("tmpdir for ARGO_HOME");
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async move {
        let store = SessionStore::open(home.path().join("sessions.db"))
            .await
            .expect("open SessionStore");
        store.migrate().await.expect("migrate SessionStore");
        let pool = store.pool();
        let now = inside_window();
        record_charge(pool, "seed-spend", WORKER_ID, dollars_to_micros(48.0), now)
            .await
            .expect("record seed charge");
        let today = spend_today_for_worker(pool, WORKER_ID, now)
            .await
            .expect("spend_today_for_worker");
        let candidate = Candidate {
            id: "act-daily-cap",
            estimated_usd: 3.0,
            fired_at: now,
        };
        let decision = evaluate(
            &business_policy(),
            &pending_for(&candidate),
            SpendSnapshot {
                today_usd_micros: today,
                this_month_usd_micros: today,
            },
            candidate.fired_at,
        );
        assert!(matches!(decision, AutoApprovalDecision::Block { .. }));
        assert!(
            decision.reason().contains("daily_cap"),
            "unexpected reason: {}",
            decision.reason(),
        );
    });
}

#[test]
fn weekday_business_hours_route_off_hours_to_manual() {
    let candidate = Candidate {
        id: "act-weekend",
        estimated_usd: 2.0,
        fired_at: outside_window(),
    };
    let decision = evaluate(
        &business_policy(),
        &pending_for(&candidate),
        SpendSnapshot::zero(),
        candidate.fired_at,
    );
    assert!(matches!(decision, AutoApprovalDecision::Ask { .. }));
    assert_eq!(decision.reason(), "policy:outside_time_window");
}

#[test]
fn manual_approval_ask_walks_slack_to_sms_fallback() {
    let candidate = Candidate {
        id: "act-fallback",
        estimated_usd: 6.0,
        fired_at: inside_window(),
    };
    let decision = evaluate(
        &business_policy(),
        &pending_for(&candidate),
        SpendSnapshot::zero(),
        candidate.fired_at,
    );
    assert!(matches!(decision, AutoApprovalDecision::Ask { .. }));

    let report = dispatch_mock_fallback(
        &notification(candidate.id),
        &FallbackChain::from_order(["slack", "sms"]),
        &[
            NotifyChannelRef::new("slack", "C123"),
            NotifyChannelRef::new("sms", "+15551234567"),
        ],
        HashMap::from([("slack", Err("transport: 503")), ("sms", Ok(()))]),
    );
    assert!(report.ok());
    assert_eq!(report.succeeded_via.as_deref(), Some("sms"));
    assert_eq!(report.attempts.len(), 2);
    assert!(matches!(
        report.attempts[0].outcome,
        AttemptOutcome::Failure { .. }
    ));
    assert!(matches!(
        report.attempts[1].outcome,
        AttemptOutcome::Success
    ));
}

fn notification(action_id: &str) -> Notification {
    Notification {
        target: OperatorTarget {
            operator_id: "operator".into(),
            email: "ops@example.com".into(),
        },
        action_id: action_id.into(),
        approval_id: action_id.into(),
        risk_level: RiskLevel::Medium,
        prompt_body: format!("Please approve {action_id}"),
        approve_url: format!("argo://approve/{action_id}"),
        reject_url: format!("argo://reject/{action_id}"),
        expiry: Utc::now() + ChronoDuration::minutes(30),
    }
}

fn dispatch_mock_fallback(
    notification: &Notification,
    chain: &FallbackChain,
    channel_refs: &[NotifyChannelRef],
    scripted: HashMap<&str, Result<(), &str>>,
) -> DeliveryReport {
    let started_at = Utc::now();
    let mut attempts = Vec::new();
    let mut succeeded_via = None;
    for kind in &chain.order {
        let outcome = if channel_refs.iter().any(|r| r.channel == *kind) {
            match scripted.get(kind.as_str()).copied().unwrap_or(Ok(())) {
                Ok(()) => AttemptOutcome::Success,
                Err(error) => AttemptOutcome::Failure {
                    error: error.to_string(),
                },
            }
        } else {
            AttemptOutcome::Unconfigured
        };
        attempts.push(DeliveryAttempt {
            channel: kind.clone(),
            started_at: Utc::now(),
            outcome,
        });
        if attempts.last().is_some_and(|a| a.outcome.is_terminal()) {
            succeeded_via = Some(kind.clone());
            break;
        }
    }
    DeliveryReport {
        action_id: notification.action_id.clone(),
        started_at,
        finished_at: Utc::now(),
        succeeded_via,
        attempts,
    }
}
