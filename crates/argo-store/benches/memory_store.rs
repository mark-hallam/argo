//! Criterion bench for memory-shaped storage primitives. Stream
//! `mission-criterion-load-tests`.
//!
//! Why this matters: append-then-read is the dominant memory
//! access pattern — append on each turn, read on resumption +
//! context-window rebuild. The argo-memory crate wraps these
//! sqlite primitives; benchmarking them at the store layer
//! gives an LLM-independent signal on the underlying WAL +
//! query cost.
//!
//! Brief drift note: the brief named `MemoryStore::append` +
//! `MemoryStore::read_range` (in `argo-memory`); the may-touch
//! list pinned the bench to `crates/argo-store/benches/`.
//! `argo-memory` depends on `argo-store`, so we cannot adopt it
//! as a dev-dep without a circular link. Bench the analogous
//! `SessionStore::append_message` + `SessionStore::list_messages`
//! — the same WAL-tuned write path argo-memory rides under the
//! hood.
//!
//! Two cases under one group:
//!   * `write_then_read_1000` — append 1000 entries, then read
//!     them all back. Baseline batch ingest.
//!   * `interleaved_10x_100` — 10 rounds of (append 100, read
//!     100). Measures cache-warming behavior + index lookup
//!     under interleaved write/read pressure.
//!
//! Run locally:
//!   `cargo bench -p argo-store --bench memory_store`
//!
//! NOT wired into CI (compile-only via `cargo build --benches`).
use argo_store::SessionStore;
use argo_types::{MessageRole, SessionId, TenantId, WorkspaceId};
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use std::fmt::Display;
use tempfile::TempDir;
use tokio::runtime::Runtime;

const BATCH_SIZE: usize = 1_000;
const INTERLEAVED_ROUNDS: usize = 10;
const INTERLEAVED_PER_ROUND: usize = 100;
const SAMPLE_BODY: &str =
    r#"{"text":"bench memory entry — fixed-size payload to keep per-row work constant"}"#;

struct BenchStore {
    _dir: TempDir,
    store: SessionStore,
    session_id: SessionId,
}

fn append_read(c: &mut Criterion) {
    let runtime = must(Runtime::new(), "tokio runtime");
    let mut group = c.benchmark_group("memory_store");
    group.sample_size(15);
    group.throughput(Throughput::Elements(BATCH_SIZE as u64));

    group.bench_function("write_then_read_1000", |b| {
        b.iter_batched_ref(
            || build_store(&runtime),
            |bench| {
                must(
                    runtime.block_on(write_then_read(bench, BATCH_SIZE)),
                    "write_then_read",
                );
            },
            BatchSize::PerIteration,
        );
    });

    group.bench_function("interleaved_10x_100", |b| {
        b.iter_batched_ref(
            || build_store(&runtime),
            |bench| {
                must(
                    runtime.block_on(interleaved(
                        bench,
                        INTERLEAVED_ROUNDS,
                        INTERLEAVED_PER_ROUND,
                    )),
                    "interleaved",
                );
            },
            BatchSize::PerIteration,
        );
    });

    group.finish();
}

fn build_store(runtime: &Runtime) -> BenchStore {
    let dir = must(TempDir::new(), "tempdir");
    let db_path = dir.path().join("argo.db");
    let workspace_root = dir.path().join("workspace").to_string_lossy().to_string();
    let store = must(runtime.block_on(SessionStore::open(&db_path)), "open store");
    must(runtime.block_on(store.migrate()), "migrate store");
    let (tenant_id, workspace_id): (TenantId, WorkspaceId) = must(
        runtime.block_on(store.ensure_workspace("bench", &workspace_root)),
        "ensure workspace",
    );
    let session_id = must(
        runtime.block_on(create_session(&store, tenant_id, workspace_id)),
        "create session",
    );
    BenchStore {
        _dir: dir,
        store,
        session_id,
    }
}

async fn create_session(
    store: &SessionStore,
    tenant_id: TenantId,
    workspace_id: WorkspaceId,
) -> argo_types::ArgoResult<SessionId> {
    store
        .create_session(tenant_id, workspace_id, "mock", "mock", Some("bench"), None)
        .await
}

async fn write_then_read(bench: &BenchStore, count: usize) -> argo_types::ArgoResult<()> {
    for _ in 0..count {
        bench
            .store
            .append_message(bench.session_id, MessageRole::User, SAMPLE_BODY)
            .await?;
    }
    let rows = bench.store.list_messages(bench.session_id).await?;
    black_box(rows);
    Ok(())
}

async fn interleaved(
    bench: &BenchStore,
    rounds: usize,
    per_round: usize,
) -> argo_types::ArgoResult<()> {
    for _ in 0..rounds {
        for _ in 0..per_round {
            bench
                .store
                .append_message(bench.session_id, MessageRole::User, SAMPLE_BODY)
                .await?;
        }
        let rows = bench.store.list_messages(bench.session_id).await?;
        black_box(rows);
    }
    Ok(())
}

fn must<T, E: Display>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("{context}: {error}"),
    }
}

criterion_group!(benches, append_read);
criterion_main!(benches);
