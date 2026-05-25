//! Cross-tenant audit-kind tests for [`crate::event`]. Included via
//! `#[path]` from `event.rs`.
use super::AuditKind;

#[test]
fn handoff_cross_tenant_attempt_round_trips_and_listed() {
    let kind = AuditKind::HandoffCrossTenantAttempt;
    let label = "handoff.cross_tenant_attempt";
    assert_eq!(AuditKind::from_label(label), Some(kind));
    assert_eq!(kind.label(), label);
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&label));
    assert_eq!(
        serde_json::to_string(&kind).unwrap(),
        "\"handoff_cross_tenant_attempt\""
    );
}
