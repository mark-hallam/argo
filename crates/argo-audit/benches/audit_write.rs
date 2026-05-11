//! Criterion bench for audit-write throughput. Stream
//! `mission-criterion-load-tests`.
//!
//! Why this matters: every agent turn writes 2-6 audit rows
//! (turn_start, assistant_message, tool_call, tool_result, …).
//! Audit writes are on the per-turn hot path; a regression in
//! their sustained write rate caps the agent loop's throughput.
//!
//! Two cases under one group:
//!   * `sequential` — one-at-a-time appends to a fresh sqlite
//!     store. Baseline single-writer rate.
//!   * `parallel_8` — 8 concurrent appenders via `tokio::task::
//!     JoinSet` to the SAME sqlite store. Exercises the WAL
//!     write-path's contention behavior.
//!
//! Run locally:
//!   `cargo bench -p argo-audit --bench audit_write`
//!
//! NOT wired into CI (compile-only via `cargo build --benches`).
use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use std::fmt::Display;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use tokio::task::JoinSet;

const MEASURED_ROWS: usize = 1_000;
const PARALLEL_WORKERS: usize = 8;

struct BenchSink {
    _dir: TempDir,
    sink: Arc<AuditSink>,
}

fn audit_write_throughput(c: &mut Criterion) {
    let runtime = must(Runtime::new(), "tokio runtime");
    let mut group = c.benchmark_group("audit_write");
    group.sample_size(20);
    group.throughput(Throughput::Elements(MEASURED_ROWS as u64));

    group.bench_function("sequential", |b| {
        b.iter_batched_ref(
            || build_sink(&runtime),
            |bench| {
                must(
                    runtime.block_on(write_sequential(&bench.sink, MEASURED_ROWS)),
                    "sequential write",
                );
            },
            BatchSize::PerIteration,
        );
    });

    group.bench_function("parallel_8", |b| {
        b.iter_batched_ref(
            || build_sink(&runtime),
            |bench| {
                must(
                    runtime.block_on(write_parallel(
                        Arc::clone(&bench.sink),
                        MEASURED_ROWS,
                        PARALLEL_WORKERS,
                    )),
                    "parallel write",
                );
            },
            BatchSize::PerIteration,
        );
    });

    group.finish();
}

fn build_sink(runtime: &Runtime) -> BenchSink {
    let dir = must(TempDir::new(), "tempdir");
    let db_path = dir.path().join("argo.db");
    let store = must(runtime.block_on(SessionStore::open(&db_path)), "open store");
    must(runtime.block_on(store.migrate()), "migrate store");
    BenchSink {
        _dir: dir,
        sink: Arc::new(AuditSink::new(Arc::new(store))),
    }
}

async fn write_sequential(sink: &AuditSink, count: usize) -> argo_types::ArgoResult<()> {
    for i in 0..count {
        let payload = serde_json::json!({"k": i, "label": "bench"});
        sink.write(None, black_box(AuditKind::ToolCall), payload, None)
            .await?;
    }
    Ok(())
}

async fn write_parallel(
    sink: Arc<AuditSink>,
    total: usize,
    workers: usize,
) -> argo_types::ArgoResult<()> {
    let per_worker = total / workers;
    let mut tasks = JoinSet::new();
    for w in 0..workers {
        let s = Arc::clone(&sink);
        tasks.spawn(async move {
            for i in 0..per_worker {
                let payload = serde_json::json!({"w": w, "k": i, "label": "bench"});
                s.write(None, AuditKind::ToolCall, payload, None).await?;
            }
            Ok::<(), argo_types::ArgoError>(())
        });
    }
    while let Some(res) = tasks.join_next().await {
        match res {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(argo_types::ArgoError::Provider(format!("join: {e}"))),
        }
    }
    Ok(())
}

fn must<T, E: Display>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("{context}: {error}"),
    }
}

criterion_group!(benches, audit_write_throughput);
criterion_main!(benches);
