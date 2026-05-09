use super::*;
use crate::handlers::worker_actions_notify::NotifySender;
use argo_channels::fallback::{
    read_recent_reports, AttemptOutcome, DeliveryReport, FallbackChain,
};
use argo_channels::{Notification, OperatorTarget, RiskLevel};
use argo_identity::NotifyChannelRef;
use chrono::{Duration as ChronoDuration, Utc};
use std::collections::HashMap;
use std::sync::Mutex;
use tempfile::tempdir;

/// Scripted per-channel outcome for [`MockChannel`].
#[derive(Debug, Clone)]
enum Scripted {
    Ok,
    Err(String),
    /// Sleep `secs` seconds before returning Ok — used to exercise
    /// the per-step timeout. The chain wraps each send in
    /// `tokio::time::timeout`; with the mock sleeping past the
    /// chain's `per_step_timeout_secs`, the outcome must be
    /// `AttemptOutcome::Timeout`.
    Hang {
        secs: u64,
    },
}

#[derive(Default)]
struct MockChannel {
    script: HashMap<String, Scripted>,
    calls: Mutex<Vec<String>>,
}

impl MockChannel {
    fn new() -> Self {
        Self::default()
    }

    fn with(mut self, kind: &str, outcome: Scripted) -> Self {
        self.script.insert(kind.to_string(), outcome);
        self
    }

    fn call_log(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl NotifySender for MockChannel {
    async fn send_via(
        &self,
        ref_: &NotifyChannelRef,
        _notification: &Notification,
    ) -> Result<(), String> {
        self.calls.lock().unwrap().push(ref_.channel.clone());
        let scripted = self
            .script
            .get(&ref_.channel)
            .cloned()
            .unwrap_or(Scripted::Ok);
        match scripted {
            Scripted::Ok => Ok(()),
            Scripted::Err(e) => Err(e),
            Scripted::Hang { secs } => {
                tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
                Ok(())
            }
        }
    }
}

fn notification(action: &str) -> Notification {
    Notification {
        target: OperatorTarget {
            operator_id: "operator".into(),
            email: "ops@example.com".into(),
        },
        action_id: action.into(),
        approval_id: action.into(),
        risk_level: RiskLevel::High,
        prompt_body: format!("Please approve {action}"),
        approve_url: format!("argo://approve/{action}"),
        reject_url: format!("argo://reject/{action}"),
        expiry: Utc::now() + ChronoDuration::hours(1),
    }
}

fn refs() -> Vec<NotifyChannelRef> {
    vec![
        NotifyChannelRef::new("slack", "C123"),
        NotifyChannelRef::new("email", "ops@example.com"),
        NotifyChannelRef::new("webinbox", "operator-1"),
    ]
}

fn chain(order: &[&str], timeout_secs: u64) -> FallbackChain {
    FallbackChain {
        order: order.iter().map(|s| (*s).to_string()).collect(),
        per_step_timeout_secs: timeout_secs,
    }
}

#[tokio::test]
async fn first_success_short_circuits() {
    let mock = MockChannel::new()
        .with("slack", Scripted::Ok)
        .with("email", Scripted::Ok)
        .with("webinbox", Scripted::Ok);
    let report = dispatch_with_fallback(
        &notification("act-1"),
        &chain(&["slack", "email", "webinbox"], 5),
        &refs(),
        &mock,
    )
    .await;
    assert_eq!(report.attempts.len(), 1, "subsequent channels not tried");
    assert_eq!(report.attempts[0].channel, "slack");
    assert!(matches!(report.attempts[0].outcome, AttemptOutcome::Success));
    assert_eq!(report.succeeded_via.as_deref(), Some("slack"));
    assert!(report.ok());
    assert_eq!(mock.call_log(), vec!["slack"]);
}

#[tokio::test]
async fn slack_fails_email_succeeds() {
    let mock = MockChannel::new()
        .with("slack", Scripted::Err("transport: 503".into()))
        .with("email", Scripted::Ok);
    let report = dispatch_with_fallback(
        &notification("act-2"),
        &chain(&["slack", "email", "webinbox"], 5),
        &refs(),
        &mock,
    )
    .await;
    assert_eq!(report.attempts.len(), 2);
    assert_eq!(report.attempts[0].channel, "slack");
    match &report.attempts[0].outcome {
        AttemptOutcome::Failure { error } => assert!(error.contains("transport")),
        other => panic!("expected Failure, got {other:?}"),
    }
    assert!(matches!(report.attempts[1].outcome, AttemptOutcome::Success));
    assert_eq!(report.succeeded_via.as_deref(), Some("email"));
    assert_eq!(mock.call_log(), vec!["slack", "email"]);
}

#[tokio::test]
async fn slack_and_email_fail_webinbox_floor_succeeds() {
    let mock = MockChannel::new()
        .with("slack", Scripted::Err("transport: 401".into()))
        .with("email", Scripted::Err("rate-limited; retry-after=30".into()))
        .with("webinbox", Scripted::Ok);
    let report = dispatch_with_fallback(
        &notification("act-3"),
        &chain(&["slack", "email", "webinbox"], 5),
        &refs(),
        &mock,
    )
    .await;
    assert_eq!(report.attempts.len(), 3);
    assert!(report.ok());
    assert_eq!(report.succeeded_via.as_deref(), Some("webinbox"));
    // Rate-limit error string passes through verbatim — caller can
    // sniff retry-after if needed.
    if let AttemptOutcome::Failure { error } = &report.attempts[1].outcome {
        assert!(error.contains("retry-after"));
    } else {
        panic!("expected email Failure");
    }
}

#[tokio::test]
async fn all_fail_returns_no_success() {
    let mock = MockChannel::new()
        .with("slack", Scripted::Err("e1".into()))
        .with("email", Scripted::Err("e2".into()))
        .with("webinbox", Scripted::Err("e3".into()));
    let report = dispatch_with_fallback(
        &notification("act-4"),
        &chain(&["slack", "email", "webinbox"], 5),
        &refs(),
        &mock,
    )
    .await;
    assert_eq!(report.attempts.len(), 3);
    assert!(report.succeeded_via.is_none());
    assert!(!report.ok());
}

#[tokio::test]
async fn per_step_timeout_counts_as_failure_and_falls_through() {
    let mock = MockChannel::new()
        // Slack hangs longer than the chain's per-step timeout, so
        // the chain must record a Timeout and walk to email.
        // Real-time wait is bounded by the 1s chain timeout below.
        .with("slack", Scripted::Hang { secs: 30 })
        .with("email", Scripted::Ok);
    let report = dispatch_with_fallback(
        &notification("act-5"),
        &chain(&["slack", "email"], 1),
        &refs(),
        &mock,
    )
    .await;
    assert_eq!(report.attempts.len(), 2);
    assert!(matches!(report.attempts[0].outcome, AttemptOutcome::Timeout));
    assert!(matches!(report.attempts[1].outcome, AttemptOutcome::Success));
    assert_eq!(report.succeeded_via.as_deref(), Some("email"));
}

#[tokio::test]
async fn unconfigured_channel_does_not_terminate_chain() {
    let mock = MockChannel::new()
        // Only `email` is in `refs()`-without-slack; the chain still
        // walks to `email` after recording slack as Unconfigured.
        .with("email", Scripted::Ok);
    let only_email = vec![NotifyChannelRef::new("email", "ops@example.com")];
    let report = dispatch_with_fallback(
        &notification("act-6"),
        &chain(&["slack", "email"], 5),
        &only_email,
        &mock,
    )
    .await;
    assert_eq!(report.attempts.len(), 2);
    assert!(matches!(
        report.attempts[0].outcome,
        AttemptOutcome::Unconfigured
    ));
    assert_eq!(report.succeeded_via.as_deref(), Some("email"));
    // Sender was never asked to send via slack (no ref to send to).
    assert_eq!(mock.call_log(), vec!["email"]);
}

#[tokio::test]
async fn empty_refs_records_unconfigured_for_every_step() {
    let mock = MockChannel::new();
    let report = dispatch_with_fallback(
        &notification("act-7"),
        &chain(&["slack", "email", "webinbox"], 5),
        &[],
        &mock,
    )
    .await;
    assert_eq!(report.attempts.len(), 3);
    assert!(report
        .attempts
        .iter()
        .all(|a| matches!(a.outcome, AttemptOutcome::Unconfigured)));
    assert!(report.succeeded_via.is_none());
    assert!(mock.call_log().is_empty());
}

#[tokio::test]
async fn first_matching_ref_wins_when_kind_appears_twice() {
    let mock = MockChannel::new().with("email", Scripted::Ok);
    let dup_refs = vec![
        NotifyChannelRef::new("email", "first@example.com"),
        NotifyChannelRef::new("email", "second@example.com"),
    ];
    let report = dispatch_with_fallback(
        &notification("act-8"),
        &chain(&["email"], 5),
        &dup_refs,
        &mock,
    )
    .await;
    assert_eq!(report.attempts.len(), 1);
    assert!(report.ok());
    // Only one call - first ref wins.
    assert_eq!(mock.call_log().len(), 1);
}

#[tokio::test]
async fn persist_report_round_trips_through_jsonl() {
    let tmp = tempdir().expect("tempdir");
    let report = DeliveryReport {
        action_id: "act-9".into(),
        started_at: Utc::now(),
        finished_at: Utc::now(),
        succeeded_via: Some("webinbox".into()),
        attempts: vec![],
    };
    persist_report(tmp.path(), &report);
    let recent = read_recent_reports(&argo_channels::fallback::reports_path(tmp.path()), 5);
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].action_id, "act-9");
}
