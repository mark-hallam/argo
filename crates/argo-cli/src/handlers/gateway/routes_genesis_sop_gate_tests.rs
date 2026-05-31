//! End-to-end SOP-comprehension gate tests through the REAL `argo_eval::run_eval`
//! runner + the disk fence (Genesis P6 — the moat).
//!
//! The pure-crate proofs live in `argo-genesis` (`sop_tests.rs` /
//! `sop_store_tests.rs`); these prove the SAME four properties end-to-end
//! through the gateway's canonical grading path + the on-disk reconstruction
//! the hire-commit fence uses, so the moat holds at the actual hire seams:
//!
//! 1. **No hire without SOP comprehension** — a teammate whose knowledge lacks
//!    the doc-specific terms is hard-Blocked by the full gate.
//! 2. **Quiz grounded in the doc** — graded against the ingested knowledge, a
//!    doc-specific keyword passes only when the doc was absorbed.
//! 3. **Tenant-scoped** — the disk fence reads the SOP from the tenant's own
//!    vertical dir; a sibling tenant's dir has none.
//! 4. **Discrimination preserved** — the SAME quiz passes the absorbed teammate
//!    and fails the un-ingested one, through the real runner.

use super::grade_sop_quiz;
use argo_genesis::{
    build_sop_quiz, ingest_sop, plan, read_sop_knowledge, write_sop_knowledge, GateState,
    GateVerdict, SafetySignal,
};

/// A content-rich refund SOP with several distinctive, role-distinct terms.
const REFUND_SOP: &str = "Acme Refund Standard Operating Procedure\n\n\
    Escalate any refund above the documented threshold to the regional finance \
    manager named Priyanka before issuing it. Never process a chargeback without \
    a matching authorization token recorded in the Concordia ledger. Apply the \
    restocking surcharge to returned merchandise unless the customer holds a \
    platinum loyalty tier.";

#[tokio::test]
async fn absorbed_teammate_clears_the_sop_floor_through_the_real_runner() {
    // PROPERTY 1+4: a teammate whose ingested knowledge IS the SOP answers every
    // doc-grounded question (the keywords appear) → the all-must-pass SOP floor
    // clears, graded through the canonical `run_eval`.
    let sop = ingest_sop(REFUND_SOP, "acme.txt").expect("ingest");
    let quiz = build_sop_quiz(&sop, "be my support agent");
    assert!(!quiz.is_empty());
    let outcome = grade_sop_quiz(&quiz, &sop.joined_text()).await;
    assert!(outcome.applicable);
    assert!(
        outcome.passed_floor,
        "absorbed teammate clears the SOP floor: {}/{}",
        outcome.passed, outcome.total
    );
}

#[tokio::test]
async fn un_ingested_teammate_fails_the_sop_floor_through_the_real_runner() {
    // PROPERTY 1+4 (the moat): the SAME quiz, graded against a teammate that did
    // NOT ingest the SOP (empty / generic role knowledge), fails the floor — no
    // hire without genuine comprehension of the SPECIFIC document.
    let sop = ingest_sop(REFUND_SOP, "acme.txt").expect("ingest");
    let quiz = build_sop_quiz(&sop, "be my support agent");

    let empty = grade_sop_quiz(&quiz, "").await;
    assert!(!empty.passed_floor, "empty-knowledge teammate must fail the SOP floor");

    let role_only =
        grade_sop_quiz(&quiz, "i am a customer support agent who answers tickets").await;
    assert!(
        !role_only.passed_floor,
        "generic role knowledge must NOT pass a doc-specific quiz: {}/{}",
        role_only.passed, role_only.total
    );
    // And the absorbed teammate strictly out-scores the un-ingested one.
    let absorbed = grade_sop_quiz(&quiz, &sop.joined_text()).await;
    assert!(absorbed.passed > role_only.passed);
}

#[tokio::test]
async fn full_gate_hard_blocks_a_teammate_that_failed_its_uploaded_sop() {
    // END-TO-END: a teammate that clears eval + comprehension + a confirmed
    // contract but FAILS its uploaded-SOP quiz must be HARD Blocked — the SOP
    // axis is fail-closed when a doc is present. This is the moat at the gate.
    use argo_genesis::{ComprehensionOutcome, ContractGateInput, EvalFloorOutcome, RoleContract};
    let p = plan("be my support agent who answers customer tickets", "sam").expect("plan");
    let sop = ingest_sop(REFUND_SOP, "acme.txt").expect("ingest");
    let quiz = build_sop_quiz(&sop, "be my support agent who answers customer tickets");
    // Grade against an un-ingested teammate → the SOP axis fails.
    let sop_outcome = grade_sop_quiz(&quiz, "knows nothing about the policy").await;
    assert!(!sop_outcome.passed_floor);

    let digest = RoleContract::derive_from_manifest(&p.manifest).digest();
    let verdict = GateVerdict::compose(
        EvalFloorOutcome::from_counts(10, 10),
        ComprehensionOutcome::from_counts(5, 5),
        sop_outcome,
        SafetySignal::Allow,
        ContractGateInput::from_plan(&p, Some(&digest)),
    );
    assert_eq!(
        verdict.state,
        GateState::Blocked,
        "an uploaded-SOP comprehension failure must hard-block the hire; reasons={:?}",
        verdict.reasons
    );
    assert!(!verdict.state.allows_hire());
    assert!(
        verdict.reasons.iter().any(|r| r.contains("SOP-comprehension failed")),
        "expected a SOP-comprehension reason; got {:?}",
        verdict.reasons
    );
}

#[tokio::test]
async fn full_gate_passes_when_the_teammate_absorbed_the_sop() {
    // The positive case: eval + comprehension + SOP all pass + confirmed
    // contract + (test) Allow safety → Pass. Proves the SOP axis does not
    // spuriously block a teammate that DID absorb the doc.
    use argo_genesis::{ComprehensionOutcome, ContractGateInput, EvalFloorOutcome, RoleContract};
    let p = plan("be my support agent who answers customer tickets", "sam").expect("plan");
    let sop = ingest_sop(REFUND_SOP, "acme.txt").expect("ingest");
    let quiz = build_sop_quiz(&sop, "be my support agent who answers customer tickets");
    let sop_outcome = grade_sop_quiz(&quiz, &sop.joined_text()).await;
    assert!(sop_outcome.passed_floor);

    let digest = RoleContract::derive_from_manifest(&p.manifest).digest();
    let verdict = GateVerdict::compose(
        EvalFloorOutcome::from_counts(10, 10),
        ComprehensionOutcome::from_counts(5, 5),
        sop_outcome,
        SafetySignal::Allow,
        ContractGateInput::from_plan(&p, Some(&digest)),
    );
    assert_eq!(verdict.state, GateState::Pass, "reasons={:?}", verdict.reasons);
    assert!(verdict.sop_applicable);
    assert_eq!(verdict.sop_passed, verdict.sop_total);
    assert!(verdict.sop_total > 0);
}

#[tokio::test]
async fn no_sop_uploaded_leaves_the_gate_unaffected() {
    // DEC-025 #5: SOP is optional. With no SOP, the axis is not applicable and
    // the gate verdict is identical to a no-SOP teammate (eval + comprehension +
    // safety + contract only).
    use argo_genesis::{ComprehensionOutcome, ContractGateInput, EvalFloorOutcome, RoleContract};
    use argo_genesis::SopComprehensionOutcome;
    let p = plan("be my support agent who answers customer tickets", "sam").expect("plan");
    let digest = RoleContract::derive_from_manifest(&p.manifest).digest();
    let verdict = GateVerdict::compose(
        EvalFloorOutcome::from_counts(10, 10),
        ComprehensionOutcome::from_counts(5, 5),
        SopComprehensionOutcome::not_applicable(),
        SafetySignal::Allow,
        ContractGateInput::from_plan(&p, Some(&digest)),
    );
    assert_eq!(verdict.state, GateState::Pass);
    assert!(!verdict.sop_applicable, "no SOP → not applicable");
    assert!(
        !verdict.reasons.iter().any(|r| r.contains("SOP")),
        "no-SOP gate must not mention SOP; got {:?}",
        verdict.reasons
    );
}

#[tokio::test]
async fn disk_fence_regrades_the_same_sop_verdict_the_sse_path_wrote() {
    // PARITY: the SSE path writes the ingested SOP + quiz to the vertical dir;
    // the hire-commit fence re-reads them and re-grades. Both must reach the
    // SAME doc-comprehension verdict. We assert the disk-reconstructed knowledge
    // + quiz still discriminate absorbed-vs-not exactly as the SSE-path grading.
    let dir = tempfile::tempdir().expect("tmp");
    let vdir = dir.path().join(".genesis").join("acme").join("genesis-sam");
    std::fs::create_dir_all(&vdir).expect("dir");
    let sop = ingest_sop(REFUND_SOP, "acme.txt").expect("ingest");
    let quiz = build_sop_quiz(&sop, "be my support agent");
    write_sop_knowledge(&vdir, &sop, &quiz).expect("write");

    // The fence re-reads from disk.
    let disk = read_sop_knowledge(&vdir);
    let reread_knowledge = disk.knowledge.expect("present").joined_text();
    // Absorbed (the reread knowledge itself) → passes; un-ingested → fails.
    let absorbed = grade_sop_quiz(&disk.quiz, &reread_knowledge).await;
    let not_absorbed = grade_sop_quiz(&disk.quiz, "no idea").await;
    assert!(absorbed.passed_floor, "disk-reconstructed SOP clears for an absorbed teammate");
    assert!(!not_absorbed.passed_floor, "disk-reconstructed SOP fails an un-ingested teammate");
}

#[tokio::test]
async fn tenant_b_cannot_read_tenant_a_uploaded_sop_at_the_fence() {
    // PROPERTY 3 (tenant-scoped, fail-closed): tenant A uploads a SOP into its
    // own `.genesis/<A>/<id>/`. Tenant B's vertical dir has NO knowledge, so the
    // SOP axis there is NOT applicable — B never sees, nor is graded against,
    // A's document. (P5 already fails the cross-tenant hire closed; this proves
    // the P6 knowledge does not leak across the tenant filesystem boundary.)
    let dir = tempfile::tempdir().expect("tmp");
    let root = dir.path().join(".genesis");
    let a = root.join("tenant-a").join("genesis-sam");
    let b = root.join("tenant-b").join("genesis-sam");
    std::fs::create_dir_all(&a).expect("a");
    std::fs::create_dir_all(&b).expect("b");
    let sop = ingest_sop(REFUND_SOP, "acme.txt").expect("ingest");
    let quiz = build_sop_quiz(&sop, "be my support agent");
    write_sop_knowledge(&a, &sop, &quiz).expect("write a");

    assert!(read_sop_knowledge(&a).knowledge.is_some(), "A sees its own SOP");
    assert!(
        read_sop_knowledge(&b).knowledge.is_none(),
        "tenant B's vertical dir must not contain tenant A's uploaded SOP"
    );
}
