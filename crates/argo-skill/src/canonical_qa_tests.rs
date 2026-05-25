//! Unit tests for the `canonical_qa` matcher (Track P brief P-04).
//!
//! Covers paraphrase matching, threshold rejection, audience
//! filtering, cross-tenant isolation, corpus-loader rejection cases,
//! and SDK envelope serialization.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::canonical_qa::{
    CanonicalEntry, CanonicalMatch, CanonicalQaIndex, CanonicalQaOutput,
    DEFAULT_SIMILARITY_THRESHOLD, MIN_PROMPTS_PER_ENTRY,
};
use crate::canonical_qa_score::{build_idf, score, tokenize};
use argo_types::{ArgoError, TenantId};
use chrono::NaiveDate;
use tempfile::tempdir;

fn entry(id: &str, prompts: &[&str], answer: &str) -> CanonicalEntry {
    CanonicalEntry {
        id: id.into(),
        prompts: prompts.iter().map(|s| (*s).to_string()).collect(),
        answer: answer.into(),
        declared_claim_classes: vec![],
        audience_tags: vec![],
        last_reviewed: NaiveDate::from_ymd_opt(2026, 5, 24).unwrap(),
        review_owner: "founder@example.com".into(),
    }
}

fn entry_with(
    id: &str,
    prompts: &[&str],
    answer: &str,
    classes: &[&str],
    audiences: &[&str],
) -> CanonicalEntry {
    let mut e = entry(id, prompts, answer);
    e.declared_claim_classes = classes.iter().map(|s| (*s).to_string()).collect();
    e.audience_tags = audiences.iter().map(|s| (*s).to_string()).collect();
    e
}

fn corpus_skycot() -> Vec<CanonicalEntry> {
    vec![
        entry_with(
            "lindy-comparison",
            &[
                "How is Skycot different from Lindy?",
                "How do you compare to Lindy?",
                "What's the difference between you and Lindy?",
                "Is this just Lindy?",
            ],
            "Lindy is a workflow tool. Skycot is the workforce platform.",
            &["competitive-positioning"],
            &["investor", "buyer"],
        ),
        entry_with(
            "valuation-deferral",
            &[
                "What's your valuation?",
                "What are you valued at?",
                "What's the round at?",
            ],
            "I'm not authorized to share valuation publicly.",
            &["valuation"],
            &["investor", "press"],
        ),
        entry_with(
            "pricing-basics",
            &[
                "How much does Skycot cost?",
                "What is the pricing model?",
                "How do you charge?",
            ],
            "Per-worker monthly subscription plus usage-based overage.",
            &["pricing"],
            &["buyer"],
        ),
    ]
}

#[test]
fn lookup_matches_paraphrase() {
    let idx = CanonicalQaIndex::from_entries(
        corpus_skycot(),
        TenantId::new(),
        DEFAULT_SIMILARITY_THRESHOLD,
    )
    .unwrap();
    let m = idx.lookup("How do you compare to Lindy?", "investor").unwrap();
    assert_eq!(m.entry_id, "lindy-comparison");
    assert!(m.similarity >= DEFAULT_SIMILARITY_THRESHOLD);
}

#[test]
fn lookup_matches_close_paraphrase_not_in_prompts() {
    let idx = CanonicalQaIndex::from_entries(
        corpus_skycot(),
        TenantId::new(),
        DEFAULT_SIMILARITY_THRESHOLD,
    )
    .unwrap();
    // "Skycot vs Lindy" hits the rare-token "lindy" + "skycot" — IDF
    // weighting should clear the threshold even with no stopword
    // overlap.
    let m = idx.lookup("Skycot versus Lindy?", "").unwrap();
    assert_eq!(m.entry_id, "lindy-comparison");
}

#[test]
fn lookup_returns_none_below_threshold() {
    let idx = CanonicalQaIndex::from_entries(
        corpus_skycot(),
        TenantId::new(),
        DEFAULT_SIMILARITY_THRESHOLD,
    )
    .unwrap();
    // Off-topic, no shared rare tokens with any entry.
    assert!(idx
        .lookup("What's the weather in Melbourne tomorrow?", "")
        .is_none());
}

#[test]
fn audience_filter_skips_off_audience_entries() {
    let idx = CanonicalQaIndex::from_entries(
        corpus_skycot(),
        TenantId::new(),
        DEFAULT_SIMILARITY_THRESHOLD,
    )
    .unwrap();
    // pricing-basics has audience_tags = ["buyer"]; an investor
    // querying about cost should miss this entry.
    assert!(idx.lookup("How much does Skycot cost?", "investor").is_none());
    // Same query for the right audience hits.
    assert!(idx.lookup("How much does Skycot cost?", "buyer").is_some());
    // Empty audience bypasses the filter.
    assert!(idx.lookup("How much does Skycot cost?", "").is_some());
}

#[test]
fn cross_tenant_indexes_are_independent() {
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let idx_a = CanonicalQaIndex::from_entries(
        corpus_skycot(),
        tenant_a,
        DEFAULT_SIMILARITY_THRESHOLD,
    )
    .unwrap();
    // Tenant B's index is built from a disjoint corpus.
    let idx_b = CanonicalQaIndex::from_entries(
        vec![entry(
            "tenant-b-only",
            &[
                "What's the capital of France?",
                "France's capital city?",
                "Paris is the capital of which country?",
            ],
            "Paris.",
        )],
        tenant_b,
        DEFAULT_SIMILARITY_THRESHOLD,
    )
    .unwrap();
    // A's query that matches in A's corpus must NOT surface in B.
    assert!(idx_a.lookup("How do you compare to Lindy?", "").is_some());
    assert!(idx_b.lookup("How do you compare to Lindy?", "").is_none());
    assert_ne!(idx_a.tenant_id(), idx_b.tenant_id());
}

#[test]
fn loader_rejects_entry_with_too_few_prompts() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("bad.toml");
    std::fs::write(
        &path,
        r#"
[[entries]]
id = "thin"
prompts = ["only one"]
answer = "ok"
declared_claim_classes = []
last_reviewed = "2026-05-24"
"#,
    )
    .unwrap();
    let err = CanonicalQaIndex::load(dir.path(), TenantId::new()).unwrap_err();
    match err {
        ArgoError::Schema(msg) => {
            assert!(msg.contains("minimum is"));
            assert!(msg.contains(&MIN_PROMPTS_PER_ENTRY.to_string()));
        }
        other => panic!("expected Schema error, got {other:?}"),
    }
}

#[test]
fn loader_rejects_malformed_toml() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("broken.toml");
    std::fs::write(&path, "this is not valid toml [[[").unwrap();
    let err = CanonicalQaIndex::load(dir.path(), TenantId::new()).unwrap_err();
    assert!(matches!(err, ArgoError::Schema(_)));
}

#[test]
fn loader_rejects_empty_id() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("empty-id.toml");
    std::fs::write(
        &path,
        r#"
[[entries]]
id = ""
prompts = ["a b", "c d", "e f"]
answer = "ok"
declared_claim_classes = []
last_reviewed = "2026-05-24"
"#,
    )
    .unwrap();
    let err = CanonicalQaIndex::load(dir.path(), TenantId::new()).unwrap_err();
    match err {
        ArgoError::Schema(msg) => assert!(msg.contains("empty id")),
        other => panic!("expected Schema error, got {other:?}"),
    }
}

#[test]
fn loader_rejects_duplicate_ids() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("dups.toml");
    std::fs::write(
        &path,
        r#"
[[entries]]
id = "same"
prompts = ["a b", "c d", "e f"]
answer = "first"
declared_claim_classes = []
last_reviewed = "2026-05-24"

[[entries]]
id = "same"
prompts = ["g h", "i j", "k l"]
answer = "second"
declared_claim_classes = []
last_reviewed = "2026-05-24"
"#,
    )
    .unwrap();
    let err = CanonicalQaIndex::load(dir.path(), TenantId::new()).unwrap_err();
    match err {
        ArgoError::Schema(msg) => assert!(msg.contains("duplicate entry id")),
        other => panic!("expected Schema error, got {other:?}"),
    }
}

#[test]
fn loader_reads_real_corpus_file_round_trip() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("real.toml");
    std::fs::write(
        &path,
        r#"
[[entries]]
id = "lindy-comparison"
prompts = [
  "How is Skycot different from Lindy?",
  "How do you compare to Lindy?",
  "Skycot versus Lindy?",
]
answer = "Different category, different buyer."
declared_claim_classes = ["competitive-positioning"]
audience_tags = ["investor"]
last_reviewed = "2026-05-24"
review_owner = "founder@example.com"
"#,
    )
    .unwrap();
    let idx = CanonicalQaIndex::load(dir.path(), TenantId::new()).unwrap();
    assert_eq!(idx.len(), 1);
    let m = idx
        .lookup("How do you compare to Lindy?", "investor")
        .unwrap();
    assert_eq!(m.entry_id, "lindy-comparison");
    assert_eq!(
        m.declared_claim_classes,
        vec!["competitive-positioning".to_string()]
    );
}

#[test]
fn match_propagates_declared_claim_classes() {
    let idx = CanonicalQaIndex::from_entries(
        corpus_skycot(),
        TenantId::new(),
        DEFAULT_SIMILARITY_THRESHOLD,
    )
    .unwrap();
    let m = idx.lookup("What's your valuation?", "investor").unwrap();
    assert_eq!(m.entry_id, "valuation-deferral");
    assert_eq!(m.declared_claim_classes, vec!["valuation".to_string()]);
}

#[test]
fn tokenizer_drops_stopwords_and_punctuation() {
    let toks = tokenize("How is Skycot different from Lindy?");
    assert!(toks.contains("skycot"));
    assert!(toks.contains("lindy"));
    assert!(toks.contains("different"));
    assert!(!toks.contains("how"));
    assert!(!toks.contains("is"));
    assert!(!toks.contains("from"));
}

#[test]
fn scorer_zero_on_disjoint_token_sets() {
    let q = tokenize("paris france capital");
    let p = tokenize("lindy comparison workflow");
    let prompts = vec![p.clone()];
    let idf = build_idf(&prompts);
    assert!(score(&q, &p, &idf) < 0.1);
}

#[test]
fn scorer_high_on_rare_token_match() {
    let q = tokenize("how do skycot and lindy differ");
    let p = tokenize("how is skycot different from lindy");
    let prompts = vec![p.clone()];
    let idf = build_idf(&prompts);
    let s = score(&q, &p, &idf);
    assert!(
        s >= DEFAULT_SIMILARITY_THRESHOLD,
        "expected score >= threshold, got {s}"
    );
}

#[test]
fn real_skycot_corpus_loads_and_serves_demo_queries() {
    // The shipped corpus under skill-library/canonical-qa/corpora/skycot
    // must load cleanly and return hits for the demo storyboard's
    // headline objection-drill queries. Drift-detector for the
    // corpus + the loader.
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("skill-library/canonical-qa/corpora/skycot");
    if !root.exists() {
        // Corpus not present in this checkout (e.g. when the crate
        // is consumed as a dependency from outside the monorepo).
        return;
    }
    let idx = CanonicalQaIndex::load(&root, TenantId::new()).unwrap();
    assert!(idx.len() >= 30, "corpus has {} entries, want ≥30", idx.len());
    let lindy = idx.lookup("How do you compare to Lindy?", "investor").unwrap();
    assert_eq!(lindy.entry_id, "lindy-comparison");
    let valuation = idx.lookup("What's your valuation?", "investor").unwrap();
    assert_eq!(valuation.entry_id, "valuation-deferral");
    assert_eq!(
        valuation.declared_claim_classes,
        vec!["valuation".to_string()]
    );
}

#[test]
fn output_envelope_serializes_consistently() {
    let m = CanonicalMatch {
        entry_id: "lindy-comparison".into(),
        answer: "x".into(),
        declared_claim_classes: vec!["competitive-positioning".into()],
        similarity: 0.83,
    };
    let out: CanonicalQaOutput = m.into();
    let json = serde_json::to_string(&out).unwrap();
    assert!(json.contains("\"matched\":true"));
    assert!(json.contains("\"entry_id\":\"lindy-comparison\""));
    let miss = CanonicalQaOutput::miss();
    let miss_json = serde_json::to_string(&miss).unwrap();
    assert!(miss_json.contains("\"matched\":false"));
}
