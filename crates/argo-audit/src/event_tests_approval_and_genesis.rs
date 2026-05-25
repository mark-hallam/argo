//! Approval, Genesis, and onboarding audit-kind tests for
//! [`crate::event`]. Included via `#[path]` from `event.rs`.
use super::AuditKind;

#[test]
fn approval_declined_round_trips_and_listed() {
    let kind = AuditKind::ApprovalDeclined;
    assert_eq!(AuditKind::from_label("approval.declined"), Some(kind));
    assert_eq!(kind.label(), "approval.declined");
    assert_eq!(
        serde_json::to_string(&kind).unwrap(),
        "\"approval_declined\""
    );
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"approval.declined"));
}

#[test]
fn genesis_fallback_used_round_trips_and_listed() {
    let kind = AuditKind::GenesisFallbackUsed;
    assert_eq!(AuditKind::from_label("genesis.fallback_used"), Some(kind));
    assert_eq!(kind.label(), "genesis.fallback_used");
    assert_eq!(
        serde_json::to_string(&kind).unwrap(),
        "\"genesis_fallback_used\""
    );
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"genesis.fallback_used"));
}

#[test]
fn approval_kinds_round_trip() {
    for (label, kind) in [
        ("approval.asked", AuditKind::ApprovalAsked),
        ("approval.replied", AuditKind::ApprovalReplied),
        ("approval.granted", AuditKind::ApprovalGranted),
        ("approval.revoked", AuditKind::ApprovalRevoked),
        ("approval.consumed", AuditKind::ApprovalConsumed),
    ] {
        assert_eq!(AuditKind::from_label(label), Some(kind));
        assert_eq!(kind.label(), label);
    }
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"approval.asked"));
    assert!(labels.contains(&"approval.replied"));
    assert!(labels.contains(&"approval.granted"));
    assert!(labels.contains(&"approval.revoked"));
    assert!(labels.contains(&"approval.consumed"));
}

#[test]
fn approval_asked_and_replied_round_trip_and_listed() {
    // Audit finding P0-2 — the ask/reply variants document the
    // notify-dispatch → channel-reply round-trip that the chain
    // forensic trail otherwise lost. Both must round-trip and
    // appear in `all()` so the verifier and gateway rollups
    // categorise them as first-class kinds (not via the legacy
    // `HttpRequest + event_type=` workaround).
    let kinds = [
        ("approval.asked", AuditKind::ApprovalAsked),
        ("approval.replied", AuditKind::ApprovalReplied),
    ];
    for (label, kind) in kinds {
        assert_eq!(AuditKind::from_label(label), Some(kind));
        assert_eq!(kind.label(), label);
    }
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    for (label, _) in kinds {
        assert!(labels.contains(&label), "missing label {label}: {labels:?}");
    }
}

#[test]
fn approval_expired_by_timeout_round_trips_and_listed() {
    let kind = AuditKind::ApprovalExpiredByTimeout;
    assert_eq!(
        AuditKind::from_label("approval.expired_by_timeout"),
        Some(kind)
    );
    assert_eq!(kind.label(), "approval.expired_by_timeout");
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"approval.expired_by_timeout"));
}

#[test]
fn onboarding_kinds_round_trip_and_listed() {
    for (label, kind) in [
        (
            "onboarding.provider_key_configured",
            AuditKind::ProviderKeyConfigured,
        ),
        (
            "onboarding.vertical_installed",
            AuditKind::VerticalInstalled,
        ),
    ] {
        assert_eq!(AuditKind::from_label(label), Some(kind));
        assert_eq!(kind.label(), label);
    }
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"onboarding.provider_key_configured"));
    assert!(labels.contains(&"onboarding.vertical_installed"));
}
