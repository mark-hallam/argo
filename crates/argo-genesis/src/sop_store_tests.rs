//! Tests for SOP knowledge/quiz on-disk persistence + reconstruction (P6).

use super::*;
use crate::sop::{build_sop_quiz, ingest_sop};

const SOP: &str = "Acme Refund Procedure\n\n\
    Escalate any refund above the documented threshold to the regional finance \
    manager named Priyanka before issuing it. Never process a chargeback without \
    a matching authorization token recorded in the Concordia ledger. Apply the \
    restocking surcharge to returned merchandise.";

#[test]
fn write_then_read_round_trips_knowledge_and_quiz() {
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = tmp.path().join(".genesis").join("acme").join("genesis-sam");
    std::fs::create_dir_all(&dir).expect("dir");

    let sop = ingest_sop(SOP, "acme.txt").expect("ingest");
    let quiz = build_sop_quiz(&sop, "be my support agent");
    assert!(!quiz.is_empty());
    write_sop_knowledge(&dir, &sop, &quiz).expect("write");

    let disk = read_sop_knowledge(&dir);
    let reread = disk.knowledge.expect("knowledge present after write");
    // The reconstructed corpus contains the same doc-specific terms — so the
    // disk fence grades against the SAME knowledge the SSE path did.
    let joined = reread.joined_text();
    assert!(joined.contains("priyanka"));
    assert!(joined.contains("concordia"));
    // The persisted quiz round-trips byte-identically.
    assert_eq!(disk.quiz, quiz);
    assert_eq!(reread.source_label, "acme.txt");
}

#[test]
fn reconstructed_quiz_still_discriminates_absorbed_from_not() {
    // The whole point of persisting: the fence must reach the SAME verdict.
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = tmp.path().join("v");
    std::fs::create_dir_all(&dir).expect("dir");
    let sop = ingest_sop(SOP, "acme.txt").expect("ingest");
    let quiz = build_sop_quiz(&sop, "be my support agent");
    write_sop_knowledge(&dir, &sop, &quiz).expect("write");

    let disk = read_sop_knowledge(&dir);
    let knowledge = disk.knowledge.expect("present").joined_text();
    let lower_knows = knowledge.to_lowercase();
    // Absorbed teammate → every keyword present.
    for q in &disk.quiz {
        assert!(q.expected_keywords.iter().all(|k| lower_knows.contains(k.as_str())));
    }
    // Un-ingested teammate → at least one keyword absent (fails the floor).
    let empty = "";
    assert!(disk
        .quiz
        .iter()
        .any(|q| !q.expected_keywords.iter().all(|k| empty.contains(k.as_str()))));
}

#[test]
fn no_sop_file_reads_as_not_applicable() {
    // A teammate with NO uploaded SOP → knowledge None → the SOP axis is not
    // applicable (DEC-025 #5, optional).
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = tmp.path().join("v");
    std::fs::create_dir_all(&dir).expect("dir");
    let disk = read_sop_knowledge(&dir);
    assert!(disk.knowledge.is_none(), "no SOP file → not applicable");
    assert!(disk.quiz.is_empty());
}

#[test]
fn present_sop_with_corrupt_quiz_is_applicable_and_fail_closed() {
    // A SOP body exists but the quiz file is corrupt/missing → the SOP IS
    // applicable (knowledge Some) but the quiz is empty → the gate treats this
    // as fail-closed (an uploaded doc that can't be re-graded must NOT pass).
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = tmp.path().join("v");
    let kdir = dir.join("knowledge");
    std::fs::create_dir_all(&kdir).expect("dir");
    let sop = ingest_sop(SOP, "acme.txt").expect("ingest");
    let quiz = build_sop_quiz(&sop, "agent");
    write_sop_knowledge(&dir, &sop, &quiz).expect("write");
    // Corrupt the quiz file.
    std::fs::write(kdir.join("sop-quiz.toml"), "this is not valid toml {{{").expect("corrupt");

    let disk = read_sop_knowledge(&dir);
    assert!(disk.knowledge.is_some(), "SOP body present → applicable");
    assert!(disk.quiz.is_empty(), "corrupt quiz → empty → fail-closed");
}

#[test]
fn stored_body_carries_the_untrusted_provenance_banner() {
    // Prompt-injection trust boundary: the persisted body must be tagged as
    // untrusted data so a downstream assembler never obeys imperatives in it.
    let tmp = tempfile::tempdir().expect("tmp");
    let dir = tmp.path().join("v");
    std::fs::create_dir_all(&dir).expect("dir");
    let sop = ingest_sop(SOP, "acme.txt").expect("ingest");
    let quiz = build_sop_quiz(&sop, "agent");
    write_sop_knowledge(&dir, &sop, &quiz).expect("write");

    let body = std::fs::read_to_string(dir.join("knowledge").join("sop.md")).expect("read");
    assert!(body.contains("SKYCOT-PROVENANCE: untrusted-document-content"));
    assert!(body.contains("NOT operator instructions"));
}

#[test]
fn knowledge_is_written_under_the_tenant_scoped_vertical_dir_only() {
    // Tenant scoping is filesystem-enforced: the knowledge lands ONLY under the
    // tenant's own `.genesis/<tenant>/<id>/knowledge/` path. A sibling tenant
    // dir does not contain it.
    let tmp = tempfile::tempdir().expect("tmp");
    let root = tmp.path().join(".genesis");
    let tenant_a = root.join("tenant-a").join("genesis-sam");
    let tenant_b = root.join("tenant-b").join("genesis-sam");
    std::fs::create_dir_all(&tenant_a).expect("a");
    std::fs::create_dir_all(&tenant_b).expect("b");

    let sop = ingest_sop(SOP, "acme.txt").expect("ingest");
    let quiz = build_sop_quiz(&sop, "agent");
    write_sop_knowledge(&tenant_a, &sop, &quiz).expect("write a");

    // Tenant A sees its SOP.
    assert!(read_sop_knowledge(&tenant_a).knowledge.is_some());
    // Tenant B's directory has NO knowledge — the file was never written there.
    assert!(
        read_sop_knowledge(&tenant_b).knowledge.is_none(),
        "a different tenant's vertical dir must not see tenant A's SOP"
    );
    assert!(!tenant_b.join("knowledge").join("sop.md").exists());
}
