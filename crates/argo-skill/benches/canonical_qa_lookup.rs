//! Criterion bench for `argo_skill::canonical_qa::CanonicalQaIndex::lookup`
//! — Track P brief P-04. Asserts hot-path lookup stays well under
//! the storyboard's ≤50ms p95 budget on a 100-entry corpus.
//!
//! Why this matters: the Pitch Agent's sub-300ms voice path needs
//! the canonical-qa lookup to be a rounding error in the latency
//! budget — anything above ~5ms eats into the speech-synthesis +
//! audio-pipeline budget. A regression here would push the
//! whole hot path out of feel-snappy range.
//!
//! Bench builds a synthetic 100-entry corpus (each entry with 4
//! prompts) and measures one lookup. Per-call work is two: tokenise
//! the query once, score it against every prompt.
//!
//! Run locally:
//!   `cargo bench -p argo-skill --bench canonical_qa_lookup`
//!
//! NOT wired into CI (compile-only via `cargo build --benches`).

use argo_skill::{CanonicalEntry, CanonicalQaIndex, CANONICAL_QA_DEFAULT_THRESHOLD};
use argo_types::TenantId;
use chrono::NaiveDate;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

const ENTRY_COUNT: usize = 100;

fn synthetic_corpus() -> Vec<CanonicalEntry> {
    let date = NaiveDate::from_ymd_opt(2026, 5, 24).unwrap();
    (0..ENTRY_COUNT)
        .map(|i| CanonicalEntry {
            id: format!("entry-{i:03}"),
            prompts: vec![
                format!("How does feature {i} work?"),
                format!("Tell me about feature {i}."),
                format!("What is feature {i} for?"),
                format!("Explain feature {i} please."),
            ],
            answer: format!("Feature {i} is the {i}th feature in the synthetic corpus."),
            declared_claim_classes: vec!["product".to_string()],
            audience_tags: vec!["investor".to_string()],
            last_reviewed: date,
            review_owner: "bench@skycot.com".to_string(),
        })
        .collect()
}

fn bench(c: &mut Criterion) {
    let idx = CanonicalQaIndex::from_entries(
        synthetic_corpus(),
        TenantId::new(),
        CANONICAL_QA_DEFAULT_THRESHOLD,
    )
    .expect("synthetic corpus must validate");
    let query = "How does feature 42 work?";
    c.bench_function("canonical_qa_lookup_100_entries", |b| {
        b.iter(|| {
            let r = idx.lookup(black_box(query), black_box("investor"));
            black_box(r)
        });
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
