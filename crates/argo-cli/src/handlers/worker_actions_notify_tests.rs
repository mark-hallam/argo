use super::*;
use crate::handlers::worker_actions::ActionRecord;
use argo_identity::NotifyChannelRef;
use chrono::Utc;
use std::sync::Mutex;

#[path = "worker_actions_notify_tests_env_mocks.rs"]
mod env_mocks;
#[path = "worker_actions_notify_tests_env_channels.rs"]
mod env_channels;

#[derive(Default)]
struct StubSender {
    calls: Mutex<Vec<NotifyChannelRef>>,
    fail_for: Vec<String>,
}

impl StubSender {
    fn new() -> Self {
        Self::default()
    }

    fn fail_for(channel: &str) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            fail_for: vec![channel.to_string()],
        }
    }

    fn calls(&self) -> Vec<NotifyChannelRef> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl NotifySender for StubSender {
    async fn send_via(&self, ref_: &NotifyChannelRef, _n: &Notification) -> Result<(), String> {
        self.calls.lock().unwrap().push(ref_.clone());
        if self.fail_for.iter().any(|f| f == &ref_.channel) {
            Err(format!("stubbed failure for {}", ref_.channel))
        } else {
            Ok(())
        }
    }
}

pub(super) fn action(action_id: &str, risk: &str) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: action_id.to_string(),
        job_id: "job-1".to_string(),
        worker_id: argo_identity::WorkerId::nil(),
        item_id: None,
        action_type: "send_buyer_followup_email".to_string(),
        risk_level: risk.to_string(),
        requires_approval: true,
        state: "pending".to_string(),
        payload: serde_json::json!({}),
        approved_by: None,
        approved_at: None,
        executed_at: None,
        platform: None,
        result: None,
        evidence: Vec::new(),
        blocked_reason: None,
        created_at: Utc::now().to_rfc3339(),
    }
}

/// `send_with_rate_limit_retry` honours
/// `ChannelError::RateLimited` by sleeping for `retry_after`
/// (capped at the helper's max backoff) before its second
/// attempt, then succeeding. Uses a short real sleep rather
/// than `tokio::time::pause` to match
/// `mcp_rate_limit_tests.rs` precedent.
#[tokio::test]
async fn rate_limit_retry_waits_then_succeeds() {
    use argo_channels::{
        ChannelError, ChannelId, ChannelResult, DeliveryReceipt, DeliveryStatus, InboundEvent,
        NotifyChannel,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    struct FlakyOnce {
        calls: AtomicUsize,
    }
    #[async_trait::async_trait]
    impl NotifyChannel for FlakyOnce {
        fn id(&self) -> ChannelId {
            ChannelId::new("flaky")
        }
        async fn send(&self, notification: Notification) -> ChannelResult<DeliveryReceipt> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            if n == 0 {
                Err(ChannelError::RateLimited {
                    retry_after: Duration::from_millis(20),
                })
            } else {
                Ok(DeliveryReceipt {
                    channel_id: ChannelId::new("flaky"),
                    notification_id: notification.approval_id,
                    provider_message_id: None,
                    status: DeliveryStatus::Sent,
                    deferred_reason: None,
                    delivered_at: Utc::now(),
                })
            }
        }
        async fn poll(&self) -> ChannelResult<Vec<InboundEvent>> {
            Ok(Vec::new())
        }
    }

    let channel = FlakyOnce {
        calls: AtomicUsize::new(0),
    };
    let n = build_notification(&action("act-rate", "high"));
    let started = std::time::Instant::now();
    send_with_rate_limit_retry("flaky", &channel, &n)
        .await
        .expect("retry must succeed on attempt 2");
    let elapsed = started.elapsed();
    assert_eq!(channel.calls.load(Ordering::SeqCst), 2);
    assert!(
        elapsed >= Duration::from_millis(15),
        "must sleep at least retry_after; elapsed={elapsed:?}",
    );
}

#[tokio::test]
async fn rate_limit_retry_gives_up_after_bound() {
    use argo_channels::{
        ChannelError, ChannelId, ChannelResult, DeliveryReceipt, InboundEvent, NotifyChannel,
    };
    use std::time::Duration;

    struct AlwaysLimited;
    #[async_trait::async_trait]
    impl NotifyChannel for AlwaysLimited {
        fn id(&self) -> ChannelId {
            ChannelId::new("always-limited")
        }
        async fn send(&self, _: Notification) -> ChannelResult<DeliveryReceipt> {
            Err(ChannelError::RateLimited {
                retry_after: Duration::from_millis(5),
            })
        }
        async fn poll(&self) -> ChannelResult<Vec<InboundEvent>> {
            Ok(Vec::new())
        }
    }
    let n = build_notification(&action("act-rate-bound", "high"));
    let err = send_with_rate_limit_retry("always-limited", &AlwaysLimited, &n)
        .await
        .expect_err("after bounded retries the limiter surfaces an error");
    assert!(err.contains("rate-limited"));
}

#[tokio::test]
async fn no_channels_is_no_op() {
    let cfg = NotifyConfig::default();
    let sender = StubSender::new();
    let results = dispatch(&cfg, &action("act-1", "high"), &sender).await;
    assert!(results.is_empty());
    assert!(sender.calls().is_empty());
}

#[tokio::test]
async fn single_channel_one_send() {
    let cfg = NotifyConfig {
        channels: vec![NotifyChannelRef::new("push", "operator-iphone")],
        ..Default::default()
    };
    let sender = StubSender::new();
    let results = dispatch(&cfg, &action("act-2", "medium"), &sender).await;
    assert_eq!(results.len(), 1);
    assert!(results[0].success);
    assert_eq!(results[0].channel, "push");
    assert_eq!(sender.calls().len(), 1);
}

#[tokio::test]
async fn multi_channel_fan_out() {
    let cfg = NotifyConfig {
        channels: vec![
            NotifyChannelRef::new("push", "iphone"),
            NotifyChannelRef::new("email", "ops@example.com"),
            NotifyChannelRef::new("slack", "C123"),
        ],
        ..Default::default()
    };
    let sender = StubSender::new();
    let results = dispatch(&cfg, &action("act-3", "low"), &sender).await;
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.success));
    let calls = sender.calls();
    assert_eq!(calls.len(), 3);
    assert!(calls.iter().any(|c| c.channel == "push"));
    assert!(calls.iter().any(|c| c.channel == "email"));
    assert!(calls.iter().any(|c| c.channel == "slack"));
}

#[tokio::test]
async fn one_channel_fails_others_still_fire() {
    let cfg = NotifyConfig {
        channels: vec![
            NotifyChannelRef::new("push", "iphone"),
            NotifyChannelRef::new("email", "ops@example.com"),
            NotifyChannelRef::new("slack", "C123"),
        ],
        ..Default::default()
    };
    let sender = StubSender::fail_for("email");
    let results = dispatch(&cfg, &action("act-4", "high"), &sender).await;
    assert_eq!(results.len(), 3);
    assert!(
        results
            .iter()
            .find(|r| r.channel == "push")
            .unwrap()
            .success
    );
    let email_result = results.iter().find(|r| r.channel == "email").unwrap();
    assert!(!email_result.success);
    assert!(email_result.error.is_some());
    assert!(
        results
            .iter()
            .find(|r| r.channel == "slack")
            .unwrap()
            .success
    );
}

#[tokio::test]
async fn risk_class_filter_drops_mismatched_channels() {
    let mut high_only = NotifyChannelRef::new("sms", "+15551234567");
    high_only.risk_level = Some("high".into());
    let mut low_only = NotifyChannelRef::new("email", "ops@example.com");
    low_only.risk_level = Some("low".into());
    let cfg = NotifyConfig {
        channels: vec![
            NotifyChannelRef::new("push", "iphone"),
            high_only.clone(),
            low_only,
        ],
        ..Default::default()
    };
    let sender = StubSender::new();
    let results = dispatch(&cfg, &action("act-5", "high"), &sender).await;
    assert_eq!(results.len(), 2, "low-only filter should drop email");
    assert!(results.iter().any(|r| r.channel == "push"));
    assert!(results.iter().any(|r| r.channel == "sms"));
    assert!(!results.iter().any(|r| r.channel == "email"));
}

#[tokio::test]
async fn risk_filter_is_case_insensitive() {
    let mut ch = NotifyChannelRef::new("push", "iphone");
    ch.risk_level = Some("HIGH".into());
    let cfg = NotifyConfig {
        channels: vec![ch],
        ..Default::default()
    };
    let sender = StubSender::new();
    let results = dispatch(&cfg, &action("act-6", "high"), &sender).await;
    assert_eq!(results.len(), 1);
    assert!(results[0].success);
}

#[test]
fn matches_risk_handles_none_filter() {
    let ch = NotifyChannelRef::new("push", "iphone");
    assert!(matches_risk(&ch, "low"));
    assert!(matches_risk(&ch, "medium"));
    assert!(matches_risk(&ch, "high"));
}

#[test]
fn build_notification_carries_action_metadata() {
    let act = action("xyz-789", "medium");
    let n = build_notification(&act);
    assert_eq!(n.action_id, "xyz-789");
    assert_eq!(n.approval_id, "xyz-789");
    assert_eq!(n.risk_level, RiskLevel::Medium);
    assert!(n.prompt_body.contains("xyz-789"));
    assert!(n.prompt_body.contains("send_buyer_followup_email"));
}

#[test]
fn notify_config_load_returns_default_when_missing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let cfg = NotifyConfig::load(tmp.path());
    assert!(cfg.channels.is_empty());
}

#[test]
fn notify_config_load_parses_channels() {
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(tmp.path().join(".skycot")).unwrap();
    let body = r#"
        [[channels]]
        channel = "push"
        target = "operator-iphone"

        [[channels]]
        channel = "email"
        target = "ops@example.com"
        risk_level = "high"
    "#;
    std::fs::write(tmp.path().join(".skycot/notify.toml"), body).unwrap();
    let cfg = NotifyConfig::load(tmp.path());
    assert_eq!(cfg.channels.len(), 2);
    assert_eq!(cfg.channels[0].channel, "push");
    assert_eq!(cfg.channels[0].target, "operator-iphone");
    assert!(cfg.channels[0].risk_level.is_none());
    assert_eq!(cfg.channels[1].channel, "email");
    assert_eq!(cfg.channels[1].risk_level.as_deref(), Some("high"));
}
