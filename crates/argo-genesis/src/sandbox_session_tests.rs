//! Tests for the sandbox quiz session — retrieval honesty, citation, and the
//! memory-edit round-trip that proves a corrected document changes the answer.
//!
//! Document bodies are kept above the SOP ingest floor (`MIN_SOP_CHARS`, 80
//! usable chars) so `ingest_onboarding_document` accepts them — short snippets
//! are rejected at ingest by design.

use super::*;
use crate::{ingest_onboarding_document, OnboardingDocument};

fn doc(body: &str, label: &str) -> OnboardingDocument {
    ingest_onboarding_document(body, label).expect("ingest")
}

#[test]
fn answers_from_uploaded_document_with_citation() {
    // The answer to a question that IS in an uploaded SOP cites that SOP.
    let d = doc(
        "Refund requests above five hundred dollars require regional manager approval \
         before they may be issued to the customer. Smaller refunds may be approved by \
         any support teammate on shift without escalation.",
        "refund-policy.md",
    );
    let ans = answer_quiz_question("How should I handle a refund over $500?", &[d]);
    assert!(ans.answered, "a covered question must be answered");
    assert!(
        ans.answer.to_lowercase().contains("regional manager"),
        "answer must quote the SOP: {}",
        ans.answer
    );
    assert_eq!(ans.citations.len(), 1);
    assert_eq!(ans.citations[0].source_label, "refund-policy.md");
}

#[test]
fn empty_memory_refuses_instead_of_fabricating() {
    // No theater: with nothing ingested the teammate says it doesn't know.
    let ans = answer_quiz_question("What is our refund policy?", &[]);
    assert!(!ans.answered);
    assert_eq!(ans.answer, NO_KNOWLEDGE_ANSWER);
    assert!(ans.citations.is_empty());
}

#[test]
fn uncovered_question_refuses_instead_of_fabricating() {
    // The document exists but says nothing about the topic asked → honest refusal.
    let d = doc(
        "Refund requests above five hundred dollars require regional manager approval \
         before they may be issued. Document the reason for every refund in the ticket.",
        "refund-policy.md",
    );
    let ans = answer_quiz_question("What is our parental leave entitlement?", &[d]);
    assert!(!ans.answered, "uncovered topic must not be fabricated");
    assert_eq!(ans.answer, NO_KNOWLEDGE_ANSWER);
}

#[test]
fn memory_edit_round_trips_into_the_next_answer() {
    // The acceptance loop: ask → wrong assumption → operator corrects the memory
    // (re-uploads a corrected doc) → next answer reflects the change.
    let before = doc(
        "Refunds over five hundred dollars require manager approval before they are \
         issued. Always record the approving manager's name against the ticket.",
        "refund-policy.md",
    );
    let first = answer_quiz_question("Who signs off a large refund?", &[before]);
    assert!(first.answered);
    assert!(first.answer.to_lowercase().contains("manager"));
    assert!(!first.answer.to_lowercase().contains("legal"));

    // Operator edits the memory entry: large refunds now need legal sign-off.
    let after = doc(
        "Refunds over five hundred dollars require legal sign-off, not just manager \
         approval, before they are issued. Record the approving lawyer on the ticket.",
        "refund-policy.md",
    );
    let second = answer_quiz_question("Who signs off a large refund?", &[after]);
    assert!(second.answered);
    assert!(
        second.answer.to_lowercase().contains("legal"),
        "edited memory must change the answer: {}",
        second.answer
    );
}

#[test]
fn best_matching_chunk_ranks_first() {
    // A question that overlaps two documents cites the one with more term overlap.
    let escalation = doc(
        "Escalate any chargeback dispute to the finance team within twenty four hours \
         of receiving the notice. Attach the original order and any customer messages.",
        "escalation.md",
    );
    let refunds = doc(
        "Refund requests above five hundred dollars require regional manager approval \
         before issue. Record the approving manager and the reason on the ticket.",
        "refund-policy.md",
    );
    let ans = answer_quiz_question(
        "What is the approval rule for a large refund request?",
        &[escalation, refunds],
    );
    assert!(ans.answered);
    assert_eq!(
        ans.citations[0].source_label, "refund-policy.md",
        "the more relevant document must be cited first"
    );
}

#[test]
fn citations_are_capped() {
    // Many matching chunks still produce a bounded, readable answer.
    let docs: Vec<OnboardingDocument> = (0..5)
        .map(|i| {
            doc(
                "Refund requests require manager approval before they are issued to the \
                 customer, and the approving manager must be recorded on the ticket.",
                &format!("policy-{i}.md"),
            )
        })
        .collect();
    let ans = answer_quiz_question("What is the refund approval rule?", &docs);
    assert!(ans.answered);
    assert!(ans.citations.len() <= MAX_CITATIONS);
}

#[test]
fn suggested_questions_are_grounded_and_bounded() {
    let d = doc(
        "Refund requests above five hundred dollars require regional manager approval. \
         Escalate any chargeback dispute to the finance team within twenty four hours.",
        "policy.md",
    );
    let qs = suggested_quiz_questions(&[d], "support teammate");
    assert!(!qs.is_empty());
    assert!(qs.len() <= 5);
    // The lead question is the broad opener.
    assert!(qs[0].to_lowercase().contains("learned"));
}

#[test]
fn suggested_questions_empty_when_nothing_ingested_and_no_role() {
    let qs = suggested_quiz_questions(&[], "");
    assert!(qs.is_empty());
}

#[test]
fn whitespace_only_question_refuses() {
    let d = doc(
        "Refunds require manager approval before they are issued, and the approving \
         manager's name must be recorded against the customer ticket every time.",
        "policy.md",
    );
    let ans = answer_quiz_question("   ?  ", &[d]);
    assert!(!ans.answered);
}
