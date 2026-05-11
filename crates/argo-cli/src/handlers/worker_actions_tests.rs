//! Unit tests for `worker_actions` (#728). Extracted from the
//! main module to keep `handlers/worker_actions.rs` under the
//! 500-LOC budget.
//!
//! Loaded via `#[cfg(test)] #[path = "worker_actions_tests.rs"]
//! mod tests;` at the bottom of `worker_actions.rs`.

use super::*;
use crate::handlers::worker_actions_emit::{
    build_emission, classify_for_emission, ActionEmissionInputs, ApprovalCategory, RiskOverride,
};

#[test]
fn action_state_round_trips_through_string() {
    for s in [
        "pending",
        "approved",
        "executing",
        "completed",
        "failed",
        "blocked",
    ] {
        let parsed = ActionState::parse(s).expect(s);
        assert_eq!(parsed.as_str(), s);
    }
    assert!(ActionState::parse("unknown").is_none());
}

#[test]
fn action_state_terminal_set_is_completed_failed_blocked() {
    assert!(!ActionState::Pending.is_terminal());
    assert!(!ActionState::Approved.is_terminal());
    assert!(!ActionState::Executing.is_terminal());
    assert!(ActionState::Completed.is_terminal());
    assert!(ActionState::Failed.is_terminal());
    assert!(ActionState::Blocked.is_terminal());
}

#[test]
fn action_state_transition_matrix_matches_design() {
    use ActionState::*;
    // Legal transitions per the module docs:
    assert!(Pending.can_transition_to(Approved));
    assert!(Pending.can_transition_to(Blocked));
    // Autonomous-safe path — manifest's `requires_approval = false`
    // actions go pending → executing directly. The
    // approval gate for `requires_approval = true` rows is
    // enforced in the `execute` handler, not the state matrix.
    assert!(Pending.can_transition_to(Executing));
    assert!(Approved.can_transition_to(Executing));
    assert!(Approved.can_transition_to(Completed));
    assert!(Executing.can_transition_to(Completed));
    assert!(Executing.can_transition_to(Failed));
    // Illegal — these MUST NOT be allowed by the writer:
    assert!(
        !Pending.can_transition_to(Completed),
        "must execute first; complete from pending is not legal"
    );
    assert!(!Completed.can_transition_to(Approved));
    assert!(!Failed.can_transition_to(Approved));
    assert!(!Blocked.can_transition_to(Approved));
    assert!(!Approved.can_transition_to(Pending));
}

/// Locks the SKYCOT safety boundary: every action that touches
/// money / public surfaces / buyers requires approval.
/// Autonomous-safe actions (drafting, verifying, asking for
/// photos) do not. A regression here would let the worker post
/// listings or message buyers without human consent.
#[test]
fn action_type_approval_boundary_matches_safety_spec() {
    // Autonomous-safe — worker may execute without operator approval.
    assert!(!ActionType::CreateListingDraft.requires_approval());
    assert!(!ActionType::VerifyPrice.requires_approval());
    assert!(!ActionType::RequestPhotos.requires_approval());
    // Approval-gated — touches money / public surfaces / buyers.
    assert!(ActionType::PostListing.requires_approval());
    assert!(ActionType::BoostListing.requires_approval());
    assert!(ActionType::ReplyToBuyer.requires_approval());
    assert!(ActionType::FollowUpBuyer.requires_approval());
    assert!(ActionType::MarkItemSold.requires_approval());
    assert!(ActionType::UpdatePrice.requires_approval());
    assert!(ActionType::RelistItem.requires_approval());
}

#[test]
fn action_type_round_trips_through_string() {
    for t in [
        "create_listing_draft",
        "verify_price",
        "request_photos",
        "post_listing",
        "boost_listing",
        "reply_to_buyer",
        "follow_up_buyer",
        "mark_item_sold",
        "update_price",
        "relist_item",
    ] {
        let parsed = ActionType::parse(t).expect(t);
        assert_eq!(parsed.as_str(), t);
    }
    assert!(ActionType::parse("unknown_action").is_none());
}

#[test]
fn action_record_schema_key_is_canonical() {
    // Locks the wire-format schema_key. If this ever drifts,
    // operators with persisted action rows on disk would see
    // them invisible to the new build.
    assert_eq!(ActionRecord::SCHEMA_KEY, "worker_action");
}

/// Round-trip an ActionRecord through JSON to confirm serde
/// renames + defaults match the on-disk shape that #729+ will
/// write.
#[test]
fn action_record_round_trips_through_json_with_defaults() {
    let record = ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.into(),
        action_id: "019df-test".into(),
        job_id: "019df-job".into(),
        worker_id: argo_identity::WorkerId::nil(),
        item_id: Some("skates-110mm".into()),
        action_type: ActionType::CreateListingDraft.as_str().into(),
        risk_level: "low".into(),
        requires_approval: false,
        state: ActionState::Pending.as_str().into(),
        payload: serde_json::json!({"title":"Pro Skates","body":"..."}),
        approved_by: None,
        approved_at: None,
        executed_at: None,
        platform: Some("facebook_marketplace".into()),
        result: None,
        evidence: vec!["https://example.com/comp".into()],
        blocked_reason: None,
        created_at: "2026-05-05T15:00:00Z".into(),
    };
    let s = serde_json::to_string(&record).unwrap();
    let parsed: ActionRecord = serde_json::from_str(&s).unwrap();
    assert_eq!(parsed.action_id, "019df-test");
    assert_eq!(parsed.action_type, "create_listing_draft");
    assert_eq!(parsed.state, "pending");
    assert_eq!(parsed.platform.as_deref(), Some("facebook_marketplace"));
    assert_eq!(parsed.evidence.len(), 1);
    // Optional fields parse cleanly when absent on disk.
    let minimal = r#"{
        "schema_key":"worker_action","action_id":"a","job_id":"j",
        "action_type":"verify_price","state":"pending",
        "payload":{},"created_at":"2026-05-05T00:00:00Z"
    }"#;
    let m: ActionRecord = serde_json::from_str(minimal).unwrap();
    assert!(m.item_id.is_none() && m.platform.is_none() && m.result.is_none());
    assert!(m.evidence.is_empty());
}

#[test]
fn short_truncates_long_action_ids() {
    assert_eq!(
        short("019df67d-4b97-7060-b44e-12eb96508d9a")
            .chars()
            .count(),
        24
    );
    assert_eq!(short("short"), "short");
}

// ─────────────────── #729 emitter tests ────────────────────

/// Synthetic vertical for the emitter tests. Mirrors the
/// gtm-engineer manifest's safety boundary at small scale.
fn synthetic_vertical() -> argo_vertical::VerticalManifest {
    let toml = r#"
schema_version = "1"

[vertical]
id          = "syn"
name        = "Synthetic"
description = "Test fixture for the action emitter."
version     = "0.0.1"

[actions]

[actions.verify_price]
risk              = "low"
requires_approval = false
description       = "Cross-check price."
executor          = "human"

[actions.post_listing]
risk              = "medium"
requires_approval = true
description       = "Public marketplace post."
executor          = "human"
platforms         = ["facebook_marketplace", "ebay"]

[actions.reply_to_buyer]
risk              = "high"
requires_approval = true
description       = "Touches another human."
executor          = "human"
"#;
    argo_vertical::parse(toml).expect("manifest parses")
}

#[test]
fn classify_for_emission_returns_safe_autonomous_for_low_no_approval() {
    let v = synthetic_vertical();
    assert_eq!(
        classify_for_emission(&v, "verify_price"),
        ApprovalCategory::SafeAutonomous,
    );
}

#[test]
fn classify_for_emission_returns_approval_for_required_action() {
    let v = synthetic_vertical();
    assert_eq!(
        classify_for_emission(&v, "post_listing"),
        ApprovalCategory::Approval,
    );
    assert_eq!(
        classify_for_emission(&v, "reply_to_buyer"),
        ApprovalCategory::Approval,
    );
}

/// Locks the SKYCOT safety contract: an action_type the
/// vertical hasn't declared MUST NOT autonomously execute. The
/// emitter persists it as `blocked` so the operator sees the
/// rejection in the ledger rather than nothing.
#[test]
fn classify_for_emission_returns_forbidden_for_undeclared_action_type() {
    let v = synthetic_vertical();
    assert_eq!(
        classify_for_emission(&v, "post_to_tiktok"),
        ApprovalCategory::Forbidden,
    );
    assert_eq!(
        classify_for_emission(&v, "drain_bank_account"),
        ApprovalCategory::Forbidden,
    );
}

#[test]
fn classify_for_emission_returns_forbidden_when_no_actions_section_declared() {
    // v2-style vertical (no [actions] section) — emission is opt-in.
    // Every emission against such a vertical is forbidden until the
    // operator declares the vocabulary.
    let toml = r#"
schema_version = "1"

[vertical]
id      = "v2-style"
name    = "v2 style"
version = "0.0.1"
"#;
    let v = argo_vertical::parse(toml).expect("parses");
    assert_eq!(
        classify_for_emission(&v, "verify_price"),
        ApprovalCategory::Forbidden,
    );
}

/// `build_emission` stamps `risk_level` + `requires_approval`
/// from the manifest. The model can't override either field
/// because they're not even read from the inputs — they're
/// looked up from the vertical declaration.
#[test]
fn build_emission_stamps_risk_and_requires_approval_from_manifest() {
    let v = synthetic_vertical();
    let inputs = ActionEmissionInputs {
        vertical: &v,
        job_id: "job-1",
        worker_id: argo_identity::WorkerId::nil(),
        action_type: "post_listing",
        payload: serde_json::json!({"title": "x"}),
        item_id: Some("item-1"),
        platform: Some("facebook_marketplace"),
        evidence: vec!["https://example.com/1".into()],
        risk_override: None,
    };
    let rec = build_emission(&inputs);
    assert_eq!(rec.action_type, "post_listing");
    assert_eq!(rec.state, "pending");
    assert_eq!(rec.risk_level, "medium");
    assert!(rec.requires_approval);
    assert_eq!(rec.platform.as_deref(), Some("facebook_marketplace"));
    assert_eq!(rec.item_id.as_deref(), Some("item-1"));
    assert_eq!(rec.evidence.len(), 1);
    assert!(rec.blocked_reason.is_none());
}

/// Operator-specific risk override (Stream
/// `mission-operator-specific-risk-overrides`) supersedes the manifest's
/// declared `risk` when populated. The resolver enforces upgrade-only
/// semantics + downgrade gating BEFORE we get here; `build_emission`
/// trusts the pre-resolved level and stamps it onto the row.
#[test]
fn build_emission_applies_risk_override_when_present() {
    let v = synthetic_vertical();
    let inputs = ActionEmissionInputs {
        vertical: &v,
        job_id: "job-1",
        worker_id: argo_identity::WorkerId::nil(),
        action_type: "post_listing",
        payload: serde_json::json!({"title": "x"}),
        item_id: None,
        platform: None,
        evidence: vec![],
        risk_override: Some(RiskOverride {
            risk_level: "high".to_string(),
            trace: serde_json::json!({
                "default": "medium",
                "override_applied": "high",
                "override_scope": "AllVerticals",
            }),
        }),
    };
    let rec = build_emission(&inputs);
    assert_eq!(rec.risk_level, "high", "override should win over manifest");
    assert!(rec.requires_approval);
}

#[test]
fn build_emission_for_safe_autonomous_lands_pending_low_no_approval() {
    let v = synthetic_vertical();
    let inputs = ActionEmissionInputs {
        vertical: &v,
        job_id: "job-1",
        worker_id: argo_identity::WorkerId::nil(),
        action_type: "verify_price",
        payload: serde_json::json!({"target": 250}),
        item_id: None,
        platform: None,
        evidence: vec![],
        risk_override: None,
    };
    let rec = build_emission(&inputs);
    assert_eq!(rec.state, "pending");
    assert_eq!(rec.risk_level, "low");
    assert!(!rec.requires_approval);
    assert!(rec.blocked_reason.is_none());
}

#[test]
fn build_emission_for_high_risk_action_stamps_high_and_requires_approval() {
    let v = synthetic_vertical();
    let inputs = ActionEmissionInputs {
        vertical: &v,
        job_id: "job-1",
        worker_id: argo_identity::WorkerId::nil(),
        action_type: "reply_to_buyer",
        payload: serde_json::json!({"message": "hi"}),
        item_id: None,
        platform: None,
        evidence: vec![],
        risk_override: None,
    };
    let rec = build_emission(&inputs);
    assert_eq!(rec.risk_level, "high");
    assert!(rec.requires_approval);
}

/// Locks the boundary at the persistence layer. An emission
/// for an undeclared action_type lands as `blocked`, with
/// `blocked_reason` naming the gap. The OS NEVER persists a
/// pending row for an undeclared action.
#[test]
fn build_emission_for_undeclared_action_lands_blocked_with_reason() {
    let v = synthetic_vertical();
    let inputs = ActionEmissionInputs {
        vertical: &v,
        job_id: "job-1",
        worker_id: argo_identity::WorkerId::nil(),
        action_type: "drain_bank_account",
        payload: serde_json::json!({"amount": 99999}),
        item_id: None,
        platform: None,
        evidence: vec![],
        risk_override: None,
    };
    let rec = build_emission(&inputs);
    assert_eq!(rec.state, "blocked");
    assert!(!rec.requires_approval);
    let reason = rec.blocked_reason.expect("blocked_reason populated");
    assert!(
        reason.contains("drain_bank_account") && reason.contains("syn"),
        "blocked_reason must name the rejected action AND the vertical; got: {reason}"
    );
}

/// Locks that the production gtm-engineer manifest (the one
/// the worker actually loads) declares the autonomous-safe
/// action types correctly. A regression here would let the
/// worker accidentally require approval for `verify_price` —
/// or, worse, autonomously fire `post_listing`.
#[test]
fn gtm_engineer_manifest_declares_expected_action_classification() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("verticals");
    let (registry, errors) = argo_vertical::load_dir(&dir).expect("load verticals dir");
    assert!(errors.is_empty(), "loader produced errors: {errors:?}");
    let v = registry.get("gtm-engineer").expect("gtm-engineer present");
    let manifest = &v.manifest;

    // Autonomous-safe set:
    for safe in ["create_listing_draft", "verify_price", "request_photos"] {
        assert_eq!(
            classify_for_emission(manifest, safe),
            ApprovalCategory::SafeAutonomous,
            "{safe} must be SafeAutonomous in the production gtm-engineer manifest"
        );
    }
    // Approval-gated set (the one that touches money / public surfaces / buyers):
    for gated in [
        "post_listing",
        "boost_listing",
        "reply_to_buyer",
        "follow_up_buyer",
        "mark_item_sold",
        "update_price",
        "relist_item",
    ] {
        assert_eq!(
            classify_for_emission(manifest, gated),
            ApprovalCategory::Approval,
            "{gated} must be Approval-gated in the production gtm-engineer manifest"
        );
    }
    // Anything not declared is forbidden:
    assert_eq!(
        classify_for_emission(manifest, "post_to_tiktok"),
        ApprovalCategory::Forbidden,
    );
}
