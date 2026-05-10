//! Trait-level integration tests for the channel-wide quiet-hours lift.
//!
//! These tests verify two invariants:
//! 1. The lifted `QuietHours` type behaves identically to the original
//!    that lived in `email.rs` (overnight-window arithmetic).
//! 2. Every non-email channel (slack, sms, voice, whatsapp, imessage,
//!    push, webinbox) defers send when its `quiet_hours()` window
//!    matches `now`, with the byte-exact `"quiet_hours"` deferred
//!    reason that operators grep across telemetry.

#![cfg(test)]

use crate::imessage::{IMessageChannel, IMessageChannelConfig};
use crate::push::{PushApprovalChannel, PushApprovalChannelConfig};
use crate::slack::{
    SlackApprovalChannel, SlackApprovalChannelConfig, SlackOperatorRoute,
};
use crate::sms::{SmsApprovalChannel, SmsApprovalChannelConfig};
use crate::voice::{VoiceApprovalChannel, VoiceApprovalChannelConfig};
use crate::webinbox::{WebInboxChannel, WebInboxChannelConfig};
use crate::whatsapp::{WhatsAppApprovalChannel, WhatsAppApprovalChannelConfig};
use crate::{
    ChannelId, DeliveryStatus, Notification, NotifyChannel, OperatorTarget, QuietHours, RateLimitConfig,
    RiskLevel,
};
use chrono::{Duration, TimeZone, Utc};
use std::collections::BTreeMap;

/// Always-on quiet-hours window covering the entire day so the check
/// fires regardless of when the test runs.
fn always_quiet() -> QuietHours {
    QuietHours {
        enabled: true,
        start: "00:00".into(),
        end: "23:59".into(),
        timezone_offset_minutes: 0,
        days: Vec::new(),
    }
}

fn notification(operator_id: &str, risk: RiskLevel) -> Notification {
    Notification {
        target: OperatorTarget {
            operator_id: operator_id.into(),
            email: format!("{operator_id}@example.com"),
        },
        action_id: "act-1".into(),
        approval_id: "appr-1".into(),
        risk_level: risk,
        prompt_body: "Quiet-hours defer test".into(),
        approve_url: "https://gw/decision?token=appr-1.approve.999.sig".into(),
        reject_url: "https://gw/decision?token=appr-1.reject.999.sig".into(),
        expiry: Utc::now() + Duration::minutes(5),
    }
}

fn assert_quiet_hours_deferred(
    receipt: &crate::DeliveryReceipt,
    expected_channel: &str,
) {
    assert_eq!(
        receipt.status,
        DeliveryStatus::Deferred,
        "expected deferred delivery for {expected_channel}"
    );
    assert_eq!(
        receipt.deferred_reason.as_deref(),
        Some("quiet_hours"),
        "expected quiet_hours reason for {expected_channel}"
    );
    assert_eq!(receipt.channel_id.0, expected_channel);
}

#[test]
fn lifted_quiet_hours_handles_overnight_window() {
    // Identical assertions to the test that previously lived in email.rs;
    // verifies the lift didn't change behavior of the core type.
    let q = QuietHours {
        enabled: true,
        start: "18:00".into(),
        end: "08:00".into(),
        timezone_offset_minutes: 0,
        days: Vec::new(),
    };
    assert!(q.contains(Utc.with_ymd_and_hms(2026, 5, 6, 22, 0, 0).unwrap()));
    assert!(q.contains(Utc.with_ymd_and_hms(2026, 5, 6, 7, 59, 0).unwrap()));
    assert!(!q.contains(Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap()));
}

#[tokio::test]
async fn slack_quiet_hours_defers() {
    let mut routes = BTreeMap::new();
    routes.insert(
        "op-1".into(),
        SlackOperatorRoute {
            workspace_id: "T123".into(),
            channel_id: Some("C123".into()),
            user_id: None,
        },
    );
    let channel = SlackApprovalChannel::new(SlackApprovalChannelConfig {
        channel_id: ChannelId::new("slack"),
        bot_token: "test".into(),
        endpoint: "http://127.0.0.1:9".into(),
        operator_routes: routes,
        rate_limit: RateLimitConfig::unlimited(),
        quiet_hours: Some(always_quiet()),
    });
    let receipt = channel
        .send(notification("op-1", RiskLevel::High))
        .await
        .expect("send");
    assert_quiet_hours_deferred(&receipt, "slack");
    assert!(channel.quiet_hours().is_some());
}

#[tokio::test]
async fn sms_quiet_hours_defers() {
    let mut phones = BTreeMap::new();
    phones.insert("op-1".into(), "+15555550199".into());
    let mut cfg = SmsApprovalChannelConfig::disabled_for_tests(phones);
    cfg.quiet_hours = Some(always_quiet());
    let channel = SmsApprovalChannel::new(cfg);
    let receipt = channel
        .send(notification("op-1", RiskLevel::High))
        .await
        .expect("send");
    assert_quiet_hours_deferred(&receipt, "sms");
    assert!(channel.quiet_hours().is_some());
}

#[tokio::test]
async fn voice_quiet_hours_defers() {
    let mut phones = BTreeMap::new();
    phones.insert("op-1".into(), "+15555550199".into());
    let mut cfg = VoiceApprovalChannelConfig::disabled_for_tests(phones);
    cfg.quiet_hours = Some(always_quiet());
    let channel = VoiceApprovalChannel::new(cfg);
    let receipt = channel
        .send(notification("op-1", RiskLevel::High))
        .await
        .expect("send");
    assert_quiet_hours_deferred(&receipt, "voice");
    assert!(channel.quiet_hours().is_some());
}

#[tokio::test]
async fn whatsapp_quiet_hours_defers() {
    let mut phones = BTreeMap::new();
    phones.insert("op-1".into(), "+15555550199".into());
    let mut cfg = WhatsAppApprovalChannelConfig::disabled_for_tests(phones);
    cfg.quiet_hours = Some(always_quiet());
    let channel = WhatsAppApprovalChannel::new(cfg);
    let receipt = channel
        .send(notification("op-1", RiskLevel::High))
        .await
        .expect("send");
    assert_quiet_hours_deferred(&receipt, "whatsapp");
    assert!(channel.quiet_hours().is_some());
}

#[tokio::test]
async fn imessage_quiet_hours_defers() {
    let mut cfg = IMessageChannelConfig::local("http://127.0.0.1:1");
    cfg.quiet_hours = Some(always_quiet());
    let channel = IMessageChannel::new(cfg).expect("channel");
    let receipt = channel
        .send(notification("op-1", RiskLevel::High))
        .await
        .expect("send");
    assert_quiet_hours_deferred(&receipt, "imessage");
    assert!(channel.quiet_hours().is_some());
}

#[tokio::test]
async fn push_quiet_hours_defers() {
    let dir = tempfile::tempdir().expect("tempdir");
    let registry = dir.path().join("devices.json");
    let mut cfg = PushApprovalChannelConfig::local(registry);
    cfg.quiet_hours = Some(always_quiet());
    let channel = PushApprovalChannel::new(cfg);
    let receipt = channel
        .send(notification("op-1", RiskLevel::High))
        .await
        .expect("send");
    assert_quiet_hours_deferred(&receipt, "push");
    assert!(channel.quiet_hours().is_some());
}

#[tokio::test]
async fn webinbox_quiet_hours_defers() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("webinbox.jsonl");
    let mut cfg = WebInboxChannelConfig::local(&path);
    cfg.quiet_hours = Some(always_quiet());
    let channel = WebInboxChannel::new(cfg);
    let receipt = channel
        .send(notification("op-1", RiskLevel::High))
        .await
        .expect("send");
    assert_quiet_hours_deferred(&receipt, "webinbox");
    assert!(channel.quiet_hours().is_some());
    // Quiet-hours defer must occur BEFORE the inbox file is touched —
    // confirms the check guards the side effect, not just the receipt.
    assert!(!path.exists(), "quiet_hours defer must skip inbox write");
}
