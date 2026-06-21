//! Tests for the role-contract comprehension gate (Genesis P4).

use super::*;
use crate::archetype::Archetype;
use crate::broad_role::BroadRoleDraft;
use crate::materialize_writer::system_prompt_from_spec;
use crate::prompt_writer::build_system_prompt;
use crate::spec::CapabilitySpec;

/// A minimal from-scratch spec for the prompt-keyword regression test.
fn spec_for(charter: &str, persona: &str) -> CapabilitySpec {
    let draft = BroadRoleDraft {
        charter: charter.to_string(),
        persona: persona.to_string(),
        persona_description: "Handles a bespoke role.".to_string(),
        action_vocabulary: vec!["review_filing".to_string(), "send_report".to_string()],
        success_metrics: vec!["zero missed deadlines".to_string()],
        onboarding_script: "Hi, I keep your filings on track.".to_string(),
        ..Default::default()
    };
    CapabilitySpec::from_broad_draft(draft, Archetype::CustomerSupportAgent)
}

/// Grade a question against a contract the same way the gateway orchestrator
/// does: every keyword must be a substring (AND semantics). Mirrors the
/// `SubstringContains` metric so the unit tests exercise the real pass rule.
fn passes(q: &ComprehensionQuestion, contract_lower: &str) -> bool {
    q.expected_keywords
        .iter()
        .all(|k| contract_lower.contains(&k.to_ascii_lowercase()))
}

#[test]
fn builds_exactly_five_questions_with_stable_ids() {
    let qs = build_comprehension_questions("be my marketing intern", &["draft_campaign".into()]);
    assert_eq!(qs.len(), 5);
    let ids: Vec<&str> = qs.iter().map(|q| q.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "comprehension_scope",
            "comprehension_approval",
            "comprehension_escalation",
            "comprehension_claims",
            "comprehension_action_fence",
        ]
    );
}

#[test]
fn scope_term_is_a_salient_charter_word() {
    let qs = build_comprehension_questions("be my compliance officer for healthcare", &[]);
    let scope = &qs[0];
    // "healthcare" (10) is the longest non-stopword token.
    assert_eq!(scope.expected_keywords, vec!["healthcare".to_string()]);
}

#[test]
fn scope_term_falls_back_to_action_then_word_when_charter_is_thin() {
    let qs = build_comprehension_questions("a b c", &["triage_ticket".into()]);
    assert_eq!(qs[0].expected_keywords, vec!["triage_ticket".to_string()]);
    let qs2 = build_comprehension_questions("a b c", &[]);
    // No salient word, no action → first whitespace token, lowercased.
    assert_eq!(qs2[0].expected_keywords, vec!["a".to_string()]);
}

#[test]
fn governance_keywords_are_the_documented_invariants() {
    let qs = build_comprehension_questions("be my marketing intern", &[]);
    assert_eq!(qs[1].expected_keywords, vec![APPROVAL_KEYWORD.to_string()]);
    assert_eq!(qs[2].expected_keywords, vec![ESCALATE_KEYWORD.to_string()]);
    assert_eq!(qs[3].expected_keywords, vec![EVIDENCE_KEYWORD.to_string()]);
    assert_eq!(qs[4].expected_keywords, vec![BLOCKED_KEYWORD.to_string()]);
}

#[test]
fn outcome_all_pass_clears_floor() {
    let o = ComprehensionOutcome::from_counts(5, 5);
    assert!(o.passed_floor);
    assert_eq!(o.pass_rate, 1.0);
}

#[test]
fn outcome_one_miss_fails_floor() {
    // A single governance miss must block — these are invariants, not a sample.
    let o = ComprehensionOutcome::from_counts(4, 5);
    assert!(!o.passed_floor);
    assert!((o.pass_rate - 0.8).abs() < 1e-9);
}

#[test]
fn outcome_zero_questions_fails_closed() {
    let o = ComprehensionOutcome::from_counts(0, 0);
    assert!(!o.passed_floor);
    assert_eq!(o.pass_rate, 0.0);
}

/// The load-bearing invariant: the ARCHETYPE-path system prompt must contain
/// every governance keyword the comprehension gate checks, so a real
/// archetype teammate clears the gate. Pins the prompt↔gate coupling at test
/// time (a reword trips here before it can falsely block a live hire).
#[test]
fn archetype_system_prompt_contains_every_governance_keyword() {
    let prompt =
        build_system_prompt("be my marketing intern", Archetype::MarketingIntern, "maya")
            .to_lowercase();
    for kw in [
        APPROVAL_KEYWORD,
        ESCALATE_KEYWORD,
        EVIDENCE_KEYWORD,
        BLOCKED_KEYWORD,
    ] {
        assert!(prompt.contains(kw), "archetype prompt missing `{kw}`");
    }
}

/// Same invariant for the from-scratch (spec) path so a synthesized teammate
/// clears the gate too.
#[test]
fn spec_system_prompt_contains_every_governance_keyword() {
    let spec = spec_for("be my compliance officer", "casey");
    let prompt = system_prompt_from_spec(&spec).to_lowercase();
    for kw in [
        APPROVAL_KEYWORD,
        ESCALATE_KEYWORD,
        EVIDENCE_KEYWORD,
        BLOCKED_KEYWORD,
    ] {
        assert!(prompt.contains(kw), "spec prompt missing `{kw}`");
    }
}

/// A real archetype teammate's full behaviour contract clears every
/// comprehension question (5/5) — the reference worker passes.
#[test]
fn reference_archetype_contract_passes_all_questions() {
    let charter = "be my marketing intern, run weekly campaigns to cold leads";
    let contract =
        build_system_prompt(charter, Archetype::MarketingIntern, "maya").to_lowercase();
    let qs = build_comprehension_questions(charter, &["draft_campaign".into()]);
    let passed = qs.iter().filter(|q| passes(q, &contract)).count();
    assert_eq!(passed, 5, "reference contract should pass all 5 questions");
    assert!(ComprehensionOutcome::from_counts(passed, qs.len()).passed_floor);
}

/// A thin/regressed contract that omits the escalation clause fails the gate —
/// proving the gate really blocks, not just scores.
#[test]
fn contract_missing_escalation_fails_the_gate() {
    let charter = "be my marketing intern";
    // A degenerate contract: names the role + approval + evidence + blocked,
    // but NEVER mentions escalation. A governance hole.
    let contract = "you are a marketing intern. sensitive actions require approval. \
                    cite evidence for every claim. undeclared actions are blocked."
        .to_string();
    let qs = build_comprehension_questions(charter, &[]);
    let passed = qs.iter().filter(|q| passes(q, &contract)).count();
    assert!(passed < qs.len(), "missing-escalation contract must lose a point");
    assert!(
        !ComprehensionOutcome::from_counts(passed, qs.len()).passed_floor,
        "any governance miss must fail the all-must-pass floor"
    );
}
