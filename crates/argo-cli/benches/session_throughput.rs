//! First Criterion bench in the tree — sessions/sec on
//! `SessionStore::create_session`. See
//! `docs/build-plan/streams/claude-bench-session-throughput.md`.
//!
//! Two cases under one group:
//!   * `cold_store_first_1000` — fresh tempdir + fresh sqlite,
//!     measures throughput inserting the first MEASURED_SESSIONS rows.
//!   * `warm_store_next_1000` — pre-seed WARM_SEED_SESSIONS rows, then
//!     measure the next MEASURED_SESSIONS.
//!
//! Run locally:
//!   `cargo bench -p argo-cli --bench session_throughput`
//!
//! NOT wired into CI (compile-only via `cargo build --benches`).
use argo_store::SessionStore;
use argo_types::{TenantId, WorkspaceId};
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::fmt::Display;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::runtime::Runtime;

const MEASURED_SESSIONS: usize = 1_000;
const WARM_SEED_SESSIONS: usize = 10_000;

struct BenchStore {
    _dir: TempDir,
    store: SessionStore,
    tenant_id: TenantId,
    workspace_id: WorkspaceId,
}

fn session_create_throughput(c: &mut Criterion) {
    let runtime = must(Runtime::new(), "tokio runtime");
    let mut group = c.benchmark_group("session_create_throughput");
    group.sample_size(50);
    group.throughput(Throughput::Elements(MEASURED_SESSIONS as u64));

    group.bench_function("cold_store_first_1000", |b| {
        b.iter_custom(|iters| {
            let mut elapsed = Duration::ZERO;
            for batch in 0..iters {
                let bench = build_store(&runtime);
                let start_idx = batch as usize * MEASURED_SESSIONS;
                let started_at = Instant::now();
                must(
                    runtime.block_on(create_sessions(&bench, start_idx, MEASURED_SESSIONS)),
                    "cold session create batch",
                );
                elapsed += started_at.elapsed();
            }
            elapsed
        });
    });

    let warm_bench = build_store(&runtime);
    must(
        runtime.block_on(create_sessions(&warm_bench, 0, WARM_SEED_SESSIONS)),
        "warm seed sessions",
    );
    let mut next_warm_session = WARM_SEED_SESSIONS;
    group.bench_function("warm_store_next_1000", |b| {
        b.iter_custom(|iters| {
            let mut elapsed = Duration::ZERO;
            for _ in 0..iters {
                let start_idx = next_warm_session;
                next_warm_session += MEASURED_SESSIONS;
                let started_at = Instant::now();
                must(
                    runtime.block_on(create_sessions(&warm_bench, start_idx, MEASURED_SESSIONS)),
                    "warm session create batch",
                );
                elapsed += started_at.elapsed();
            }
            elapsed
        });
    });

    group.finish();
}

fn build_store(runtime: &Runtime) -> BenchStore {
    let dir = must(TempDir::new(), "tempdir");
    let db_path = dir.path().join("argo.db");
    let workspace_root = dir.path().join("workspace").to_string_lossy().to_string();
    let store = must(runtime.block_on(SessionStore::open(&db_path)), "open store");
    must(runtime.block_on(store.migrate()), "migrate store");
    let (tenant_id, workspace_id) = must(
        runtime.block_on(store.ensure_workspace("bench", &workspace_root)),
        "ensure workspace",
    );
    BenchStore {
        _dir: dir,
        store,
        tenant_id,
        workspace_id,
    }
}

async fn create_sessions(
    bench: &BenchStore,
    start: usize,
    count: usize,
) -> argo_types::ArgoResult<()> {
    for idx in start..start + count {
        bench
            .store
            .create_session(
                bench.tenant_id,
                bench.workspace_id,
                "mock",
                "mock",
                Some(&format!("bench-session-{idx}")),
                None,
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

criterion_group!(benches, session_create_throughput);
criterion_main!(benches);
