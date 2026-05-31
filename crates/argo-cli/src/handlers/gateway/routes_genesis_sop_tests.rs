//! Tests for request-level SOP ingestion (Genesis P6 gateway helper).

use super::*;
use base64::engine::general_purpose;

const SOP_TEXT: &str = "Acme Refund Procedure. Escalate any refund above the documented \
    threshold to the regional finance manager named Priyanka before issuing it. Never process \
    a chargeback without a matching authorization token in the Concordia ledger.";

#[test]
fn no_sop_field_yields_none() {
    let dir = tempfile::tempdir().expect("tmp");
    let out = ingest_request_sop(None, None, None, "be my support agent", dir.path());
    assert!(matches!(out, SopIngestOutcome::None));
    // Nothing written.
    assert!(!dir.path().join("knowledge").exists());
}

#[test]
fn text_sop_is_ingested_and_persisted() {
    let dir = tempfile::tempdir().expect("tmp");
    let out = ingest_request_sop(
        Some(SOP_TEXT),
        Some("text"),
        Some("acme.txt"),
        "be my support agent",
        dir.path(),
    );
    match out {
        SopIngestOutcome::Ingested { source_label, quiz_len } => {
            assert_eq!(source_label, "acme.txt");
            assert!(quiz_len > 0, "a content-rich SOP yields questions");
        }
        other => panic!("expected Ingested, got {:?}", std::mem::discriminant(&other)),
    }
    // Knowledge + quiz persisted under the vertical dir.
    assert!(dir.path().join("knowledge").join("sop.md").exists());
    assert!(dir.path().join("knowledge").join("sop-quiz.toml").exists());
}

#[test]
fn empty_sop_text_is_rejected_not_silently_dropped() {
    let dir = tempfile::tempdir().expect("tmp");
    // Whitespace-only → treated as no SOP (None), since we trim-and-filter the
    // field first (a blank field is "operator did not attach a SOP").
    let blank = ingest_request_sop(Some("   "), Some("text"), None, "role", dir.path());
    assert!(matches!(blank, SopIngestOutcome::None));
    // A too-short-but-nonblank body IS a supplied SOP → rejected fail-closed.
    let tiny = ingest_request_sop(Some("hello"), Some("text"), None, "role", dir.path());
    assert!(matches!(tiny, SopIngestOutcome::Rejected(_)));
}

#[test]
fn malformed_pdf_base64_is_rejected() {
    let dir = tempfile::tempdir().expect("tmp");
    // Valid base64, but not a PDF → extractor fails → Rejected (fail-closed).
    let not_a_pdf = general_purpose::STANDARD.encode(b"this is plainly not a pdf file at all");
    let out = ingest_request_sop(
        Some(&not_a_pdf),
        Some("pdf_base64"),
        Some("x.pdf"),
        "role",
        dir.path(),
    );
    assert!(matches!(out, SopIngestOutcome::Rejected(_)), "non-PDF base64 must be rejected");
}

#[test]
fn invalid_base64_is_rejected() {
    let dir = tempfile::tempdir().expect("tmp");
    let out = ingest_request_sop(
        Some("!!!not base64!!!"),
        Some("pdf_base64"),
        None,
        "role",
        dir.path(),
    );
    assert!(matches!(out, SopIngestOutcome::Rejected(_)));
}

#[test]
fn role_text_for_plan_includes_charter_and_actions() {
    let plan = argo_genesis::plan("be my marketing intern, run weekly campaigns", "maya")
        .expect("plan");
    let text = role_text_for_plan(&plan).to_lowercase();
    assert!(text.contains("marketing"), "charter terms present");
    // Action names are appended so role-overlapping SOP terms get filtered.
    if let Some(actions) = plan.manifest.actions.as_ref() {
        if let Some(first) = actions.actions.keys().next() {
            assert!(text.contains(&first.to_lowercase()));
        }
    }
}
