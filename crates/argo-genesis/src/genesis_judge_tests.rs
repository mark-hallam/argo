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
        ..Default::default()
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

/// Build a two-action [`argo_vertical::ActionsSection`] for the
/// approval-annotation tests: one approval-gated send action + one autonomous
/// read action. BTreeMap key order is alphabetical, so `send_response` sorts
/// before `triage_ticket`.
fn two_action_section() -> argo_vertical::ActionsSection {
    use argo_vertical::{ActionDeclaration, ActionRiskLevel};
    let mut actions = std::collections::BTreeMap::new();
    actions.insert(
        "send_response".to_string(),
        ActionDeclaration {
            risk: ActionRiskLevel::High,
            requires_approval: true,
            ..ActionDeclaration::default()
        },
    );
    actions.insert(
        "triage_ticket".to_string(),
        ActionDeclaration {
            risk: ActionRiskLevel::Low,
            requires_approval: false,
            ..ActionDeclaration::default()
        },
    );
    argo_vertical::ActionsSection { actions }
}

#[test]
fn from_plan_projection_carries_parallel_approval_flags() {
    // The plan projection must capture each action's requires_approval flag
    // PARALLEL to its name (same order, same length) — that parallelism is
    // what lets the prompt annotate each action correctly.
    let section = two_action_section();
    let input = JudgeInput::from_plan_projection("charter", "persona", Some(&section));
    let approvals = input
        .action_approvals
        .as_ref()
        .expect("plan projection populates approvals");
    assert_eq!(
        approvals.len(),
        input.action_vocabulary.len(),
        "approvals must be parallel to names"
    );
    // BTreeMap order: send_response (gated) before triage_ticket (autonomous).
    assert_eq!(input.action_vocabulary, vec!["send_response", "triage_ticket"]);
    assert_eq!(approvals, &vec![true, false]);
}

#[test]
fn build_judge_prompt_annotates_each_action_with_its_approval_status() {
    // The load-bearing fix: the prompt must tell the judge WHICH actions are
    // approval-gated, so it can exempt the safe-by-construction ones.
    let section = two_action_section();
    let input = JudgeInput::from_plan_projection(
        "be my customer-support teammate",
        "Sam, a support agent",
        Some(&section),
    );
    let prompt = build_judge_prompt(&input);
    assert!(
        prompt.contains("send_response (approval-gated)"),
        "approval-gated send action must be annotated as approval-gated: {prompt}"
    );
    assert!(
        prompt.contains("triage_ticket (autonomous)"),
        "autonomous action must be annotated as autonomous: {prompt}"
    );
}

#[test]
fn build_judge_prompt_instructs_not_to_block_approval_gated_actions() {
    // Q4 must only target AUTONOMOUS dangerous actions, and the prompt must
    // explicitly exempt approval-gated send/pay/delete actions. This is what
    // stops the judge dead-ending the onboarding first hire.
    let section = two_action_section();
    let input = JudgeInput::from_plan_projection("charter", "persona", Some(&section));
    let prompt = build_judge_prompt(&input);
    assert!(
        prompt.contains("AUTONOMOUS (NOT approval-gated)"),
        "Q4 must scope to autonomous dangerous actions: {prompt}"
    );
    assert!(
        prompt.contains("Do NOT block an action merely because it sends"),
        "prompt must explicitly exempt approval-gated send/pay/delete actions: {prompt}"
    );
    // The original bias-to-block on AUTONOMOUS danger is preserved.
    assert!(
        prompt.contains("NOT allow_with_disclaimer"),
        "autonomous-danger bias-to-block must remain: {prompt}"
    );
}

#[test]
fn from_broad_leaves_actions_unannotated() {
    // The broad-role path carries no approval semantics → action_approvals is
    // None → the prompt falls back to bare names (every action treated as
    // autonomous), identical to the pre-fix behaviour. No broad-role regression.
    let draft = sample_draft();
    let input = JudgeInput::from_broad(&draft);
    assert!(
        input.action_approvals.is_none(),
        "broad-role projection must not assert approval semantics it does not have"
    );
    let prompt = build_judge_prompt(&input);
    assert!(prompt.contains("check_in"), "bare action name still present");
    assert!(
        !prompt.contains("check_in (approval-gated)") && !prompt.contains("check_in (autonomous)"),
        "broad-role actions must NOT be annotated: {prompt}"
    );
}
