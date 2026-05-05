//! Unit tests for `worker_actions` (#728). Extracted from the
//! main module to keep `handlers/worker_actions.rs` under the
//! 500-LOC budget.
//!
//! Loaded via `#[cfg(test)] #[path = "worker_actions_tests.rs"]
//! mod tests;` at the bottom of `worker_actions.rs`.

use super::*;

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
    assert!(Approved.can_transition_to(Executing));
    assert!(Approved.can_transition_to(Completed));
    assert!(Executing.can_transition_to(Completed));
    assert!(Executing.can_transition_to(Failed));
    // Illegal — these MUST NOT be allowed by the writer:
    assert!(!Pending.can_transition_to(Executing), "must approve first");
    assert!(!Pending.can_transition_to(Completed), "must approve first");
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
        item_id: Some("skates-110mm".into()),
        action_type: ActionType::CreateListingDraft.as_str().into(),
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
