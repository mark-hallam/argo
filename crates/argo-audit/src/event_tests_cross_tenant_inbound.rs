//! Cross-tenant inbound channel audit-kind tests for [`crate::event`].
//! Included via `#[path]` from `event.rs`.
use super::AuditKind;

#[test]
fn cross_tenant_inbound_rejected_round_trips_and_listed() {
    let kind = AuditKind::CrossTenantInboundRejected;
    assert_eq!(
        AuditKind::from_label("channel.cross_tenant_inbound_rejected"),
        Some(kind),
    );
    assert_eq!(kind.label(), "channel.cross_tenant_inbound_rejected");
    assert_eq!(
        serde_json::to_string(&kind).unwrap(),
        "\"cross_tenant_inbound_rejected\"",
    );
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"channel.cross_tenant_inbound_rejected"));
}
