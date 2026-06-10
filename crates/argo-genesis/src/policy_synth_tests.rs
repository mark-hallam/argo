//! Tests for [`crate::policy_synth`] — the boundaries-spec → truth-boundary
//! policy translator.
//!
//! The load-bearing proof is the round-trip: a policy synthesized from an
//! operator's boundaries spec, written as TOML, loaded by the SAME
//! `argo_capability::load_policy` the runtime uses, makes `evaluate_claim`
//! Escalate the gated class — so the boundaries the operator set ARE the
//! boundaries the runtime enforces.

use super::*;
use argo_capability::{
    evaluate_claim, load_policy, ChannelKind, ClaimCandidate, ClaimDisposition,
};
use argo_types::TenantId;

fn cs_role() -> &'static str {
    "Customer success teammate who answers tickets and handles renewals"
}

#[test]
fn smart_defaults_customer_success_financial_read_only_with_refund_cap() {
    let spec = smart_defaults(cs_role());
    let financial = spec
        .access_scopes
        .iter()
        .find(|s| s.category == DataCategory::Financial)
        .expect("financial scope present");
    assert_eq!(financial.access, AccessLevel::ReadOnly);
    assert!(
        spec.approval_threshold_usd.is_some(),
        "customer-success default carries a refund cap"
    );
}

#[test]
fn smart_defaults_three_archetypes_distinct() {
    let sales = smart_defaults("Sales SDR who books demos and qualifies leads");
    let seller = smart_defaults("Marketplace seller assistant managing listings");
    let fallback = smart_defaults("Some entirely novel internal role with no keyword");

    // Each archetype is recognised and pre-fills four category toggles.
    assert_eq!(sales.access_scopes.len(), 4);
    assert_eq!(seller.access_scopes.len(), 4);
    assert_eq!(fallback.access_scopes.len(), 4);
    // Sales carries a (different) cap; the unknown fallback carries none.
    assert_eq!(sales.approval_threshold_usd, Some(1000.0));
    assert_eq!(seller.approval_threshold_usd, Some(250.0));
    assert_eq!(fallback.approval_threshold_usd, None);
}

#[test]
fn round_trip_escalation_topic_enforced_via_load_policy() {
    let dir = tempfile::tempdir().expect("tempdir");
    let tenant = TenantId::new();
    let spec = BoundariesSpec {
        access_scopes: vec![AccessScope {
            category: DataCategory::Financial,
            access: AccessLevel::ReadOnly,
        }],
        approval_threshold_usd: Some(500.0),
        escalation_topics: vec!["legal".into()],
    };

    let path = persist_policy(&spec, tenant, dir.path()).expect("write policy");
    let loaded = load_policy(&path).expect("load synthesized policy via canonical loader");
    assert_eq!(loaded.tenant_id, tenant);
    assert_eq!(loaded.id, GENESIS_BOUNDARIES_POLICY_ID);

    // The escalation topic the operator typed now Escalates a declared claim.
    let escalated = evaluate_claim(
        &loaded,
        ChannelKind::Chat,
        &ClaimCandidate {
            text: "you have a strong legal case".into(),
            declared_classes: vec!["legal".into()],
        },
    );
    assert_eq!(escalated.disposition, ClaimDisposition::Escalate);
    assert_eq!(escalated.matched_class.as_deref(), Some("legal"));

    // A class the operator did NOT gate still passes (Allow-by-default).
    let allowed = evaluate_claim(
        &loaded,
        ChannelKind::Chat,
        &ClaimCandidate {
            text: "the weather is sunny".into(),
            declared_classes: vec!["weather".into()],
        },
    );
    assert_eq!(allowed.disposition, ClaimDisposition::Allow);
}

#[test]
fn dollar_ceiling_gates_financial_even_without_read_only_toggle() {
    let tenant = TenantId::new();
    let spec = BoundariesSpec {
        approval_threshold_usd: Some(250.0),
        ..Default::default()
    };
    let policy = synthesize_policy(&spec, tenant);
    let verdict = evaluate_claim(
        &policy,
        ChannelKind::WebEmbed,
        &ClaimCandidate {
            text: "I'll refund you $900".into(),
            declared_classes: vec!["financial".into()],
        },
    );
    assert_eq!(verdict.disposition, ClaimDisposition::Escalate);
}

#[test]
fn cross_tenant_policy_not_effective_for_another_tenant() {
    // Tenant A synthesizes + persists; the policy is stamped with A's tenant.
    let dir = tempfile::tempdir().expect("tempdir");
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let spec = BoundariesSpec {
        escalation_topics: vec!["compliance".into()],
        ..Default::default()
    };
    let path = persist_policy(&spec, tenant_a, dir.path()).expect("write A's policy");
    let loaded = load_policy(&path).expect("load A's policy");

    // The persisted policy is bound to A, never B. The runtime's per-turn guard
    // (`active_policy_for`) applies a policy ONLY when `policy.tenant_id ==
    // session.tenant_id`, so a B session can never pick up A's boundaries.
    assert_eq!(loaded.tenant_id, tenant_a);
    assert_ne!(loaded.tenant_id, tenant_b);
}

#[test]
fn boundaries_echo_reflects_synthesized_classes() {
    let spec = BoundariesSpec {
        access_scopes: vec![AccessScope {
            category: DataCategory::CustomerPii,
            access: AccessLevel::ReadOnly,
        }],
        approval_threshold_usd: Some(500.0),
        escalation_topics: vec!["Legal Questions".into()],
    };
    let echo = boundaries_echo(&spec);
    let classes: Vec<String> = serde_json::from_value(echo["gated_classes"].clone()).unwrap();
    assert!(classes.contains(&"customer-pii".to_string()));
    assert!(classes.contains(&"financial".to_string()));
    assert!(classes.contains(&"legal-questions".to_string()));
    assert_eq!(echo["approval_threshold_usd"], 500.0);
}

#[test]
fn normalize_topic_collapses_and_trims() {
    assert_eq!(normalize_topic("  Legal  Contracts! "), Some("legal-contracts".into()));
    assert_eq!(normalize_topic("legal"), Some("legal".into()));
    assert_eq!(normalize_topic("   "), None);
    assert_eq!(normalize_topic("!!!"), None);
}

#[test]
fn spec_wire_round_trips_through_json() {
    let spec = BoundariesSpec {
        access_scopes: vec![AccessScope {
            category: DataCategory::SourceCode,
            access: AccessLevel::ReadWrite,
        }],
        approval_threshold_usd: Some(750.0),
        escalation_topics: vec!["security".into()],
    };
    let wire = serde_json::to_string(&spec).unwrap();
    assert!(wire.contains("source_code"));
    assert!(wire.contains("read_write"));
    let back: BoundariesSpec = serde_json::from_str(&wire).unwrap();
    assert_eq!(back, spec);
}

#[test]
fn empty_spec_synthesizes_no_classes() {
    let spec = BoundariesSpec::default();
    assert!(spec.is_empty());
    let policy = synthesize_policy(&spec, TenantId::new());
    assert!(policy.claim_classes.is_empty());
}
