//! Unit tests for the broader-generation path. See
//! `mission-genesis-non-archetype-roles` acceptance criteria for the
//! list of cases.

use super::*;
use crate::archetype::{Archetype, ClassificationResult};
use argo_types::{ArgoError, ArgoResult};
use std::sync::Mutex;

const CANNED_BROAD_RESPONSE: &str = r#"{
  "persona_description": "A calm, observant chief vibes officer who keeps team energy positive.",
  "action_vocabulary": ["check_in", "draft_kudos", "schedule_offsite", "summarise_mood"],
  "success_metrics": ["weekly_vibe_score_up", "kudos_sent_per_week", "team_nps_quarterly"],
  "onboarding_script": "Hi team! I'm Vibe, your new chief vibes officer. I'll be checking in with each of you weekly to keep our team energy positive and surface anything worth celebrating."
}"#;

struct MockBroadClient {
    response: String,
    last_prompt: Mutex<Option<String>>,
}

impl MockBroadClient {
    fn new(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
            last_prompt: Mutex::new(None),
        }
    }
}

#[async_trait::async_trait]
impl BroadRoleClient for MockBroadClient {
    async fn synthesize(&self, prompt: &str) -> ArgoResult<String> {
        if let Ok(mut slot) = self.last_prompt.lock() {
            *slot = Some(prompt.to_string());
        }
        Ok(self.response.clone())
    }
}

struct FailingBroadClient;

#[async_trait::async_trait]
impl BroadRoleClient for FailingBroadClient {
    async fn synthesize(&self, _prompt: &str) -> ArgoResult<String> {
        Err(ArgoError::Provider("simulated transport failure".into()))
    }
}

#[test]
fn low_confidence_triggers_broad() {
    let low = ClassificationResult {
        archetype: Some(Archetype::MarketingIntern),
        confidence: 0.4,
    };
    assert!(should_use_broad_role(&low), "0.4 < 0.6 must route to broad");

    let none_match = ClassificationResult {
        archetype: None,
        confidence: 0.9,
    };
    assert!(
        should_use_broad_role(&none_match),
        "archetype=None must route to broad regardless of confidence",
    );
}

#[test]
fn high_confidence_stays_archetype() {
    let high = ClassificationResult {
        archetype: Some(Archetype::MarketingIntern),
        confidence: 0.85,
    };
    assert!(
        !should_use_broad_role(&high),
        "0.85 ≥ 0.6 must keep archetype path",
    );

    let boundary = ClassificationResult {
        archetype: Some(Archetype::SalesSdr),
        confidence: BROAD_ROLE_CONFIDENCE_THRESHOLD,
    };
    assert!(
        !should_use_broad_role(&boundary),
        "boundary (== threshold) must keep archetype path",
    );
}

#[test]
fn broad_output_sentinel_archetype_broad() {
    let draft = BroadRoleDraft {
        charter: "x".into(),
        persona: "p".into(),
        persona_description: "d".into(),
        action_vocabulary: vec!["a".into()],
        success_metrics: vec![],
        onboarding_script: String::new(),
    };
    assert_eq!(draft.archetype_slug(), "broad");
    assert_eq!(draft.archetype_slug(), BROAD_SENTINEL);
}

#[test]
fn broad_prompt_asks_3_8_actions() {
    let prompt = build_broad_role_prompt("be my chief vibes officer", "vibe");
    assert!(
        prompt.contains("3 to 8"),
        "prompt must ask for 3-8 actions; got: {prompt}",
    );
    assert!(prompt.contains("action_vocabulary"), "prompt must mention action_vocabulary");
    assert!(prompt.contains("Charter: be my chief vibes officer"));
}

#[tokio::test]
async fn broad_empty_charter_errors() {
    let client = MockBroadClient::new(CANNED_BROAD_RESPONSE);
    let err = genesis_broad_role("   ", "vibe", &client).await.expect_err("empty charter");
    assert!(
        err.to_string().contains("charter is empty"),
        "expected empty-charter error, got: {err}",
    );
}

#[tokio::test]
async fn broader_roundtrip_via_mock_provider() {
    let client = MockBroadClient::new(CANNED_BROAD_RESPONSE);
    let draft = genesis_broad_role(
        "be my chief vibes officer, keep the team energy positive",
        "vibe",
        &client,
    )
    .await
    .expect("mock client returns canned JSON");
    assert_eq!(draft.archetype_slug(), "broad");
    assert_eq!(draft.persona, "vibe");
    assert!(draft.persona_description.contains("chief vibes officer"));
    assert!(draft.action_vocabulary.len() >= 3 && draft.action_vocabulary.len() <= 8);
    assert!(draft.action_vocabulary.contains(&"check_in".to_string()));
    assert!(draft.success_metrics.len() >= 3 && draft.success_metrics.len() <= 5);
    assert!(!draft.onboarding_script.is_empty());
}

#[tokio::test]
async fn broad_propagates_provider_error() {
    // 7th test (bonus): a provider error surfaces as ArgoError::Provider
    // rather than crashing. The pipeline-level "conservative Block on
    // judge error" is tested in the genesis_judge_tests module.
    let client = FailingBroadClient;
    let err = genesis_broad_role("be my chief vibes officer", "vibe", &client)
        .await
        .expect_err("provider error must propagate");
    assert!(matches!(err, ArgoError::Provider(_)), "got: {err:?}");
}

#[test]
fn prepend_persona_disclaimer_mutates_description() {
    let mut draft = BroadRoleDraft {
        charter: "x".into(),
        persona: "p".into(),
        persona_description: "Original persona text.".into(),
        action_vocabulary: vec![],
        success_metrics: vec![],
        onboarding_script: String::new(),
    };
    draft.prepend_persona_disclaimer("not a real doctor");
    assert!(draft.persona_description.starts_with("[DISCLAIMER: not a real doctor]"));
    assert!(draft.persona_description.contains("Original persona text."));
}

#[test]
fn parse_broad_role_response_tolerates_prose_around_json() {
    let raw = format!("Sure! Here you go:\n{CANNED_BROAD_RESPONSE}\nLet me know if anything is off.");
    let parsed = parse_broad_role_response(&raw).expect("prose-wrapped JSON parses");
    assert_eq!(parsed.action_vocabulary.len(), 4);
}

#[test]
fn parse_broad_role_response_errors_when_required_fields_missing() {
    let raw = r#"{"persona_description": "", "action_vocabulary": []}"#;
    let err = parse_broad_role_response(raw).expect_err("empty fields");
    assert!(matches!(err, ArgoError::Provider(_)));
}

// ---- Integration smoke (brief acceptance criteria) ----

use crate::archetype::ProviderClient;
use crate::genesis_judge::GenesisJudgeClient;
use crate::pipeline::GenesisConfig;

struct LowConfidenceLlm;

#[async_trait::async_trait]
impl ProviderClient for LowConfidenceLlm {
    async fn classify(&self, _prompt: &str) -> ArgoResult<String> {
        // "none 0.1" — model declines to pick an archetype, very low
        // confidence. Triggers the broad-role path.
        Ok("none 0.1".to_string())
    }
}

struct HighConfidenceLlm;

#[async_trait::async_trait]
impl ProviderClient for HighConfidenceLlm {
    async fn classify(&self, _prompt: &str) -> ArgoResult<String> {
        Ok("marketing_intern 0.95".to_string())
    }
}

struct AllowJudge;

#[async_trait::async_trait]
impl GenesisJudgeClient for AllowJudge {
    async fn judge(&self, _prompt: &str) -> ArgoResult<String> {
        Ok(r#"{"verdict": "allow"}"#.to_string())
    }
}

#[tokio::test]
async fn smoke_low_confidence_routes_to_broad_via_mock_providers() {
    let llm = LowConfidenceLlm;
    let broad = MockBroadClient::new(CANNED_BROAD_RESPONSE);
    let judge = AllowJudge;
    let config = GenesisConfig { use_llm_classifier: true };
    let outcome = plan_or_broad(
        "be my chief vibes officer, keep the team energy positive",
        "vibe",
        &config,
        Some(&llm),
        Some(&broad),
        Some(&judge),
        /* allow_broad_role = */ true,
    )
    .await
    .expect("plan_or_broad runs end-to-end");
    match outcome {
        crate::broad_role::GenesisOutcome::Broad(draft) => {
            assert_eq!(draft.archetype_slug(), "broad");
            assert!(draft.action_vocabulary.contains(&"check_in".to_string()));
        }
        other => panic!("expected Broad outcome, got {other:?}"),
    }
}

#[tokio::test]
async fn smoke_low_confidence_without_flag_errors_with_flag_mention() {
    let llm = LowConfidenceLlm;
    let broad = MockBroadClient::new(CANNED_BROAD_RESPONSE);
    let judge = AllowJudge;
    let config = GenesisConfig { use_llm_classifier: true };
    let err = plan_or_broad(
        "be my chief vibes officer, keep the team energy positive",
        "vibe",
        &config,
        Some(&llm),
        Some(&broad),
        Some(&judge),
        /* allow_broad_role = */ false,
    )
    .await
    .expect_err("low-confidence + no flag must error");
    assert!(
        err.to_string().contains("--allow-broad-role"),
        "error must mention the flag; got: {err}",
    );
}

#[tokio::test]
async fn smoke_high_confidence_stays_archetype_path() {
    let llm = HighConfidenceLlm;
    let broad = MockBroadClient::new(CANNED_BROAD_RESPONSE);
    let judge = AllowJudge;
    let config = GenesisConfig { use_llm_classifier: true };
    let outcome = plan_or_broad(
        "be my marketing intern, run weekly campaigns to cold leads",
        "maya",
        &config,
        Some(&llm),
        Some(&broad),
        Some(&judge),
        /* allow_broad_role = */ true,
    )
    .await
    .expect("archetype path runs");
    match outcome {
        crate::broad_role::GenesisOutcome::Plan(plan) => {
            assert_eq!(plan.archetype, Archetype::MarketingIntern);
        }
        other => panic!("expected Plan outcome, got {other:?}"),
    }
}

#[tokio::test]
async fn smoke_judge_block_errors_with_reason() {
    struct BlockingJudge;
    #[async_trait::async_trait]
    impl GenesisJudgeClient for BlockingJudge {
        async fn judge(&self, _prompt: &str) -> ArgoResult<String> {
            Ok(r#"{"verdict": "block", "reason": "impersonates a real person"}"#.to_string())
        }
    }
    let llm = HighConfidenceLlm;
    let broad = MockBroadClient::new(CANNED_BROAD_RESPONSE);
    let judge = BlockingJudge;
    let config = GenesisConfig { use_llm_classifier: true };
    let err = plan_or_broad(
        "be my marketing intern, run weekly campaigns to cold leads",
        "maya",
        &config,
        Some(&llm),
        Some(&broad),
        Some(&judge),
        true,
    )
    .await
    .expect_err("judge block must error");
    assert!(err.to_string().contains("impersonates"), "got: {err}");
}

#[tokio::test]
async fn smoke_judge_allow_with_disclaimer_mutates_broad_draft() {
    struct DisclaimerJudge;
    #[async_trait::async_trait]
    impl GenesisJudgeClient for DisclaimerJudge {
        async fn judge(&self, _prompt: &str) -> ArgoResult<String> {
            Ok(r#"{"verdict": "allow_with_disclaimer", "reason": "not a licensed therapist"}"#
                .into())
        }
    }
    let llm = LowConfidenceLlm;
    let broad = MockBroadClient::new(CANNED_BROAD_RESPONSE);
    let judge = DisclaimerJudge;
    let config = GenesisConfig { use_llm_classifier: true };
    let outcome = plan_or_broad(
        "be my chief vibes officer", "vibe", &config,
        Some(&llm), Some(&broad), Some(&judge),
        true,
    )
    .await
    .expect("disclaimer path runs");
    match outcome {
        crate::broad_role::GenesisOutcome::Broad(draft) => {
            assert!(
                draft.persona_description.starts_with("[DISCLAIMER: not a licensed therapist]"),
                "disclaimer must prepend; got: {}",
                draft.persona_description,
            );
        }
        other => panic!("expected Broad outcome, got {other:?}"),
    }
}
