//! #60 mission load-bearing wire — coverage for the
//! vocabulary-evolution path. Sibling of `refine_tests.rs` so
//! both stay under the per-file 500-LOC budget.

use super::*;
use crate::archetype::Archetype;
use crate::manifest_writer::build_manifest;
use crate::pipeline::GenesisPlan;

fn sample_plan() -> GenesisPlan {
    let manifest = build_manifest(
        "be a marketplace seller",
        Archetype::GtmEngineer,
        "marketmax",
        "gtm-engineer-marketmax",
    )
    .expect("manifest builds");
    GenesisPlan {
        charter: "be a marketplace seller".into(),
        persona: "marketmax".into(),
        archetype: Archetype::GtmEngineer,
        vertical_id: "gtm-engineer-marketmax".into(),
        manifest,
    }
}

#[tokio::test]
async fn new_action_type_lands_in_vocab() {
    let response = serde_json::json!({
        "archetype_confirmed": true,
        "additional_actions": [{
            "action_type": "schedule_meeting",
            "risk": "low",
            "requires_approval": false,
            "description": "Book a meeting with the buyer",
        }],
        "system_prompt_addendum": "",
    });
    let caller = NoopCaller {
        response: response.to_string(),
    };
    let original = sample_plan();
    assert!(
        !original
            .manifest
            .actions
            .as_ref()
            .unwrap()
            .actions
            .contains_key("schedule_meeting"),
        "seed vocab must not already contain the proposal"
    );
    let (refined, evolved) =
        refine_plan_with_cap(original, &caller, RefineConfig::DEFAULT_MAX_VOCAB_SIZE)
            .await
            .expect("refine");
    assert!(refined
        .manifest
        .actions
        .as_ref()
        .unwrap()
        .actions
        .contains_key("schedule_meeting"));
    assert_eq!(evolved, vec!["schedule_meeting".to_string()]);
}

#[tokio::test]
async fn new_action_type_rejected_when_cap_reached() {
    // With max_vocab_size set to the existing vocabulary size,
    // the proposed new action_type must be silently dropped.
    let response = serde_json::json!({
        "archetype_confirmed": true,
        "additional_actions": [{
            "action_type": "escalate_to_legal",
            "risk": "low",
            "requires_approval": false,
            "description": "Hand off to the legal team",
        }],
        "system_prompt_addendum": "",
    });
    let caller = NoopCaller {
        response: response.to_string(),
    };
    let original = sample_plan();
    let cap = original.manifest.actions.as_ref().unwrap().actions.len();
    let (refined, evolved) = refine_plan_with_cap(original, &caller, cap)
        .await
        .expect("refine");
    assert!(evolved.is_empty(), "no growth allowed at cap");
    assert_eq!(
        refined.manifest.actions.as_ref().unwrap().actions.len(),
        cap,
        "vocabulary must stay capped"
    );
    assert!(!refined
        .manifest
        .actions
        .as_ref()
        .unwrap()
        .actions
        .contains_key("escalate_to_legal"));
}

#[tokio::test]
async fn sensitive_risk_forces_requires_approval() {
    // Action_type whose name does NOT match a sensitive verb
    // (so safety_judge::auto_fix won't catch it) but whose risk
    // is High must be force-promoted to requires_approval=true
    // by the merge-time defense-in-depth pass.
    let response = serde_json::json!({
        "archetype_confirmed": true,
        "additional_actions": [{
            "action_type": "escalate_to_legal",
            "risk": "high",
            "requires_approval": false,
            "description": "Forward the matter to legal counsel",
        }],
        "system_prompt_addendum": "",
    });
    let caller = NoopCaller {
        response: response.to_string(),
    };
    let (refined, evolved) =
        refine_plan_with_cap(sample_plan(), &caller, RefineConfig::DEFAULT_MAX_VOCAB_SIZE)
            .await
            .expect("refine");
    assert_eq!(evolved, vec!["escalate_to_legal".to_string()]);
    let action = refined
        .manifest
        .actions
        .as_ref()
        .unwrap()
        .actions
        .get("escalate_to_legal")
        .expect("merged");
    assert!(
        action.requires_approval,
        "High-risk action must be force-promoted to requires_approval=true"
    );
}

#[tokio::test]
async fn existing_action_type_still_merges_normally() {
    // Proposing an action_type that already exists must be a
    // no-op (preserve the existing declaration). evolved_vocab
    // must NOT list the duplicate.
    let original = sample_plan();
    let existing_name: String = original
        .manifest
        .actions
        .as_ref()
        .unwrap()
        .actions
        .keys()
        .next()
        .expect("seed vocab non-empty")
        .clone();
    let existing_decl = original
        .manifest
        .actions
        .as_ref()
        .unwrap()
        .actions
        .get(&existing_name)
        .cloned()
        .unwrap();
    let response = serde_json::json!({
        "archetype_confirmed": true,
        "additional_actions": [{
            "action_type": existing_name.clone(),
            "risk": "low",
            "requires_approval": false,
            "description": "DIFFERENT description should NOT overwrite",
        }],
        "system_prompt_addendum": "",
    });
    let caller = NoopCaller {
        response: response.to_string(),
    };
    let original_count = original.manifest.actions.as_ref().unwrap().actions.len();
    let (refined, evolved) =
        refine_plan_with_cap(original, &caller, RefineConfig::DEFAULT_MAX_VOCAB_SIZE)
            .await
            .expect("refine");
    assert!(
        evolved.is_empty(),
        "existing action_type must not appear in evolved_vocab"
    );
    let refined_actions = refined.manifest.actions.as_ref().unwrap();
    assert_eq!(refined_actions.actions.len(), original_count);
    let preserved = refined_actions
        .actions
        .get(&existing_name)
        .expect("existing key still present");
    assert_eq!(
        preserved.description, existing_decl.description,
        "existing declaration must be preserved byte-for-byte"
    );
}

#[tokio::test]
async fn evolved_vocab_field_populated_correctly() {
    // Two new action_types proposed: both should land, and the
    // returned evolved-vocab list should carry exactly those
    // names (in proposal order).
    let response = serde_json::json!({
        "archetype_confirmed": true,
        "additional_actions": [
            {
                "action_type": "draft_buyer_followup",
                "risk": "low",
                "requires_approval": false,
                "description": "Draft a buyer follow-up note",
            },
            {
                "action_type": "log_listing_view",
                "risk": "low",
                "requires_approval": false,
                "description": "Log a buyer viewing a listing",
            },
        ],
        "system_prompt_addendum": "",
    });
    let caller = NoopCaller {
        response: response.to_string(),
    };
    let (_refined, evolved) =
        refine_plan_with_cap(sample_plan(), &caller, RefineConfig::DEFAULT_MAX_VOCAB_SIZE)
            .await
            .expect("refine");
    assert_eq!(
        evolved,
        vec![
            "draft_buyer_followup".to_string(),
            "log_listing_view".to_string(),
        ],
        "evolved_vocab must list new action_types in proposal order"
    );
}
