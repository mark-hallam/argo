//! Unit tests for the `truth_boundary` module.
//!
//! Covers the acceptance criteria from
//! `streams/pitch-truth-boundary-capability-schema.md`:
//!  - Allow-by-default for unmatched classes
//!  - Forbid for matched class on in-scope channel
//!  - Allow for matched class on out-of-scope channel (Except)
//!  - Only-list channel scoping
//!  - Escalate disposition
//!  - Policy loader rejects malformed TOML
//!  - Refusal-template loader rejects empty `variants`
//!  - Multi-tenant: wrong-tenant lookup returns no-match (cross-tenant negative)
//!  - Example policies load cleanly from the workspace root
//!  - Both example policies parse with non-empty claim_classes

use crate::truth_boundary::{
    ChannelKind, ChannelScope, ClaimCandidate, ClaimClass, ClaimDisposition, ClaimEvaluation,
    EscalationKind, ProvenanceSource, TruthBoundaryPolicy, evaluate_claim,
};
use crate::truth_boundary_policies::{
    load_policies_from_dir, load_policy, load_refusal_template,
};
use argo_types::TenantId;
use std::path::PathBuf;

/// Locate the workspace root by walking up from CARGO_MANIFEST_DIR
/// (`crates/argo-capability/`) until we find the policies/ dir.
fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut cur = manifest.as_path();
    loop {
        if cur.join("policies").is_dir() {
            return cur.to_path_buf();
        }
        match cur.parent() {
            Some(p) => cur = p,
            None => panic!("could not find workspace root with policies/ dir"),
        }
    }
}

fn skycot_tenant() -> TenantId {
    // Matches policies/truth-boundary/skycot-public-pitch.toml
    TenantId(
        uuid::Uuid::parse_str("29f29b2b-31b4-4a14-9613-b09c0fd7da12")
            .expect("hardcoded test uuid is valid"),
    )
}

fn forbid_revenue_policy() -> TruthBoundaryPolicy {
    TruthBoundaryPolicy {
        id: "test-policy".into(),
        tenant_id: skycot_tenant(),
        claim_classes: vec![ClaimClass {
            name: "revenue".into(),
            disposition: ClaimDisposition::Forbid {
                refusal_template_id: "defer-to-mark-financial".into(),
            },
            channels: ChannelScope::AllChannels,
            escalate_to: None,
        }],
    }
}

#[test]
fn allow_when_no_claim_class_matches() {
    let policy = forbid_revenue_policy();
    let candidate = ClaimCandidate {
        text: "We exist.".into(),
        declared_classes: vec!["pleasantry".into()],
    };
    let r = evaluate_claim(&policy, ChannelKind::WebEmbed, &candidate);
    assert_eq!(r.disposition, ClaimDisposition::Allow);
    assert!(r.matched_class.is_none());
}

#[test]
fn allow_when_declared_classes_empty() {
    let policy = forbid_revenue_policy();
    let candidate = ClaimCandidate {
        text: "Hello world".into(),
        declared_classes: vec![],
    };
    let r = evaluate_claim(&policy, ChannelKind::WebEmbed, &candidate);
    assert_eq!(r.disposition, ClaimDisposition::Allow);
    assert!(r.matched_class.is_none());
}

#[test]
fn forbid_when_class_matches_on_all_channels() {
    let policy = forbid_revenue_policy();
    let candidate = ClaimCandidate {
        text: "Our ARR is $4.2M".into(),
        declared_classes: vec!["revenue".into()],
    };
    let r = evaluate_claim(&policy, ChannelKind::WebEmbed, &candidate);
    assert!(matches!(r.disposition, ClaimDisposition::Forbid { .. }));
    assert_eq!(r.matched_class.as_deref(), Some("revenue"));
    let ClaimDisposition::Forbid {
        refusal_template_id,
    } = r.disposition
    else {
        panic!("expected Forbid");
    };
    assert_eq!(refusal_template_id, "defer-to-mark-financial");
}

#[test]
fn channel_scope_only_in_scope_forbids() {
    let policy = TruthBoundaryPolicy {
        id: "p".into(),
        tenant_id: skycot_tenant(),
        claim_classes: vec![ClaimClass {
            name: "customer-count".into(),
            disposition: ClaimDisposition::Forbid {
                refusal_template_id: "defer-to-public-customers-page".into(),
            },
            channels: ChannelScope::Only(vec![ChannelKind::WebEmbed, ChannelKind::LiveMeeting]),
            escalate_to: None,
        }],
    };
    let candidate = ClaimCandidate {
        text: "We have 28 customers".into(),
        declared_classes: vec!["customer-count".into()],
    };
    let in_scope = evaluate_claim(&policy, ChannelKind::WebEmbed, &candidate);
    assert!(matches!(in_scope.disposition, ClaimDisposition::Forbid { .. }));
    let out_of_scope = evaluate_claim(&policy, ChannelKind::Rehearsal, &candidate);
    assert_eq!(out_of_scope.disposition, ClaimDisposition::Allow);
    assert!(out_of_scope.matched_class.is_none());
}

#[test]
fn channel_scope_except_inverts() {
    let policy = TruthBoundaryPolicy {
        id: "p".into(),
        tenant_id: skycot_tenant(),
        claim_classes: vec![ClaimClass {
            name: "revenue".into(),
            disposition: ClaimDisposition::Forbid {
                refusal_template_id: "defer-to-mark-financial".into(),
            },
            channels: ChannelScope::Except(vec![ChannelKind::Rehearsal]),
            escalate_to: None,
        }],
    };
    let candidate = ClaimCandidate {
        text: "ARR is $4.2M".into(),
        declared_classes: vec!["revenue".into()],
    };
    // out of scope (rehearsal) — Allow
    let rehearsal = evaluate_claim(&policy, ChannelKind::Rehearsal, &candidate);
    assert_eq!(rehearsal.disposition, ClaimDisposition::Allow);
    assert!(rehearsal.matched_class.is_none());
    // in scope (anything else) — Forbid
    let public = evaluate_claim(&policy, ChannelKind::WebEmbed, &candidate);
    assert!(matches!(public.disposition, ClaimDisposition::Forbid { .. }));
    assert_eq!(public.matched_class.as_deref(), Some("revenue"));
}

#[test]
fn escalate_disposition_round_trips() {
    let policy = TruthBoundaryPolicy {
        id: "p".into(),
        tenant_id: skycot_tenant(),
        claim_classes: vec![ClaimClass {
            name: "unannounced-roadmap".into(),
            disposition: ClaimDisposition::Escalate,
            channels: ChannelScope::AllChannels,
            escalate_to: None,
        }],
    };
    let candidate = ClaimCandidate {
        text: "We'll ship X next quarter.".into(),
        declared_classes: vec!["unannounced-roadmap".into()],
    };
    let r = evaluate_claim(&policy, ChannelKind::WebEmbed, &candidate);
    assert_eq!(r.disposition, ClaimDisposition::Escalate);
    assert_eq!(r.matched_class.as_deref(), Some("unannounced-roadmap"));
}

#[test]
fn cross_tenant_lookup_returns_no_match() {
    // The evaluator is pure; cross-tenant scoping is enforced at the
    // CALLER level (the engine resolves policy by `tenant_id + policy_id`).
    // We simulate that: a caller looking up "skycot-public-pitch" with
    // the WRONG tenant gets None back, and the runtime treats that as
    // Allow-by-default (no policy → no claim restrictions).
    let skycot_policy = forbid_revenue_policy();
    let registry: Vec<TruthBoundaryPolicy> = vec![skycot_policy.clone()];

    let wrong_tenant = TenantId::new();
    let resolved: Option<&TruthBoundaryPolicy> = registry
        .iter()
        .find(|p| p.id == "test-policy" && p.tenant_id == wrong_tenant);
    assert!(
        resolved.is_none(),
        "cross-tenant policy lookup must not match"
    );

    // And with the RIGHT tenant, the same lookup DOES match —
    // confirming the negative is meaningful, not a tautology.
    let resolved_right: Option<&TruthBoundaryPolicy> = registry
        .iter()
        .find(|p| p.id == "test-policy" && p.tenant_id == skycot_policy.tenant_id);
    assert!(resolved_right.is_some());
}

#[test]
fn malformed_toml_rejected_with_useful_error() {
    let tmp = std::env::temp_dir().join(format!(
        "argo-capability-test-malformed-{}.toml",
        uuid::Uuid::now_v7()
    ));
    std::fs::write(&tmp, "this is = not [ valid toml").unwrap();
    let err = load_policy(&tmp).expect_err("malformed TOML must error");
    let msg = err.to_string();
    assert!(msg.contains("malformed"), "error should mention 'malformed', got: {msg}");
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn policy_with_empty_id_rejected() {
    let tmp = std::env::temp_dir().join(format!(
        "argo-capability-test-empty-id-{}.toml",
        uuid::Uuid::now_v7()
    ));
    std::fs::write(
        &tmp,
        "id = \"\"\ntenant_id = \"29f29b2b-31b4-4a14-9613-b09c0fd7da12\"\n",
    )
    .unwrap();
    let err = load_policy(&tmp).expect_err("empty id must error");
    assert!(err.to_string().contains("empty id"));
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn policy_with_invalid_tenant_uuid_rejected() {
    let tmp = std::env::temp_dir().join(format!(
        "argo-capability-test-bad-uuid-{}.toml",
        uuid::Uuid::now_v7()
    ));
    std::fs::write(
        &tmp,
        "id = \"p\"\ntenant_id = \"not-a-uuid\"\n[[claim_classes]]\nname = \"x\"\ndisposition = \"Allow\"\nchannels = \"AllChannels\"\n",
    )
    .unwrap();
    let err = load_policy(&tmp).expect_err("non-uuid tenant_id must error");
    assert!(err.to_string().contains("not a valid UUID"));
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn forbid_without_refusal_template_rejected() {
    let tmp = std::env::temp_dir().join(format!(
        "argo-capability-test-forbid-{}.toml",
        uuid::Uuid::now_v7()
    ));
    std::fs::write(
        &tmp,
        "id = \"p\"\ntenant_id = \"29f29b2b-31b4-4a14-9613-b09c0fd7da12\"\n[[claim_classes]]\nname = \"revenue\"\ndisposition = \"Forbid\"\nchannels = \"AllChannels\"\n",
    )
    .unwrap();
    let err = load_policy(&tmp).expect_err("Forbid without refusal_template_id must error");
    assert!(err.to_string().contains("refusal_template_id"));
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn refusal_template_with_empty_variants_rejected() {
    let tmp = std::env::temp_dir().join(format!(
        "argo-capability-test-empty-variants-{}.toml",
        uuid::Uuid::now_v7()
    ));
    std::fs::write(
        &tmp,
        "id = \"defer-to-mark-financial\"\nvariants = []\n",
    )
    .unwrap();
    let err = load_refusal_template(&tmp)
        .expect_err("empty variants must error");
    assert!(err.to_string().contains("empty variants"));
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn example_skycot_public_pitch_loads_cleanly() {
    let path = workspace_root()
        .join("policies/truth-boundary/skycot-public-pitch.toml");
    let policy = load_policy(&path).expect("skycot-public-pitch.toml must parse");
    assert_eq!(policy.id, "skycot-public-pitch");
    assert!(
        !policy.claim_classes.is_empty(),
        "example policy must have at least one claim class"
    );
    assert!(
        policy.claim_classes.iter().any(|c| c.name == "revenue"),
        "skycot-public-pitch must lock down revenue"
    );
    // The escalation target on the unannounced-roadmap class must parse.
    let roadmap = policy
        .claim_classes
        .iter()
        .find(|c| c.name == "unannounced-roadmap")
        .expect("unannounced-roadmap class present");
    let target = roadmap
        .escalate_to
        .as_ref()
        .expect("unannounced-roadmap has escalate_to target");
    assert_eq!(target.kind, EscalationKind::Approver);
    assert_eq!(target.address, "founder@example.com");
}

#[test]
fn example_skycot_private_rehearsal_loads_cleanly() {
    let path = workspace_root()
        .join("policies/truth-boundary/skycot-private-rehearsal.toml");
    let policy = load_policy(&path).expect("skycot-private-rehearsal.toml must parse");
    assert_eq!(policy.id, "skycot-private-rehearsal");
    // The whole point of the rehearsal policy: revenue is Allow, not Forbid,
    // and only scoped to the Rehearsal channel.
    let rev = policy
        .claim_classes
        .iter()
        .find(|c| c.name == "revenue")
        .expect("revenue class present");
    assert_eq!(rev.disposition, ClaimDisposition::Allow);
    match &rev.channels {
        ChannelScope::Only(list) => {
            assert_eq!(list, &vec![ChannelKind::Rehearsal]);
        }
        other => panic!("expected Only:Rehearsal, got {other:?}"),
    }
}

#[test]
fn load_policies_from_dir_picks_up_both_examples() {
    let dir = workspace_root().join("policies/truth-boundary");
    let policies = load_policies_from_dir(&dir).expect("dir must load");
    assert!(
        policies.len() >= 2,
        "expected at least two policies in {dir:?}, got {}",
        policies.len()
    );
    assert!(policies.iter().any(|p| p.id == "skycot-public-pitch"));
    assert!(policies.iter().any(|p| p.id == "skycot-private-rehearsal"));
}

#[test]
fn evaluation_carries_default_generative_provenance() {
    // The pure evaluator records Generative provenance by default;
    // P-04 (canonical Q&A) will overwrite it before evaluation.
    let policy = forbid_revenue_policy();
    let candidate = ClaimCandidate {
        text: "x".into(),
        declared_classes: vec!["revenue".into()],
    };
    let r: ClaimEvaluation = evaluate_claim(&policy, ChannelKind::WebEmbed, &candidate);
    assert_eq!(r.provenance.source, ProvenanceSource::Generative);
    assert_eq!(r.confidence, 1.0);
}

#[test]
fn channel_scope_covers_helper_is_consistent() {
    // Direct check of the covers() helper that the evaluator uses.
    assert!(ChannelScope::AllChannels.covers(ChannelKind::WebEmbed));
    assert!(ChannelScope::AllChannels.covers(ChannelKind::Rehearsal));

    let only_web = ChannelScope::Only(vec![ChannelKind::WebEmbed]);
    assert!(only_web.covers(ChannelKind::WebEmbed));
    assert!(!only_web.covers(ChannelKind::Rehearsal));

    let except_rehearsal = ChannelScope::Except(vec![ChannelKind::Rehearsal]);
    assert!(except_rehearsal.covers(ChannelKind::WebEmbed));
    assert!(!except_rehearsal.covers(ChannelKind::Rehearsal));
}

#[test]
fn unknown_channel_in_toml_rejected() {
    let tmp = std::env::temp_dir().join(format!(
        "argo-capability-test-bad-channel-{}.toml",
        uuid::Uuid::now_v7()
    ));
    std::fs::write(
        &tmp,
        "id = \"p\"\ntenant_id = \"29f29b2b-31b4-4a14-9613-b09c0fd7da12\"\n[[claim_classes]]\nname = \"x\"\ndisposition = \"Allow\"\nchannels = \"Only:SmokeSignals\"\n",
    )
    .unwrap();
    let err = load_policy(&tmp).expect_err("unknown channel must error");
    assert!(err.to_string().contains("unknown channel"));
    let _ = std::fs::remove_file(&tmp);
}

