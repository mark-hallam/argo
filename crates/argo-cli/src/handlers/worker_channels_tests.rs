//! Unit tests for `crate::handlers::worker_channels`. Sibling-tests
//! pattern (`#[path = "..._tests.rs"] mod tests;`) keeps the parent
//! handler under the 500-LOC ceiling.

use super::{add_to, channel_matches, parse_channel_spec, MAX_CHANNELS_PER_WORKER};
use argo_identity::NotifyChannelRef;
use argo_types::ArgoError;

fn ok_email() -> NotifyChannelRef {
    parse_channel_spec("email:ops@example.com").unwrap()
}

#[test]
fn parse_email_happy_path() {
    let parsed = parse_channel_spec("email:ops@example.com").unwrap();
    assert_eq!(parsed.channel, "email");
    assert_eq!(parsed.target, "ops@example.com");
    assert!(parsed.risk_level.is_none());
}

#[test]
fn parse_slack_happy_path() {
    let parsed = parse_channel_spec("slack:U0123ABCD").unwrap();
    assert_eq!(parsed.channel, "slack");
    assert_eq!(parsed.target, "U0123ABCD");
}

#[test]
fn parse_whatsapp_e164() {
    let parsed = parse_channel_spec("whatsapp:+15551234567").unwrap();
    assert_eq!(parsed.channel, "whatsapp");
    assert_eq!(parsed.target, "+15551234567");
}

#[test]
fn parse_sms_e164() {
    let parsed = parse_channel_spec("sms:+447911123456").unwrap();
    assert_eq!(parsed.channel, "sms");
}

#[test]
fn parse_push_no_shape_check() {
    // Push device ids are opaque — anything non-empty is accepted.
    let parsed = parse_channel_spec("push:operator-iphone").unwrap();
    assert_eq!(parsed.target, "operator-iphone");
}

#[test]
fn parse_each_supported_kind_round_trips() {
    for (kind, target) in [
        ("push", "iphone"),
        ("webinbox", "tok-abc"),
        ("slack", "C123"),
        ("discord", "789"),
        ("chat", "https://chat.example.com/hook"),
        ("sse", "session-1"),
    ] {
        let spec = format!("{kind}:{target}");
        let parsed = parse_channel_spec(&spec).unwrap();
        assert_eq!(parsed.channel, kind, "kind mismatch for spec {spec}");
        assert_eq!(parsed.target, target);
    }
}

#[test]
fn parse_rejects_missing_colon() {
    let err = parse_channel_spec("foo").unwrap_err();
    assert!(matches!(err, ArgoError::Schema(_)), "got {err:?}");
}

#[test]
fn parse_rejects_unknown_kind() {
    let err = parse_channel_spec("telex:hello").unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("not supported"), "got {msg}");
}

#[test]
fn parse_rejects_empty_kind() {
    let err = parse_channel_spec(":target").unwrap_err();
    assert!(matches!(err, ArgoError::Schema(_)));
}

#[test]
fn parse_rejects_empty_target() {
    let err = parse_channel_spec("email:").unwrap_err();
    assert!(matches!(err, ArgoError::Schema(_)));
}

#[test]
fn parse_rejects_bad_email_no_at() {
    let err = parse_channel_spec("email:not-an-email").unwrap_err();
    assert!(matches!(err, ArgoError::Schema(_)));
}

#[test]
fn parse_rejects_bad_email_no_dot_in_domain() {
    let err = parse_channel_spec("email:ops@local").unwrap_err();
    assert!(matches!(err, ArgoError::Schema(_)));
}

#[test]
fn parse_rejects_bad_email_trailing_dot() {
    let err = parse_channel_spec("email:ops@example.").unwrap_err();
    assert!(matches!(err, ArgoError::Schema(_)));
}

#[test]
fn parse_rejects_whatsapp_missing_plus() {
    let err = parse_channel_spec("whatsapp:15551234567").unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("E.164"), "got {msg}");
}

#[test]
fn parse_rejects_whatsapp_too_short() {
    let err = parse_channel_spec("whatsapp:+12345").unwrap_err();
    assert!(matches!(err, ArgoError::Schema(_)));
}

#[test]
fn parse_rejects_whatsapp_non_digits() {
    let err = parse_channel_spec("whatsapp:+1555ABCD123").unwrap_err();
    assert!(matches!(err, ArgoError::Schema(_)));
}

#[test]
fn add_to_appends_when_unique() {
    let mut current = vec![ok_email()];
    let added = parse_channel_spec("slack:C1").unwrap();
    let outcome = add_to(&mut current, added).unwrap();
    assert!(outcome.changed);
    assert!(!outcome.duplicate);
    assert_eq!(outcome.total, 2);
    assert_eq!(current.len(), 2);
}

#[test]
fn add_to_idempotent_dup() {
    let mut current = vec![ok_email()];
    let dup = parse_channel_spec("email:ops@example.com").unwrap();
    let outcome = add_to(&mut current, dup).unwrap();
    assert!(!outcome.changed);
    assert!(outcome.duplicate);
    assert_eq!(outcome.total, 1);
    assert_eq!(current.len(), 1, "duplicate must NOT push a second copy");
}

#[test]
fn add_to_dup_ignores_risk_level_difference() {
    // `risk_level` is a presentation filter, not part of identity —
    // adding the same (kind, target) with a different risk level is
    // still a duplicate by spec.
    let mut current = vec![NotifyChannelRef {
        channel: "email".into(),
        target: "ops@example.com".into(),
        risk_level: Some("high".into()),
    }];
    let no_risk = parse_channel_spec("email:ops@example.com").unwrap();
    let outcome = add_to(&mut current, no_risk).unwrap();
    assert!(outcome.duplicate, "(kind, target) determines identity");
}

#[test]
fn add_to_rejects_at_cap() {
    let mut current: Vec<NotifyChannelRef> = (0..MAX_CHANNELS_PER_WORKER)
        .map(|i| NotifyChannelRef::new("push", format!("device-{i}")))
        .collect();
    let extra = parse_channel_spec("email:ops@example.com").unwrap();
    let err = add_to(&mut current, extra).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("cap reached"), "got {msg}");
    assert_eq!(current.len(), MAX_CHANNELS_PER_WORKER);
}

#[test]
fn channel_matches_pair_only() {
    let a = NotifyChannelRef::new("email", "ops@example.com");
    let b = NotifyChannelRef::new("email", "ops@example.com");
    let mut c = NotifyChannelRef::new("email", "ops@example.com");
    c.risk_level = Some("high".into());
    let d = NotifyChannelRef::new("slack", "C1");
    assert!(channel_matches(&a, &b));
    assert!(channel_matches(&a, &c), "risk_level not part of identity");
    assert!(!channel_matches(&a, &d));
}

#[test]
fn add_to_then_match_lookup() {
    // Round-trip: add a channel, then verify channel_matches finds it.
    let mut current: Vec<NotifyChannelRef> = vec![];
    let added = parse_channel_spec("slack:C2").unwrap();
    let outcome = add_to(&mut current, added).unwrap();
    assert!(outcome.changed);
    let target = parse_channel_spec("slack:C2").unwrap();
    assert!(current.iter().any(|ch| channel_matches(ch, &target)));
}
