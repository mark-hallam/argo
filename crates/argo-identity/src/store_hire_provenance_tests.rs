//! Provenance-chain stamping tests for Genesis hires (DEC-025 #2 + #6 /
//! Genesis P3 + P5). Extracted from `store_hire.rs` to keep that file under
//! the 500-LOC ceiling; declared via `#[path] mod probation_tests;`.

use super::*;

fn draft_with_plan(plan: Option<&str>) -> HireDraft {
    HireDraft {
        suggested_name: "Maya".into(),
        charter: "Own outbound marketplace follow-up end-to-end.".into(),
        suggested_vertical: "marketplace-seller".into(),
        approval_policy: ApprovalPolicyChoice {
            level: "balanced".into(),
        },
        duties: vec![HireDutyDraft {
            cron: "0 9 * * 1-5".into(),
            entry_action: "follow_up".into(),
            description: "Send approved follow-up.".into(),
            default_enabled: true,
        }],
        voice_profile: None,
        tagline: None,
        genesis_plan_id: plan.map(str::to_string),
    }
}

#[test]
fn genesis_hire_stamps_plan_probation_and_truth_boundary_markers() {
    let now = Utc::now();
    let chain = build_provenance_chain(Some("marketing-intern-maya"), now);
    assert_eq!(
        genesis_plan_id_from_provenance(&chain).as_deref(),
        Some("marketing-intern-maya")
    );
    let until = probation_until_from_provenance(&chain).expect("probation marker present");
    // Pinned exactly PROBATION_HOURS after `now`.
    assert_eq!((until - now).num_hours(), PROBATION_HOURS);
    // A freshly-hired teammate is in probation right now.
    assert!(is_in_probation(&chain, now));
    // P5 (a): the conservative default truth-boundary policy is bound.
    assert_eq!(
        truth_boundary_policy_id_from_provenance(&chain).as_deref(),
        Some(GENESIS_DEFAULT_TRUTH_BOUNDARY_POLICY_ID),
    );
}

#[test]
fn non_genesis_hire_has_no_probation_window_or_truth_boundary() {
    // No Genesis plan id ⇒ empty chain ⇒ never in probation, no bound
    // claim policy. A hand-authored / pre-Genesis worker is governed by
    // its ordinary policy, not gated.
    let chain = build_provenance_chain(None, Utc::now());
    assert!(chain.is_empty());
    assert!(!is_in_probation(&chain, Utc::now()));
    assert!(probation_until_from_provenance(&chain).is_none());
    assert!(truth_boundary_policy_id_from_provenance(&chain).is_none());
    // Blank / whitespace plan id is treated as no plan id.
    assert!(build_provenance_chain(Some("   "), Utc::now()).is_empty());
}

#[test]
fn probation_graduates_after_the_window() {
    let now = Utc::now();
    let chain = build_provenance_chain(Some("p"), now);
    // Just before the boundary → still gated.
    let pre = now + Duration::hours(PROBATION_HOURS) - Duration::minutes(1);
    assert!(is_in_probation(&chain, pre));
    // After the boundary → graduated (normal policy).
    let post = now + Duration::hours(PROBATION_HOURS) + Duration::minutes(1);
    assert!(!is_in_probation(&chain, post));
}

#[test]
fn both_hire_paths_stamp_probation_via_the_single_chokepoint() {
    // Every Genesis hire (flagship SSE + /onboarding/first-hire) builds
    // its `HireDraft` with `genesis_plan_id: Some(..)` and funnels
    // through `assemble_worker_from_draft`. Proving the assembled
    // persona carries the probation marker proves neither path can
    // mint a Genesis worker that skips probation.
    let worker = assemble_worker_from_draft(
        &draft_with_plan(Some("marketing-intern-maya")),
        TenantId::new(),
        "human:founder@acme.test",
    );
    assert!(
        is_in_probation(&worker.persona.provenance_chain, Utc::now()),
        "a Genesis-hired worker must start in probation"
    );

    // The fallback (no plan id) path is NOT a generated teammate and
    // carries no probation window.
    let plain = assemble_worker_from_draft(
        &draft_with_plan(None),
        TenantId::new(),
        "human:founder@acme.test",
    );
    assert!(!is_in_probation(&plain.persona.provenance_chain, Utc::now()));
}
