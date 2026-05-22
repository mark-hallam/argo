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
use crate::slack::{SlackApprovalChannel, SlackApprovalChannelConfig, SlackOperatorRoute};
use crate::sms::{SmsApprovalChannel, SmsApprovalChannelConfig};
use crate::voice::{VoiceApprovalChannel, VoiceApprovalChannelConfig};
use crate::webinbox::{WebInboxChannel, WebInboxChannelConfig};
use crate::whatsapp::{WhatsAppApprovalChannel, WhatsAppApprovalChannelConfig};
use crate::fallback::FallbackChain;
use crate::{
    ChannelId, DeliveryStatus, Notification, NotifyChannel, OperatorQuietHoursConfig,
    OperatorTarget, QuietHours, QuietHoursResolver, RateLimitConfig, RiskLevel,
};
use chrono::{DateTime, Duration, TimeZone, Utc};
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

fn assert_quiet_hours_deferred(receipt: &crate::DeliveryReceipt, expected_channel: &str) {
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

// ──────────────────────────────────────────────────────────────────────
// mission-quiet-hours-channels — workspace `[operator.quiet_hours]`
// fallback-chain filter. These tests exercise [`QuietHoursResolver`],
// which lives one layer above the per-channel `quiet_hours` silo and
// removes noisy channel kinds from the dispatcher's `FallbackChain`
// BEFORE any channel send runs.
// ──────────────────────────────────────────────────────────────────────

/// Build a Melbourne-anchored config covering 22:00 → 07:00 local —
/// the brief's worked example. Channel set covers the five canonical
/// noisy kinds.
fn melbourne_overnight_config() -> OperatorQuietHoursConfig {
    OperatorQuietHoursConfig {
        enabled: true,
        start: "22:00".into(),
        end: "07:00".into(),
        timezone: "Australia/Melbourne".into(),
        noisy_channels: vec![
            "sms".into(),
            "voice".into(),
            "whatsapp".into(),
            "imessage".into(),
            "push".into(),
        ],
    }
}

/// Build a UTC `DateTime` at a given Melbourne wall-clock time on
/// 2026-05-23 (the brief's authoring date — a non-DST Saturday in
/// Melbourne, AEST = UTC+10). Lets the tests target a precise window
/// position without round-tripping through `chrono_tz` ourselves.
fn melbourne_dt(hour: u32, minute: u32) -> DateTime<Utc> {
    // AEST = UTC+10 in May; the brief calls out 22:00 Melbourne → 12:00 UTC.
    let utc_hour = (i32::try_from(hour).unwrap_or(0) - 10).rem_euclid(24);
    let utc_hour_u = u32::try_from(utc_hour).unwrap_or(0);
    Utc.with_ymd_and_hms(2026, 5, 23, utc_hour_u, minute, 0)
        .unwrap()
}

#[test]
fn quiet_hours_skips_sms_during_window() {
    // 23:00 Melbourne — squarely inside 22:00 → 07:00. SMS is noisy,
    // WebInbox is not: filter strips SMS, WebInbox survives.
    let resolver = QuietHoursResolver::new(melbourne_overnight_config());
    let now = melbourne_dt(23, 0);
    let chain = FallbackChain::from_order(["sms", "webinbox"]);
    let filtered = resolver.filter_chain(chain, now);
    assert_eq!(filtered.order, vec!["webinbox".to_string()]);
    assert!(resolver.is_quiet_now("sms", now));
    assert!(!resolver.is_quiet_now("webinbox", now));
}

#[test]
fn quiet_hours_no_effect_outside_window() {
    // 12:00 Melbourne — outside 22:00 → 07:00. SMS rings through;
    // chain reaches dispatch unchanged.
    let resolver = QuietHoursResolver::new(melbourne_overnight_config());
    let now = melbourne_dt(12, 0);
    let chain = FallbackChain::from_order(["sms", "webinbox"]);
    let filtered = resolver.filter_chain(chain, now);
    assert_eq!(
        filtered.order,
        vec!["sms".to_string(), "webinbox".to_string()]
    );
    assert!(!resolver.is_quiet_now("sms", now));
}

#[test]
fn quiet_hours_window_wraps_midnight_correctly() {
    // 02:00 Melbourne — the brief's overnight-wrap case. start=22:00
    // end=07:00 must treat 02:00 as inside the window even though
    // the local time is numerically below `start`.
    let resolver = QuietHoursResolver::new(melbourne_overnight_config());
    let now = melbourne_dt(2, 0);
    assert!(
        resolver.is_quiet_now("sms", now),
        "02:00 local must fall inside the overnight 22:00→07:00 window"
    );
    // Sanity: 07:00 sharp is OUTSIDE (end is exclusive).
    let at_seven = melbourne_dt(7, 0);
    assert!(!resolver.is_quiet_now("sms", at_seven));
}

#[test]
fn quiet_hours_disabled_preserves_chain() {
    // enabled=false: the resolver is a no-op even inside what would
    // otherwise be the window. Chain reaches dispatch unchanged.
    let mut cfg = melbourne_overnight_config();
    cfg.enabled = false;
    let resolver = QuietHoursResolver::new(cfg);
    let now = melbourne_dt(23, 0);
    let chain = FallbackChain::from_order(["sms", "voice", "webinbox"]);
    let filtered = resolver.filter_chain(chain.clone(), now);
    assert_eq!(filtered.order, chain.order);
    assert!(!resolver.is_quiet_now("sms", now));
}

#[test]
fn quiet_hours_empty_chain_falls_through_to_webinbox() {
    // All 5 noisy channels in the chain at 02:00 Melbourne — every
    // entry is in quiet, so the filter empties the chain and the
    // resolver appends `webinbox` as the always-on fallthrough.
    let resolver = QuietHoursResolver::new(melbourne_overnight_config());
    let now = melbourne_dt(2, 0);
    let chain = FallbackChain::from_order(["sms", "voice", "whatsapp", "imessage", "push"]);
    let filtered = resolver.filter_chain(chain, now);
    assert_eq!(filtered.order, vec!["webinbox".to_string()]);
}

#[test]
fn quiet_hours_config_roundtrips_through_toml() {
    // Acceptance: `argo.toml` config section parsed cleanly
    // (round-trip serde test). Confirms the operator surface lines
    // up byte-for-byte with the brief's documented shape.
    let body = r#"
        [operator.quiet_hours]
        enabled = true
        start = "22:00"
        end = "07:00"
        timezone = "Australia/Melbourne"
        noisy_channels = ["sms", "voice", "whatsapp", "imessage", "push"]
    "#;
    #[derive(serde::Deserialize)]
    struct File {
        operator: Operator,
    }
    #[derive(serde::Deserialize)]
    struct Operator {
        quiet_hours: OperatorQuietHoursConfig,
    }
    let parsed: File = toml::from_str(body).expect("parse argo.toml shape");
    assert!(parsed.operator.quiet_hours.enabled);
    assert_eq!(parsed.operator.quiet_hours.start, "22:00");
    assert_eq!(parsed.operator.quiet_hours.end, "07:00");
    assert_eq!(parsed.operator.quiet_hours.timezone, "Australia/Melbourne");
    assert_eq!(parsed.operator.quiet_hours.noisy_channels.len(), 5);
    // Construct the resolver from the parsed config and confirm it
    // behaves identically to the equivalent in-memory build.
    let resolver = QuietHoursResolver::new(parsed.operator.quiet_hours);
    assert!(resolver.is_quiet_now("sms", melbourne_dt(23, 0)));
    assert!(!resolver.is_quiet_now("sms", melbourne_dt(12, 0)));
}

#[test]
fn quiet_hours_disabled_by_default_when_absent() {
    // Acceptance: "If absent or `enabled = false`, behavior is
    // unchanged from current main." Parsing an empty TOML produces a
    // disabled resolver via `OperatorQuietHoursConfig::default`.
    let cfg = OperatorQuietHoursConfig::default();
    assert!(!cfg.enabled);
    let resolver = QuietHoursResolver::new(cfg);
    let chain = FallbackChain::from_order(["sms", "webinbox"]);
    let filtered = resolver.filter_chain(chain.clone(), melbourne_dt(23, 0));
    assert_eq!(filtered.order, chain.order);
}

#[test]
fn quiet_hours_resolver_from_missing_path_is_disabled() {
    // `from_toml_path` must never panic on a missing file — the
    // dispatcher calls it on every action and operators routinely run
    // without an `argo.toml`.
    let dir = tempfile::tempdir().expect("tempdir");
    let resolver = QuietHoursResolver::from_toml_path(&dir.path().join("argo.toml"));
    assert!(!resolver.config().enabled);
    let chain = FallbackChain::from_order(["sms", "webinbox"]);
    let filtered = resolver.filter_chain(chain.clone(), melbourne_dt(23, 0));
    assert_eq!(filtered.order, chain.order);
}

#[test]
fn quiet_hours_resolver_coexists_with_operator_preferences_block() {
    // The workspace `[operator.quiet_hours]` block must parse cleanly
    // alongside an unrelated `[operator.<id>.preferences]` block —
    // the two surfaces live under the same `operator.*` namespace and
    // a naive struct shape could clobber one or the other.
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("argo.toml");
    std::fs::write(
        &path,
        r#"
            [operator.quiet_hours]
            enabled = true
            start = "22:00"
            end = "07:00"
            timezone = "Australia/Melbourne"
            noisy_channels = ["sms"]

            [operator.alice.preferences]
            primary_channel = "slack"
        "#,
    )
    .expect("write argo.toml");
    let resolver = QuietHoursResolver::from_toml_path(&path);
    assert!(resolver.config().enabled);
    assert!(resolver.is_quiet_now("sms", melbourne_dt(23, 0)));
}
