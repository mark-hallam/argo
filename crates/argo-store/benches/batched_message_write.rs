//! Criterion bench for the batched message-append API
//! (Stream `claude-perf-store-batched-writes`). The brief's
//! premise is that under swarm load — many concurrent runners
//! appending messages — the per-row transaction path saturates
//! SQLite's WAL writer lock; coalescing through one transaction
//! per batch removes that contention.
//!
//! Two cases under one group, each appending `MEASURED_MESSAGES`
//! rows to the same warm store:
//!
//! - `unbatched_serial` — `ARGO_STORE_BATCHED_WRITES=0`, single
//!   submitter writing one row per transaction. The historical
//!   baseline; matches `single` in
//!   `argo-cli/benches/message_write.rs`.
//! - `batched_concurrent_8` — default batched path, 8 concurrent
//!   submitters (the brief's "8 parallel runners" target). Each
//!   `submit().await` enqueues into the per-store
//!   `BatchedWriter` and the drain coalesces overlapping
//!   submitters into one transaction per batch.
//!
//! Run locally:
//!   `cargo bench -p argo-store --bench batched_message_write`
//!
//! NOT wired into CI (compile-only via `cargo build --benches`).
use argo_store::SessionStore;
use argo_types::{ArgoResult, MessageRole, SessionId, TenantId, WorkspaceId};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use std::fmt::Display;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::runtime::Runtime;

const MEASURED_MESSAGES: usize = 1_000;
const CONCURRENT_SUBMITTERS: usize = 8;
const SAMPLE_CONTENT: &str =
    r#"{"text":"bench message body — fixed-size payload to keep per-row work constant"}"#;

struct BenchStore {
    _dir: TempDir,
    store: Arc<SessionStore>,
    session_id: SessionId,
}

fn batched_throughput(c: &mut Criterion) {
    let runtime = must(Runtime::new(), "tokio runtime");
    let mut group = c.benchmark_group("batched_throughput");
    group.sample_size(20);
    group.throughput(Throughput::Elements(MEASURED_MESSAGES as u64));

    group.bench_function("unbatched_serial", |b| {
        // SAFETY: bench harness is single-threaded inside this
        // closure. We pin batching off for the whole bench function
        // and remove the var afterwards.
        unsafe { std::env::set_var("ARGO_STORE_BATCHED_WRITES", "0") };
        b.iter_batched_ref(
            || build_store(&runtime),
            |bench| {
                must(
                    runtime.block_on(submit_serial(bench, MEASURED_MESSAGES)),
                    "unbatched serial submit",
                );
            },
            BatchSize::PerIteration,
        );
        unsafe { std::env::remove_var("ARGO_STORE_BATCHED_WRITES") };
    });

    group.bench_function("batched_concurrent_8", |b| {
        unsafe { std::env::remove_var("ARGO_STORE_BATCHED_WRITES") };
        b.iter_batched_ref(
            || build_store(&runtime),
            |bench| {
                must(
                    runtime.block_on(submit_concurrent(bench, MEASURED_MESSAGES)),
                    "batched concurrent submit",
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
    let (tenant_id, workspace_id) = must(
        runtime.block_on(store.ensure_workspace("bench", &workspace_root)),
        "ensure workspace",
    );
    let session_id = must(
        runtime.block_on(create_bench_session(&store, tenant_id, workspace_id)),
        "create bench session",
    );
    BenchStore {
        _dir: dir,
        store: Arc::new(store),
        session_id,
    }
}

async fn create_bench_session(
    store: &SessionStore,
    tenant_id: TenantId,
    workspace_id: WorkspaceId,
) -> ArgoResult<SessionId> {
    store
        .create_session(tenant_id, workspace_id, "mock", "mock", Some("bench"), None)
        .await
}

async fn submit_serial(bench: &BenchStore, count: usize) -> ArgoResult<()> {
    for _ in 0..count {
        bench
            .store
            .append_message(bench.session_id, MessageRole::User, SAMPLE_CONTENT)
            .await?;
    }
    Ok(())
}

async fn submit_concurrent(bench: &BenchStore, count: usize) -> ArgoResult<()> {
    let per_worker = count.div_ceil(CONCURRENT_SUBMITTERS);
    let mut handles = Vec::with_capacity(CONCURRENT_SUBMITTERS);
    let session_id = bench.session_id;
    for _ in 0..CONCURRENT_SUBMITTERS {
        let store = Arc::clone(&bench.store);
        handles.push(tokio::spawn(async move {
            for _ in 0..per_worker {
                store
                    .append_message(session_id, MessageRole::User, SAMPLE_CONTENT)
                    .await?;
            }
            Ok::<(), argo_types::ArgoError>(())
        }));
    }
    for h in handles {
        match h.await {
            Ok(r) => r?,
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

criterion_group!(benches, batched_throughput);
criterion_main!(benches);
