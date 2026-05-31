//! Pure-crate tests for the SOP-upload comprehension gate (Genesis P6).
//!
//! These prove the moat's properties without a provider or the gateway:
//! ingestion, quiz-grounded-in-doc, discrimination, and fail-closed
//! optionality. The end-to-end grade-through-`run_eval` proofs live in the
//! gateway (`routes_genesis_gate_tests.rs`); these are the deterministic
//! re-implementation of the same `SubstringContains` scoring so the pure crate
//! can assert the gate's behaviour standalone.

use super::*;

/// A realistic refund-policy SOP with several distinctive domain terms a
/// teammate could only know by reading THIS document.
const REFUND_SOP: &str = "Acme Refund Standard Operating Procedure\n\n\
    Approve refunds automatically when the order total is small. Escalate any \
    refund above the threshold to the regional finance manager named Priyanka \
    before issuing it. Never process a chargeback without a matching ticket \
    reference, which our team calls an authorization token. Apply the \
    restocking surcharge to returned merchandise unless the customer holds a \
    platinum loyalty tier. Document every escalation in the Concordia ledger \
    so the auditor can reconcile it.";

/// The deterministic re-implementation of the gateway's
/// `run_sop_comprehension_floor`: a question passes iff EVERY doc-specific
/// keyword is a substring of the teammate's ingested knowledge text (the
/// `SubstringContains` AND-semantics), and the all-must-pass floor clears iff
/// every question passed. Mirrors how the canonical `run_eval` + `ContractSubject`
/// score it in production.
fn score_quiz(quiz: &[SopQuestion], teammate_knowledge: &str) -> SopComprehensionOutcome {
    if quiz.is_empty() {
        return SopComprehensionOutcome::from_counts(0, 0);
    }
    let lower = teammate_knowledge.to_lowercase();
    let passed = quiz
        .iter()
        .filter(|q| q.expected_keywords.iter().all(|k| lower.contains(k.as_str())))
        .count();
    SopComprehensionOutcome::from_counts(passed, quiz.len())
}

#[test]
fn ingest_chunks_a_real_sop_and_is_not_empty() {
    let sop = ingest_sop(REFUND_SOP, "acme-refund.txt").expect("content-rich SOP ingests");
    assert!(!sop.is_empty());
    assert_eq!(sop.source_label, "acme-refund.txt");
    assert!(!sop.chunks.is_empty());
    // The ingested knowledge preserves the doc's distinctive terms.
    let joined = sop.joined_text();
    assert!(joined.contains("priyanka"));
    assert!(joined.contains("concordia"));
}

#[test]
fn ingest_rejects_an_empty_or_image_only_doc_fail_closed() {
    // Empty / whitespace-only (the image-only-PDF case where extraction
    // returns blank) is rejected at ingest, not silently waved through.
    assert!(ingest_sop("   \n\n  ", "scan.pdf").is_err());
    assert!(ingest_sop("", "blank.txt").is_err());
    // Too-short junk is rejected too.
    assert!(ingest_sop("hi", "junk.txt").is_err());
}

#[test]
fn ingest_falls_back_to_a_generic_label_when_none_supplied() {
    let sop = ingest_sop(REFUND_SOP, "   ").expect("ingests");
    assert_eq!(sop.source_label, "uploaded-sop");
}

#[test]
fn quiz_keywords_are_grounded_in_the_document_not_constants() {
    let sop = ingest_sop(REFUND_SOP, "acme.txt").expect("ingests");
    let quiz = build_sop_quiz(&sop, "be my customer support agent");
    assert!(!quiz.is_empty(), "a content-rich SOP yields questions");
    let knowledge = sop.joined_text();
    for q in &quiz {
        assert!(!q.expected_keywords.is_empty());
        for kw in &q.expected_keywords {
            assert!(
                knowledge.contains(kw.as_str()),
                "every quiz keyword must appear in the ingested doc (grounded): {kw}"
            );
        }
    }
}

#[test]
fn quiz_keywords_exclude_terms_the_role_already_implies() {
    // If the role text itself names a term, that term cannot distinguish
    // "read the doc" from "knows the role", so it must NOT be a quiz keyword.
    let sop = ingest_sop(REFUND_SOP, "acme.txt").expect("ingests");
    // A role that already mentions "Priyanka" and "chargeback".
    let role = "you are the refund desk reporting to priyanka; you handle chargeback disputes";
    let quiz = build_sop_quiz(&sop, role);
    for q in &quiz {
        for kw in &q.expected_keywords {
            assert_ne!(kw, "priyanka", "role-overlapping term must be filtered out");
            assert_ne!(kw, "chargeback", "role-overlapping term must be filtered out");
        }
    }
}

#[test]
fn teammate_that_absorbed_the_sop_passes_the_quiz() {
    let sop = ingest_sop(REFUND_SOP, "acme.txt").expect("ingests");
    let quiz = build_sop_quiz(&sop, "be my support agent");
    // A teammate whose knowledge contains the ingested SOP answers every
    // question with text that holds the doc-specific keywords → passes.
    let teammate_knowledge = sop.joined_text();
    let outcome = score_quiz(&quiz, &teammate_knowledge);
    assert!(outcome.applicable);
    assert!(
        outcome.passed_floor,
        "a teammate that absorbed the SOP clears the all-must-pass floor: {}/{}",
        outcome.passed, outcome.total
    );
}

#[test]
fn teammate_without_the_sop_fails_the_quiz_discrimination_preserved() {
    let sop = ingest_sop(REFUND_SOP, "acme.txt").expect("ingests");
    let quiz = build_sop_quiz(&sop, "be my support agent");
    // A teammate that did NOT ingest the SOP — its knowledge is empty / generic
    // role text — cannot produce the doc-specific terms, so the SAME quiz fails.
    let empty_knowledge = "";
    let empty_outcome = score_quiz(&quiz, empty_knowledge);
    assert!(
        !empty_outcome.passed_floor,
        "an un-ingested teammate must NOT clear the SOP floor (no hire without comprehension)"
    );
    // Even a teammate that knows its generic role but not the SOP fails — none
    // of the doc-specific terms (Priyanka, Concordia, authorization token, …)
    // appear in plain role knowledge.
    let role_only = "i am a customer support agent. i answer tickets and help customers.";
    let role_outcome = score_quiz(&quiz, role_only);
    assert!(
        !role_outcome.passed_floor,
        "generic role knowledge must NOT pass a doc-specific quiz: {}/{}",
        role_outcome.passed, role_outcome.total
    );
}

#[test]
fn the_same_quiz_discriminates_absorbed_from_not() {
    // The single strongest property: ONE quiz, two teammates, opposite verdicts.
    let sop = ingest_sop(REFUND_SOP, "acme.txt").expect("ingests");
    let quiz = build_sop_quiz(&sop, "be my support agent");
    let absorbed = score_quiz(&quiz, &sop.joined_text());
    let not_absorbed = score_quiz(&quiz, "generic support agent who knows nothing specific");
    assert!(absorbed.passed_floor);
    assert!(!not_absorbed.passed_floor);
    assert!(
        absorbed.passed > not_absorbed.passed,
        "the absorbed teammate answers strictly more questions"
    );
}

#[test]
fn not_applicable_outcome_passes_vacuously_when_no_sop_uploaded() {
    // DEC-025 #5: SOP is optional. No upload → the axis is not applicable and
    // does not block (the other four gate axes still apply).
    let na = SopComprehensionOutcome::not_applicable();
    assert!(!na.applicable);
    assert!(na.passed_floor, "no-SOP path must not block hire");
}

#[test]
fn a_present_sop_with_zero_gradeable_questions_fails_closed() {
    // A doc whose every salient term is also in the role text yields an empty
    // quiz. A present-but-unquizzable SOP is a fail-closed signal: total==0 →
    // floor fails (a doc the teammate can't be quizzed on must not wave it
    // through).
    let outcome = SopComprehensionOutcome::from_counts(0, 0);
    assert!(outcome.applicable);
    assert!(
        !outcome.passed_floor,
        "an applicable SOP with no gradeable question must fail closed"
    );
}

#[test]
fn quiz_generation_is_deterministic_repeatable_gate() {
    // The same doc + role yields the byte-identical quiz every time — the gate
    // is repeatable (a priority of the platform).
    let sop = ingest_sop(REFUND_SOP, "acme.txt").expect("ingests");
    let q1 = build_sop_quiz(&sop, "be my support agent");
    let q2 = build_sop_quiz(&sop, "be my support agent");
    assert_eq!(q1, q2);
}

#[test]
fn ingest_is_robust_to_pdf_style_whitespace_noise() {
    // PDF extraction emits ragged whitespace / form-feeds; normalisation must
    // still yield usable knowledge that preserves the domain terms.
    let noisy = "Acme   Refund\n\n\n\n   Procedure   \n   Escalate   to   Priyanka   \
                 the   regional   finance   manager   before   any   chargeback   refund   \
                 above   the   documented   threshold   in   the   Concordia   ledger.";
    let sop = ingest_sop(noisy, "noisy.pdf").expect("ingests despite noise");
    let joined = sop.joined_text();
    assert!(joined.contains("priyanka"));
    assert!(joined.contains("concordia"));
    // No collapsed-together garbage tokens.
    assert!(!joined.contains("priyankathe"));
}
