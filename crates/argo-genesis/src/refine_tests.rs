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
async fn over_three_additional_actions_falls_back() {
    // The brief caps additional_actions at 3.
    let response = serde_json::json!({
        "archetype_confirmed": true,
        "additional_actions": [
            { "action_type": "x1", "risk": "low", "requires_approval": false, "description": "x" },
            { "action_type": "x2", "risk": "low", "requires_approval": false, "description": "x" },
            { "action_type": "x3", "risk": "low", "requires_approval": false, "description": "x" },
            { "action_type": "x4", "risk": "low", "requires_approval": false, "description": "x" },
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
        refined_count, original_count,
        "over-cap response must fall back unchanged"
    );
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
