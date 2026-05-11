//! Criterion bench for `argo_skill::select_variant` — the
//! deterministic SHA-256 50/50 split that routes A/B test
//! invocations. Stream `mission-criterion-load-tests`.
//!
//! Why this matters: `select_variant` runs once per skill
//! invocation under an active A/B test. It is microsecond-scale,
//! so 10k iterations are required for criterion to extract real
//! signal. A regression here would multiply across every A/B-
//! routed invocation in production.
//!
//! Two cases under one group:
//!   * `fixed_run_id` — one run id, varying invocation ids
//!     (the steady-state case: one A/B test running across many
//!     calls).
//!   * `varying_run_id` — both run id and invocation id change
//!     each call (multi-tenant: many A/B tests in flight at once).
//!
//! Brief drift note: the brief framed this as "weighted-random
//! selection over N variants"; the actual `select_variant` API
//! is a deterministic binary A/B split. Bench what exists.
//!
//! Run locally:
//!   `cargo bench -p argo-skill --bench select_variant`
//!
//! NOT wired into CI (compile-only via `cargo build --benches`).
use argo_skill::select_variant;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

const ITERATIONS: usize = 10_000;
const FIXED_RUN_ID: &str = "ab-run-01926cc0-7a02-7c80-9e7d-cafe00000001";

fn ab_decision(c: &mut Criterion) {
    let mut group = c.benchmark_group("select_variant");
    group.throughput(Throughput::Elements(ITERATIONS as u64));
    group.sample_size(50);

    let invocations = build_invocation_ids(ITERATIONS);

    group.bench_function("fixed_run_id", |b| {
        b.iter(|| {
            for invocation in &invocations {
                let v = select_variant(black_box(FIXED_RUN_ID), black_box(invocation));
                black_box(v);
            }
        });
    });

    let runs = build_run_ids(ITERATIONS);
    group.bench_function("varying_run_id", |b| {
        b.iter(|| {
            for (run, invocation) in runs.iter().zip(invocations.iter()) {
                let v = select_variant(black_box(run), black_box(invocation));
                black_box(v);
            }
        });
    });

    group.finish();
}

fn build_invocation_ids(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("inv-{i:010}")).collect()
}

fn build_run_ids(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("ab-run-{i:08}-{:08}", i.wrapping_mul(0x9E37_79B9))).collect()
}

criterion_group!(benches, ab_decision);
criterion_main!(benches);
