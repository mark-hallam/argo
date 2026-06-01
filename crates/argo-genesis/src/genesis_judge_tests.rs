//! Unit tests for the Genesis safety judge gate. Mission
//! `mission-genesis-non-archetype-roles` acceptance criteria call
//! for 5 tests; we add an extra for the verdict-parse fallback to
//! defend the "conservative Block on judge crash" semantic.

use super::*;
use crate::broad_role::BroadRoleDraft;
use argo_types::{ArgoError, ArgoResult};

struct CannedJudge {
    response: String,
}

#[async_trait::async_trait]
impl GenesisJudgeClient for CannedJudge {
    async fn judge(&self, _prompt: &str) -> ArgoResult<String> {
        Ok(self.response.clone())
    }
}

struct FailingJudge;

#[async_trait::async_trait]
impl GenesisJudgeClient for FailingJudge {
    async fn judge(&self, _prompt: &str) -> ArgoResult<String> {
        Err(ArgoError::Provider(
            "simulated judge transport crash".into(),
        ))
    }
}

fn sample_draft() -> BroadRoleDraft {
    BroadRoleDraft {
        charter: "be my chief vibes officer".into(),
        persona: "vibe".into(),
        persona_description: "Calm, observant chief vibes officer.".into(),
        action_vocabulary: vec!["check_in".into(), "draft_kudos".into()],
        success_metrics: vec!["weekly_vibe_score_up".into()],
        onboarding_script: "Hi team!".into(),
    }
}

#[tokio::test]
async fn allow_clean_draft() {
    let draft = sample_draft();
    let judge = CannedJudge {
        response: r#"{"verdict": "allow"}"#.into(),
    };
    let input = JudgeInput::from_broad(&draft);
    let verdict = judge_genesis_draft(&input, &judge)
        .await
        .expect("judge runs");
    assert_eq!(verdict, GenesisJudgeVerdict::Allow);
}

#[tokio::test]
async fn block_illegal_instructions() {
    let mut draft = sample_draft();
    draft.action_vocabulary = vec!["forge_signature".into(), "wire_funds".into()];
    let judge = CannedJudge {
        response:
            r#"{"verdict": "block", "reason": "action vocabulary includes financial fraud verbs"}"#
                .into(),
    };
    let input = JudgeInput::from_broad(&draft);
    let verdict = judge_genesis_draft(&input, &judge)
        .await
        .expect("judge runs");
    match verdict {
        GenesisJudgeVerdict::Block { reason } => {
            assert!(reason.contains("financial fraud"), "got: {reason}");
        }
        other => panic!("expected Block, got {other:?}"),
    }
}

#[tokio::test]
async fn allow_with_disclaimer_medical_without_disclaimer() {
    let mut draft = sample_draft();
    draft.persona_description =
        "Friendly medical helper who can answer any health question.".into();
    draft.action_vocabulary = vec!["answer_health_question".into(), "draft_advice".into()];
    let judge = CannedJudge {
        response: r#"{"verdict": "allow_with_disclaimer", "reason": "not a licensed medical professional; consult a doctor"}"#.into(),
    };
    let input = JudgeInput::from_broad(&draft);
    let verdict = judge_genesis_draft(&input, &judge)
        .await
        .expect("judge runs");
    match verdict {
        GenesisJudgeVerdict::AllowWithDisclaimer { reason } => {
            assert!(reason.contains("licensed medical"), "got: {reason}");
        }
        other => panic!("expected AllowWithDisclaimer, got {other:?}"),
    }
}

#[tokio::test]
async fn impersonation_detection() {
    let mut draft = sample_draft();
    draft.persona_description =
        "You are Elon Musk, the CEO of Tesla and SpaceX, replying to fans on social media.".into();
    let judge = CannedJudge {
        response: r#"{"verdict": "block", "reason": "persona impersonates a real public figure"}"#
            .into(),
    };
    let input = JudgeInput::from_broad(&draft);
    let verdict = judge_genesis_draft(&input, &judge)
        .await
        .expect("judge runs");
    match verdict {
        GenesisJudgeVerdict::Block { reason } => {
            assert!(reason.contains("impersonates"), "got: {reason}");
        }
        other => panic!("expected Block, got {other:?}"),
    }
}

#[tokio::test]
async fn judge_error_falls_back_to_block() {
    let draft = sample_draft();
    let judge = FailingJudge;
    let input = JudgeInput::from_broad(&draft);
    let verdict = judge_genesis_draft(&input, &judge)
        .await
        .expect("never panics");
    match verdict {
        GenesisJudgeVerdict::Block { reason } => {
            assert!(
                reason.contains("transport error"),
                "expected transport-error wording, got: {reason}",
            );
        }
        other => panic!(
            "judge crash MUST conservatively block, got {other:?} — this defends a load-bearing invariant",
        ),
    }
}

#[tokio::test]
async fn unparseable_judge_response_blocks() {
    let draft = sample_draft();
    let judge = CannedJudge {
        response: "this is not json at all, just prose".into(),
    };
    let input = JudgeInput::from_broad(&draft);
    let verdict = judge_genesis_draft(&input, &judge)
        .await
        .expect("never panics");
    match verdict {
        GenesisJudgeVerdict::Block { .. } => {}
        other => panic!("expected Block on unparseable response, got {other:?}"),
    }
}

#[test]
fn build_judge_prompt_includes_three_questions() {
    let draft = sample_draft();
    let input = JudgeInput::from_broad(&draft);
    let prompt = build_judge_prompt(&input);
    assert!(prompt.contains("Q1:"), "prompt missing Q1");
    assert!(prompt.contains("Q2:"), "prompt missing Q2");
    assert!(prompt.contains("Q3:"), "prompt missing Q3");
    assert!(prompt.contains("be my chief vibes officer"));
    assert!(prompt.contains("check_in"));
}

#[test]
fn build_judge_prompt_enumerates_dangerous_families_and_biases_to_block() {
    // Layer 2 of the synonym-blindness fix: the judge prompt must name the
    // four dangerous capability families AND instruct BLOCK (not
    // allow_with_disclaimer) when they would run autonomously.
    let draft = sample_draft();
    let input = JudgeInput::from_broad(&draft);
    let prompt = build_judge_prompt(&input);
    assert!(prompt.contains("Q4:"), "prompt missing the danger-family question");
    assert!(prompt.contains("MONEY-MOVEMENT"), "money family not enumerated");
    assert!(
        prompt.contains("EXTERNAL-DATA-EGRESS"),
        "egress family not enumerated"
    );
    assert!(
        prompt.contains("CREDENTIAL-ENTRY"),
        "credential family not enumerated"
    );
    assert!(
        prompt.contains("IRREVERSIBLE-DESTRUCTION"),
        "destruction family not enumerated"
    );
    // The bias-to-block instruction is the load-bearing line.
    assert!(
        prompt.contains("NOT allow_with_disclaimer"),
        "prompt must steer the judge to block, not disclaim, on Q4"
    );
}

#[tokio::test]
async fn block_autonomous_money_movement_role() {
    // A treasury-style role whose vocabulary moves money. With the hardened
    // prompt the judge returns block; we assert the pipeline honours it.
    let mut draft = sample_draft();
    draft.charter = "treasury assistant that moves money between our accounts".into();
    draft.action_vocabulary = vec!["transfer_funds".into(), "process_payment".into()];
    let judge = CannedJudge {
        response: r#"{"verdict": "block", "reason": "autonomous money movement (transfer/payment) must always require human approval"}"#.into(),
    };
    let input = JudgeInput::from_broad(&draft);
    let verdict = judge_genesis_draft(&input, &judge)
        .await
        .expect("judge runs");
    match verdict {
        GenesisJudgeVerdict::Block { reason } => {
            assert!(reason.contains("money movement"), "got: {reason}");
        }
        other => panic!("expected Block for autonomous money movement, got {other:?}"),
    }
}
