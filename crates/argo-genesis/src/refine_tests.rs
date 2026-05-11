//! Stream `claude-genesis-llm-refinement` tests. Loaded from
//! `refine.rs` via `#[cfg(test)] #[path = ...] mod tests;` so
//! the module file stays under the per-file 500-LOC budget.

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
async fn malformed_json_response_falls_back_to_input_plan() {
    let caller = NoopCaller {
        response: "{ this is not valid JSON".into(),
    };
    let original = sample_plan();
    let original_action_count = original
        .manifest
        .actions
        .as_ref()
        .map(|a| a.actions.len())
        .unwrap_or(0);
    let refined = refine_plan(original, &caller).await.expect("refine");
    let refined_action_count = refined
        .manifest
        .actions
        .as_ref()
        .map(|a| a.actions.len())
        .unwrap_or(0);
    assert_eq!(
        original_action_count, refined_action_count,
        "malformed JSON must fall back unchanged"
    );
}

#[tokio::test]
async fn caller_error_falls_back_to_input_plan() {
    struct ErrorCaller;
    #[async_trait::async_trait]
    impl AnthropicCaller for ErrorCaller {
        async fn call(&self, _prompt: String) -> Result<String, GenesisError> {
            Err(GenesisError::Validation {
                stage: "test",
                detail: "synthetic failure".into(),
            })
        }
    }
    let original = sample_plan();
    let original_id = original.vertical_id.clone();
    let refined = refine_plan(original, &ErrorCaller).await.expect("refine");
    assert_eq!(refined.vertical_id, original_id);
}

#[tokio::test]
async fn valid_response_appends_addendum_and_merges_actions() {
    let response = serde_json::json!({
        "archetype_confirmed": true,
        "additional_actions": [
            {
                "action_type": "draft_buyer_response",
                "risk": "medium",
                "requires_approval": false,
                "description": "Draft a response to a buyer enquiry",
            }
        ],
        "system_prompt_addendum": "Focus on weekly cadence: list Mon, follow up Thu.",
    });
    let caller = NoopCaller {
        response: response.to_string(),
    };
    let original = sample_plan();
    let original_count = original.manifest.actions.as_ref().unwrap().actions.len();
    let refined = refine_plan(original, &caller).await.expect("refine");
    let actions = refined.manifest.actions.as_ref().unwrap();
    assert_eq!(actions.actions.len(), original_count + 1);
    assert!(actions.actions.contains_key("draft_buyer_response"));
    assert!(refined
        .manifest
        .vertical
        .description
        .contains("weekly cadence"));
}

#[tokio::test]
async fn safety_judge_re_runs_on_refined_actions() {
    // Model proposes a sensitive verb action with
    // requires_approval=false. The auto-fix pass should flip it
    // to true before the judge runs; the manifest must end up
    // safe.
    let response = serde_json::json!({
        "archetype_confirmed": true,
        "additional_actions": [
            {
                "action_type": "send_invoice",
                "risk": "high",
                "requires_approval": false,
                "description": "Send a buyer the invoice",
            }
        ],
        "system_prompt_addendum": "",
    });
    let caller = NoopCaller {
        response: response.to_string(),
    };
    let refined = refine_plan(sample_plan(), &caller).await.expect("refine");
    let actions = refined.manifest.actions.as_ref().unwrap();
    let send_invoice = actions
        .actions
        .get("send_invoice")
        .expect("send_invoice merged");
    // Auto-fix promoted requires_approval from false → true
    // because the action_type contains the sensitive verb "send".
    assert!(
        send_invoice.requires_approval,
        "sensitive verb must be auto-fixed to require approval"
    );
}

#[tokio::test]
async fn five_additional_actions_all_land() {
    // Old behaviour capped additional_actions at 3 (over-cap
    // responses were rejected as a parse error). The new
    // default cap is RefineConfig::DEFAULT_MAX_NEW_ACTIONS (8),
    // so 5 actions must all land in the manifest.
    let response = serde_json::json!({
        "archetype_confirmed": true,
        "additional_actions": [
            { "action_type": "x1", "risk": "low", "requires_approval": false, "description": "x" },
            { "action_type": "x2", "risk": "low", "requires_approval": false, "description": "x" },
            { "action_type": "x3", "risk": "low", "requires_approval": false, "description": "x" },
            { "action_type": "x4", "risk": "low", "requires_approval": false, "description": "x" },
            { "action_type": "x5", "risk": "low", "requires_approval": false, "description": "x" },
        ],
        "system_prompt_addendum": "",
    });
    let caller = NoopCaller {
        response: response.to_string(),
    };
    let original_count = sample_plan().manifest.actions.as_ref().unwrap().actions.len();
    let refined = refine_plan(sample_plan(), &caller).await.expect("refine");
    let refined_count = refined.manifest.actions.as_ref().unwrap().actions.len();
    assert_eq!(
        refined_count,
        original_count + 5,
        "all 5 proposed actions should land under the default cap of 8"
    );
    let actions = refined.manifest.actions.as_ref().unwrap();
    for name in ["x1", "x2", "x3", "x4", "x5"] {
        assert!(actions.actions.contains_key(name), "{name} merged");
    }
}

#[tokio::test]
async fn configurable_cap_truncates_excess_actions() {
    // Set the env var so refine_plan reads max_new_actions=2,
    // then send 5 proposed actions. Exactly 2 must land.
    let prev = std::env::var("ARGO_GENESIS_REFINE_MAX_ACTIONS").ok();
    std::env::set_var("ARGO_GENESIS_REFINE_MAX_ACTIONS", "2");
    let response = serde_json::json!({
        "archetype_confirmed": true,
        "additional_actions": [
            { "action_type": "y1", "risk": "low", "requires_approval": false, "description": "y" },
            { "action_type": "y2", "risk": "low", "requires_approval": false, "description": "y" },
            { "action_type": "y3", "risk": "low", "requires_approval": false, "description": "y" },
            { "action_type": "y4", "risk": "low", "requires_approval": false, "description": "y" },
            { "action_type": "y5", "risk": "low", "requires_approval": false, "description": "y" },
        ],
        "system_prompt_addendum": "",
    });
    let caller = NoopCaller {
        response: response.to_string(),
    };
    let original_count = sample_plan().manifest.actions.as_ref().unwrap().actions.len();
    let result = refine_plan(sample_plan(), &caller).await;
    match prev {
        Some(v) => std::env::set_var("ARGO_GENESIS_REFINE_MAX_ACTIONS", v),
        None => std::env::remove_var("ARGO_GENESIS_REFINE_MAX_ACTIONS"),
    }
    let refined = result.expect("refine");
    let refined_count = refined.manifest.actions.as_ref().unwrap().actions.len();
    assert_eq!(
        refined_count,
        original_count + 2,
        "configurable cap must truncate 5 → 2"
    );
}

#[tokio::test]
async fn safety_judge_fires_on_refined_sensitive_verb_with_uncapped_actions() {
    // Lifting the count cap must NOT lift the safety gate. With
    // 5 proposed actions (all of which land under the default
    // cap of 8), the sensitive-verb one must still get its
    // requires_approval flag auto-fixed to true.
    let response = serde_json::json!({
        "archetype_confirmed": true,
        "additional_actions": [
            { "action_type": "draft_summary", "risk": "low", "requires_approval": false, "description": "Draft a summary" },
            { "action_type": "send_invoice", "risk": "high", "requires_approval": false, "description": "Send the buyer the invoice" },
            { "action_type": "review_listing", "risk": "low", "requires_approval": false, "description": "Review a listing" },
            { "action_type": "tag_inventory", "risk": "low", "requires_approval": false, "description": "Tag inventory" },
            { "action_type": "summarise_metrics", "risk": "low", "requires_approval": false, "description": "Summarise metrics" },
        ],
        "system_prompt_addendum": "",
    });
    let caller = NoopCaller {
        response: response.to_string(),
    };
    let refined = refine_plan(sample_plan(), &caller).await.expect("refine");
    let actions = refined.manifest.actions.as_ref().unwrap();
    assert!(
        actions.actions.contains_key("send_invoice"),
        "sensitive action still merges"
    );
    let send_invoice = actions.actions.get("send_invoice").unwrap();
    assert!(
        send_invoice.requires_approval,
        "safety gate must still auto-fix sensitive verbs even with cap lifted"
    );
    // And the non-sensitive actions are not gratuitously gated.
    let draft_summary = actions.actions.get("draft_summary").unwrap();
    assert!(!draft_summary.requires_approval);
}

#[test]
fn refine_config_default_max_new_actions_is_eight() {
    assert_eq!(RefineConfig::DEFAULT_MAX_NEW_ACTIONS, 8);
}

#[tokio::test]
async fn over_long_addendum_falls_back() {
    let response = serde_json::json!({
        "archetype_confirmed": true,
        "additional_actions": [],
        "system_prompt_addendum": "x".repeat(5000),
    });
    let caller = NoopCaller {
        response: response.to_string(),
    };
    let original = sample_plan();
    let original_desc = original.manifest.vertical.description.clone();
    let refined = refine_plan(original, &caller).await.expect("refine");
    assert_eq!(
        refined.manifest.vertical.description, original_desc,
        "over-long addendum must fall back unchanged"
    );
}

#[test]
fn refine_config_default_model_matches_brief() {
    assert_eq!(RefineConfig::DEFAULT_MODEL, "claude-sonnet-4-6");
}

#[test]
fn refine_config_from_env_errors_when_api_key_missing() {
    let prev = std::env::var("ANTHROPIC_API_KEY").ok();
    std::env::remove_var("ANTHROPIC_API_KEY");
    let result = RefineConfig::from_env();
    if let Some(v) = prev {
        std::env::set_var("ANTHROPIC_API_KEY", v);
    }
    let err = result.expect_err("missing API key");
    let msg = err.to_string();
    assert!(msg.contains("ANTHROPIC_API_KEY"), "got: {msg}");
}

#[test]
fn unknown_field_in_response_is_rejected() {
    // `deny_unknown_fields` should reject extra keys.
    let raw = serde_json::json!({
        "archetype_confirmed": true,
        "additional_actions": [],
        "system_prompt_addendum": "",
        "extra_field_the_model_invented": "junk",
    });
    let parsed: Result<RefinementResponse, _> = serde_json::from_value(raw);
    assert!(parsed.is_err(), "deny_unknown_fields must reject");
}
