//! Tests for [`crate::onboarding_documents`] — chunking, tenant-scoped disk
//! round-trip, the cross-tenant negative, and the parse-failure floor.

use super::*;
use std::path::PathBuf;

/// A unique temp dir for one test (no external tempfile dep — mirrors the
/// sibling sop_store tests' posture).
fn temp_root(tag: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    dir.push(format!("argo-onboarding-{tag}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("create temp root");
    dir
}

const SAMPLE: &str = "Returns are accepted within thirty days when the customer \
presents the original receipt. Escalate any refund above five hundred dollars to \
the regional finance manager named Priya before issuing it. Never authorise a \
chargeback without a matching RMA ticket reference on file.";

#[test]
fn ingests_and_chunks_a_document() {
    let doc = ingest_onboarding_document(SAMPLE, "return-policy.md").expect("ingest");
    assert_eq!(doc.source_label, "return-policy.md");
    assert!(doc.chunk_count() >= 1);
    // The doc-specific terms survive into the retrieval corpus.
    let joined = doc.joined_text();
    assert!(joined.contains("priya"));
    assert!(joined.contains("chargeback"));
}

#[test]
fn parse_failure_floor_rejects_too_short() {
    // An image-only / junk doc that extracts to almost nothing fails closed at
    // ingest — the gateway turns this Err into a per-file "skipped" so the
    // remaining files still ingest.
    let err = ingest_onboarding_document("tiny", "scan.pdf").unwrap_err();
    assert!(matches!(err, crate::GenesisError::Validation { .. }));
}

#[test]
fn batch_round_trips_through_disk() {
    let root = temp_root("roundtrip");
    let vdir = root.join(".genesis").join("acme").join("genesis-sam");
    let docs = vec![
        ingest_onboarding_document(SAMPLE, "return-policy.md").expect("ingest a"),
        ingest_onboarding_document(
            "The escalation matrix routes billing disputes to the finance pod and \
             security incidents to the on-call engineer within fifteen minutes.",
            "escalation.md",
        )
        .expect("ingest b"),
    ];
    write_onboarding_documents(&vdir, &docs).expect("write");

    let labels = onboarding_document_labels(&vdir);
    assert_eq!(labels, vec!["return-policy.md", "escalation.md"]);

    let read = read_onboarding_documents(&vdir);
    assert_eq!(read.len(), 2);
    assert_eq!(read[0].source_label, "return-policy.md");
    assert!(read[0].joined_text().contains("priya"));
    assert!(read[1].joined_text().contains("escalation matrix"));

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn cross_tenant_documents_do_not_leak() {
    // Tenant A uploads SOPs under its own .genesis/<slug>/<id> tree. Tenant B's
    // teammate resolves a DIFFERENT directory — and must read nothing.
    let root = temp_root("crosstenant");
    let tenant_a = root.join(".genesis").join("tenant-a").join("genesis-sam");
    let tenant_b = root.join(".genesis").join("tenant-b").join("genesis-sam");

    let docs = vec![ingest_onboarding_document(SAMPLE, "secret-sop.md").expect("ingest")];
    write_onboarding_documents(&tenant_a, &docs).expect("write A");

    // A sees its own document.
    assert_eq!(onboarding_document_labels(&tenant_a), vec!["secret-sop.md"]);
    assert_eq!(read_onboarding_documents(&tenant_a).len(), 1);

    // B (a different tenant segment, nothing written) sees nothing — fail-closed.
    assert!(onboarding_document_labels(&tenant_b).is_empty());
    assert!(read_onboarding_documents(&tenant_b).is_empty());

    std::fs::remove_dir_all(&root).ok();
}

const VOICE_SAMPLE: &str = "Hi, I want you to act as our front-desk concierge. \
Greet every visitor by name, offer them water or coffee, and let the host know \
their guest has arrived within two minutes of check-in.";

const IMAGE_SAMPLE: &str = "Screenshot of the escalation board: red lane is for \
Sev-1 outages paged to the on-call engineer, amber lane is for billing disputes \
routed to the finance pod, green lane is for general questions answered inline.";

const WIKI_SAMPLE: &str = "From the internal wiki: refunds above five hundred \
dollars require regional manager approval, and every chargeback dispute must be \
filed with the finance team within twenty-four hours of receipt.";

#[test]
fn source_kind_round_trips_through_disk() {
    let root = temp_root("sourcekind");
    let vdir = root.join(".genesis").join("acme").join("genesis-multi");
    let docs = vec![
        ingest_onboarding_document_with_kind(VOICE_SAMPLE, "intro.wav", SourceKind::Voice)
            .expect("voice"),
        ingest_onboarding_document_with_kind(IMAGE_SAMPLE, "board.png", SourceKind::Image)
            .expect("image"),
        ingest_onboarding_document_with_kind(WIKI_SAMPLE, "wiki.html", SourceKind::WikiUrl)
            .expect("wiki"),
        ingest_onboarding_document(SAMPLE, "return-policy.md").expect("doc"),
    ];
    write_onboarding_documents(&vdir, &docs).expect("write");

    let read = read_onboarding_documents(&vdir);
    assert_eq!(read.len(), 4);
    assert_eq!(read[0].source_kind, SourceKind::Voice);
    assert_eq!(read[1].source_kind, SourceKind::Image);
    assert_eq!(read[2].source_kind, SourceKind::WikiUrl);
    // The plain text upload defaults to Document.
    assert_eq!(read[3].source_kind, SourceKind::Document);

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn legacy_index_without_kind_defaults_to_document() {
    // A Phase-1 index.toml predates the `kind` field; it must still load (as
    // Document) so the additive change never breaks an existing tenant store.
    let root = temp_root("legacyindex");
    let dir = root
        .join(".genesis")
        .join("acme")
        .join("genesis-legacy")
        .join("knowledge")
        .join("onboarding-documents");
    std::fs::create_dir_all(&dir).expect("dir");
    std::fs::write(
        dir.join("00-policy.md"),
        format!("# Onboarding document: policy.md\n\n{SAMPLE}\n"),
    )
    .expect("body");
    // No `kind` key on the entry — the legacy shape.
    std::fs::write(
        dir.join("index.toml"),
        "[[document]]\nsource_label = \"policy.md\"\nfile = \"00-policy.md\"\nchunks = 1\n",
    )
    .expect("index");

    let vdir = root.join(".genesis").join("acme").join("genesis-legacy");
    let read = read_onboarding_documents(&vdir);
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].source_kind, SourceKind::Document);

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn cross_tenant_multimodal_does_not_leak() {
    // Phase-2: tenant A's voice / image / wiki-URL intake must be just as
    // isolated as Phase-1 text docs — tenant B reads nothing.
    let root = temp_root("crosstenant-mm");
    let tenant_a = root.join(".genesis").join("tenant-a").join("genesis-multi");
    let tenant_b = root.join(".genesis").join("tenant-b").join("genesis-multi");

    let docs = vec![
        ingest_onboarding_document_with_kind(VOICE_SAMPLE, "intro.wav", SourceKind::Voice)
            .expect("voice"),
        ingest_onboarding_document_with_kind(IMAGE_SAMPLE, "board.png", SourceKind::Image)
            .expect("image"),
        ingest_onboarding_document_with_kind(WIKI_SAMPLE, "wiki.html", SourceKind::WikiUrl)
            .expect("wiki"),
    ];
    write_onboarding_documents(&tenant_a, &docs).expect("write A");

    assert_eq!(read_onboarding_documents(&tenant_a).len(), 3);
    assert!(read_onboarding_documents(&tenant_b).is_empty());
    assert!(onboarding_document_labels(&tenant_b).is_empty());

    std::fs::remove_dir_all(&root).ok();
}
