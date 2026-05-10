//! Criterion bench for `MemoryStore::compose_context` latency.
//!
//! `compose_context` runs on every agent turn — see
//! `crates/argo-memory/src/layers.rs::compose_context`. The brief
//! `claude-bench-memory-query-perf` installs this bench so future
//! memory-layer changes can be compared against a stable p50/p95/p99
//! baseline.
//!
//! Three group cases, each measuring single-call latency:
//!
//!   * `small` —   100 entries seeded.
//!   * `medium` — 10 000 entries seeded.
//!   * `large` —  100 000 entries seeded.
//!
//! Each case spreads its rows deterministically across the three
//! storage layers (`recent`, `indexed`, `summary`) and across a
//! handful of tag-like phrases so FTS scoring + the layered fan-out
//! both have realistic work to do. The embedding provider used by
//! `compose_context` is `Voyage3EmbeddingProvider`, whose `embed`
//! impl is purely deterministic (sha256-based hash embedding) — no
//! network, no randomness, so bench numbers are reproducible
//! across runs.
//!
//! Run locally:
//!   `cargo bench -p argo-memory --bench query_perf`
//!
//! NOT wired into CI (compile-only via `cargo build --benches`).
use argo_identity::{MemoryNs, MemoryScope, WorkerId};
use argo_memory::{MemoryFile, MemoryLayer, MemoryStore};
use argo_store::SessionStore;
use argo_types::{TenantId, WorkspaceId};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

const QUERY_ACTION: &str = "refund_escalation";
const TAG_PHRASES: &[&str] = &[
    "refund escalation playbook",
    "soc2 evidence request",
    "enterprise renewal blocker",
    "shipping address mismatch",
    "duplicate charge investigation",
    "billing migration follow-up",
    "lost package replacement policy",
    "premium tier upgrade triage",
];
const LAYERS: &[MemoryLayer] = &[MemoryLayer::Recent, MemoryLayer::Indexed, MemoryLayer::Summary];

fn compose_context_latency(c: &mut Criterion) {
    let runtime = must(Runtime::new(), "tokio runtime");

    let mut group = c.benchmark_group("compose_context_latency");
    group.measurement_time(Duration::from_secs(10));
    group.throughput(Throughput::Elements(1));

    // Per-case sample size. The brief's 100-sample target assumes
    // ms-scale calls; at 10k+ seeded entries `compose_context`
    // re-embeds all indexed rows per call (`refresh_indexed_embeddings`
    // — see `layers_indexing.rs`), so per-call latency is seconds.
    // We cap medium/large samples to keep the bench tractable while
    // still letting Criterion publish p50/p95/p99 with usable
    // confidence.
    for (label, n_entries, samples) in [
        ("small", 100, 100),
        ("medium", 10_000, 30),
        ("large", 100_000, 10),
    ] {
        let bench = build_seeded_store(&runtime, n_entries);
        group.sample_size(samples);
        group.bench_function(label, |b| {
            b.iter_batched_ref(
                || (),
                |_| {
                    must(
                        runtime.block_on(bench.memory.compose_context(&bench.ns, QUERY_ACTION)),
                        "compose_context",
                    );
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

struct BenchStore {
    memory: Arc<MemoryStore>,
    ns: MemoryNs,
}

fn build_seeded_store(runtime: &Runtime, n_entries: usize) -> BenchStore {
    let store = must(runtime.block_on(SessionStore::open(":memory:")), "open store");
    must(runtime.block_on(store.migrate()), "migrate store");
    let tenant = TenantId::new();
    let workspace_id = WorkspaceId::new();
    must(
        runtime.block_on(insert_workspace(&store, workspace_id, tenant)),
        "insert workspace",
    );
    let memory = Arc::new(MemoryStore::new(Arc::new(store)));
    let ns = MemoryNs::new(tenant, WorkerId::new(), MemoryScope::WorkerSelf);
    must(
        runtime.block_on(seed_entries(&memory, &ns, workspace_id, n_entries)),
        "seed entries",
    );
    BenchStore { memory, ns }
}

async fn insert_workspace(
    store: &SessionStore,
    workspace_id: WorkspaceId,
    tenant: TenantId,
) -> argo_types::ArgoResult<()> {
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at) \
         VALUES (?, ?, 'bench-memory-query', '.', strftime('%s','now'))",
    )
    .bind(workspace_id.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .map_err(|e| argo_types::ArgoError::Provider(format!("bench insert workspace: {e}")))?;
    Ok(())
}

async fn seed_entries(
    memory: &MemoryStore,
    ns: &MemoryNs,
    workspace_id: WorkspaceId,
    n_entries: usize,
) -> argo_types::ArgoResult<()> {
    for idx in 0..n_entries {
        let layer = LAYERS[idx % LAYERS.len()];
        let phrase = TAG_PHRASES[idx % TAG_PHRASES.len()];
        // Deterministic body: phrase + sequence number keeps the FTS
        // surface non-trivial without relying on randomness.
        let body = format!("{phrase} note #{idx}");
        memory
            .append_in_namespace_with_layer(
                ns,
                workspace_id,
                MemoryFile::Memory,
                body,
                None,
                layer,
            )
            .await?;
    }
    Ok(())
}

fn must<T, E: Display>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("{context}: {error}"),
    }
}

criterion_group!(benches, compose_context_latency);
criterion_main!(benches);
