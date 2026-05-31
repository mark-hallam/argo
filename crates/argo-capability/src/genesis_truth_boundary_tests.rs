//! Tests for the conservative default Genesis truth-boundary policy
//! (DEC-025 #6 / Genesis P5 part (a)).

use super::*;
use crate::truth_boundary::{evaluate_claim, ChannelKind, ClaimCandidate, ClaimDisposition};
use argo_types::TenantId;

/// Every gated claim class escalates on EVERY channel — the conservative
/// "hold for the operator" posture. Adversarial: prove there is no channel a
/// gated claim slips through on.
#[test]
fn every_gated_class_escalates_on_every_channel() {
    let tenant = TenantId::new();
    let policy = genesis_default_policy(tenant);
    let channels = [
        ChannelKind::Rehearsal,
        ChannelKind::WebEmbed,
        ChannelKind::LiveMeeting,
        ChannelKind::Chat,
    ];
    for class in GENESIS_GATED_CLAIM_CLASSES {
        for ch in channels {
            let verdict = evaluate_claim(
                &policy,
                ch,
                &ClaimCandidate {
                    text: format!("a {class} claim"),
                    declared_classes: vec![(*class).to_string()],
                },
            );
            assert_eq!(
                verdict.disposition,
                ClaimDisposition::Escalate,
                "class '{class}' must escalate on channel {ch:?}",
            );
            assert_eq!(verdict.matched_class.as_deref(), Some(*class));
        }
    }
}

/// The policy is tenant-scoped: its `tenant_id` is exactly the worker's
/// tenant. The runner's per-turn guard (`active_policy_for`) uses this to
/// refuse applying one tenant's policy to another tenant's session.
#[test]
fn policy_is_scoped_to_the_supplied_tenant() {
    let a = TenantId::new();
    let b = TenantId::new();
    assert_eq!(genesis_default_policy(a).tenant_id, a);
    assert_eq!(genesis_default_policy(b).tenant_id, b);
    assert_ne!(
        genesis_default_policy(a).tenant_id,
        genesis_default_policy(b).tenant_id,
    );
}

/// The honest ceiling: an UNDECLARED claim is Allow-by-default (the semantic
/// classifier was never built). We assert this explicitly so the limitation
/// is captured as a test, not just prose.
#[test]
fn undeclared_claim_is_allow_by_default_the_honest_ceiling() {
    let policy = genesis_default_policy(TenantId::new());
    let verdict = evaluate_claim(
        &policy,
        ChannelKind::WebEmbed,
        &ClaimCandidate {
            text: "Our ARR is $4.2M and this is legal advice.".into(),
            declared_classes: vec![],
        },
    );
    assert_eq!(
        verdict.disposition,
        ClaimDisposition::Allow,
        "an undeclared claim is Allow-by-default — documented ceiling",
    );
    assert!(verdict.matched_class.is_none());
}

/// A claim class NOT in the gated set passes (the policy is a deny-list of
/// high-risk classes, not an allow-list — a benign declared class like
/// `greeting` is not held).
#[test]
fn non_gated_declared_class_is_allowed() {
    let policy = genesis_default_policy(TenantId::new());
    let verdict = evaluate_claim(
        &policy,
        ChannelKind::Chat,
        &ClaimCandidate {
            text: "Hello, how can I help?".into(),
            declared_classes: vec!["greeting".into()],
        },
    );
    assert_eq!(verdict.disposition, ClaimDisposition::Allow);
}

/// The bound policy carries the canonical id the provenance marker + the
/// runner key off. A drift between this constant and the four gated classes
/// would silently weaken the binding, so pin both.
#[test]
fn policy_id_and_class_set_are_stable() {
    let policy = genesis_default_policy(TenantId::new());
    assert_eq!(policy.id, GENESIS_DEFAULT_POLICY_ID);
    assert_eq!(policy.id, "genesis-default-conservative");
    assert_eq!(policy.claim_classes.len(), 4);
    let names: Vec<&str> = policy.claim_classes.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["factual", "financial", "legal", "medical"]);
}
