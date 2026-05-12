//! Unit tests for [`crate::event`]. Included via `#[path]` from
//! `event.rs` so the tests stay private to the crate while keeping
//! `event.rs` under the 500-LOC ceiling.
use super::AuditKind;

#[test]
fn label_round_trip_for_every_kind() {
    for k in AuditKind::all() {
        assert_eq!(AuditKind::from_label(k.label()), Some(k), "{:?}", k);
    }
}

#[test]
fn all_eleven_v02_kinds_present() {
    let v02_count = AuditKind::all().iter().filter(|k| k.is_v02()).count();
    assert_eq!(v02_count, 11);
}

#[test]
fn unknown_label_yields_none() {
    assert!(AuditKind::from_label("not_a_kind").is_none());
}

#[test]
fn phase1_kinds_marked_not_v02() {
    for k in [
        AuditKind::TurnStart,
        AuditKind::AssistantMessage,
        AuditKind::ToolCall,
        AuditKind::ToolResult,
        AuditKind::TurnEnd,
        AuditKind::Halt,
    ] {
        assert!(!k.is_v02(), "{:?} should not be v02", k);
    }
}

#[test]
fn escalation_required_is_known_vertical_kind() {
    // Locked: gateway rollups, CLI filters, and the verifier
    // all walk `AuditKind::all()` — escalation.required must
    // be enumerable there.
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"escalation.required"), "got: {labels:?}");
    assert!(AuditKind::EscalationRequired.is_vertical());
    assert!(!AuditKind::EscalationRequired.is_v02());
    assert_eq!(
        AuditKind::from_label("escalation.required"),
        Some(AuditKind::EscalationRequired)
    );
}

#[test]
fn approval_kinds_round_trip_and_are_vertical() {
    for (label, kind) in [
        ("approval.granted", AuditKind::ApprovalGranted),
        ("approval.revoked", AuditKind::ApprovalRevoked),
        ("approval.consumed", AuditKind::ApprovalConsumed),
    ] {
        assert_eq!(AuditKind::from_label(label), Some(kind));
        assert_eq!(kind.label(), label);
        assert!(kind.is_vertical(), "{kind:?} should be vertical");
        assert!(!kind.is_v02(), "{kind:?} must not be v02");
    }
    // all three must appear in all()
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"approval.granted"));
    assert!(labels.contains(&"approval.revoked"));
    assert!(labels.contains(&"approval.consumed"));
}

#[test]
fn capability_gap_round_trips_and_is_vertical() {
    let kind = AuditKind::CapabilityGap;
    assert_eq!(AuditKind::from_label("capability.gap"), Some(kind));
    assert_eq!(kind.label(), "capability.gap");
    assert!(kind.is_vertical());
    assert!(!kind.is_v02());
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"capability.gap"));
}

#[test]
fn worker_oom_restart_round_trips_and_listed() {
    let kind = AuditKind::WorkerOomRestart;
    assert_eq!(AuditKind::from_label("worker.oom_restart"), Some(kind));
    assert_eq!(kind.label(), "worker.oom_restart");
    assert!(!kind.is_v02());
    assert!(!kind.is_vertical());
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"worker.oom_restart"));
}

#[test]
fn runtime_context_compressed_round_trips_and_listed() {
    let kind = AuditKind::RuntimeContextCompressed;
    assert_eq!(
        AuditKind::from_label("runtime.context_compressed"),
        Some(kind)
    );
    assert_eq!(kind.label(), "runtime.context_compressed");
    assert!(!kind.is_v02());
    assert!(!kind.is_vertical());
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"runtime.context_compressed"));
}

#[test]
fn retro_kinds_round_trip_and_listed() {
    for (label, kind) in [
        ("retro.created", AuditKind::RetroCreated),
        ("retro.finalized", AuditKind::RetroFinalized),
    ] {
        assert_eq!(AuditKind::from_label(label), Some(kind));
        assert_eq!(kind.label(), label);
        assert!(!kind.is_v02());
        assert!(!kind.is_vertical());
    }
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"retro.created"));
    assert!(labels.contains(&"retro.finalized"));
}

#[test]
fn security_key_rotated_round_trips_and_listed() {
    let kind = AuditKind::SecurityKeyRotated;
    assert_eq!(AuditKind::from_label("security.key_rotated"), Some(kind));
    assert_eq!(kind.label(), "security.key_rotated");
    assert!(!kind.is_v02());
    assert!(!kind.is_vertical());
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"security.key_rotated"));
}

#[test]
fn approval_expired_by_timeout_round_trips_and_listed() {
    let kind = AuditKind::ApprovalExpiredByTimeout;
    assert_eq!(
        AuditKind::from_label("approval.expired_by_timeout"),
        Some(kind)
    );
    assert_eq!(kind.label(), "approval.expired_by_timeout");
    assert!(!kind.is_v02());
    assert!(!kind.is_vertical());
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"approval.expired_by_timeout"));
}

#[test]
fn legacy_http_request_event_type_strings_still_parse_as_labels() {
    // Acceptance: the new variant labels were chosen to match the
    // historical `event_type` payload strings emitted while the
    // `HttpRequest + event_type` workaround was in place. Audit-feed
    // clients that filtered on those strings keep working unchanged
    // when they switch to the new top-level `kind` column.
    for label in [
        "runtime.context_compressed",
        "retro.created",
        "retro.finalized",
        "security.key_rotated",
        "approval.expired_by_timeout",
    ] {
        assert!(
            AuditKind::from_label(label).is_some(),
            "legacy label {label} must now parse as a first-class kind"
        );
    }
}

#[test]
fn skill_review_kinds_round_trip_and_are_vertical() {
    for (label, kind) in [
        ("skill.approved", AuditKind::SkillApproved),
        ("skill.rejected", AuditKind::SkillRejected),
        ("skill.candidate.promoted", AuditKind::SkillCandidatePromoted),
        ("skill.rollback", AuditKind::SkillRolledBack),
    ] {
        assert_eq!(AuditKind::from_label(label), Some(kind));
        assert_eq!(kind.label(), label);
        assert!(kind.is_vertical());
        assert!(!kind.is_v02());
    }
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"skill.approved"));
    assert!(labels.contains(&"skill.rejected"));
    assert!(labels.contains(&"skill.candidate.promoted"));
    assert!(labels.contains(&"skill.rollback"));
}
