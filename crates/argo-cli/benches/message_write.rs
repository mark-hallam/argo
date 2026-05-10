//! Criterion bench for message append throughput. See
//! `docs/build-plan/streams/claude-bench-message-write-perf.md`.
//!
//! Three cases under one group, each appending `MEASURED_MESSAGES`
//! messages to a freshly-warmed sqlite tempdir + single session:
//!   * `single` — `SessionStore::append_message` one-at-a-time
//!     (the current path: one tx + one SELECT MAX(seq) per row).
//!   * `batch_10` — 1000 messages in 100 batches of 10, each
//!     batch wrapped in one transaction.
//!   * `batch_100` — 1000 messages in 10 batches of 100, each
//!     batch wrapped in one transaction.
//!
//! The batch cases simulate a future `append_messages_batch` API
//! by issuing raw INSERTs through `SessionStore::pool()` inside a
//! single `BEGIN`/`COMMIT`. They diverge from `append_message` in
//! two ways for fairness with that future API: seq is computed
//! incrementally in Rust (one MAX(seq) lookup per batch, not per
//! row), and PII detection is skipped (argo-security is not a
//! direct argo-cli dep). Both differences favour the batch path
//! and are why the brief calls these "upper-bound estimates" — the
//! real `claude-perf-store-batched-writes` brief should re-run
//! this bench against its actual API.
//!
//! Run locally:
//!   `cargo bench -p argo-cli --bench message_write`
//!
//! NOT wired into CI (compile-only via `cargo build --benches`).
use argo_store::SessionStore;
use argo_types::{MessageRole, SessionId, TenantId, WorkspaceId};
use chrono::Utc;
use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use std::fmt::Display;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use uuid::Uuid;

const MEASURED_MESSAGES: usize = 1_000;
const SAMPLE_CONTENT: &str =
    r#"{"text":"bench message body — fixed-size payload to keep per-row work constant"}"#;

struct BenchStore {
    _dir: TempDir,
    store: SessionStore,
    session_id: SessionId,
}

fn message_append_throughput(c: &mut Criterion) {
    let runtime = must(Runtime::new(), "tokio runtime");
    let mut group = c.benchmark_group("message_append_throughput");
    group.sample_size(20);
    group.throughput(Throughput::Elements(MEASURED_MESSAGES as u64));

    group.bench_function("single", |b| {
        b.iter_batched_ref(
            || build_store(&runtime),
            |bench| {
                must(
                    runtime.block_on(append_one_at_a_time(bench, MEASURED_MESSAGES)),
                    "single append batch",
                );
            },
            BatchSize::PerIteration,
        );
    });

    group.bench_function("batch_10", |b| {
        b.iter_batched_ref(
            || build_store(&runtime),
            |bench| {
                must(
                    runtime.block_on(append_batched(bench, MEASURED_MESSAGES, 10)),
                    "batch-10 append batch",
                );
            },
            BatchSize::PerIteration,
        );
    });

    group.bench_function("batch_100", |b| {
        b.iter_batched_ref(
            || build_store(&runtime),
            |bench| {
                must(
                    runtime.block_on(append_batched(bench, MEASURED_MESSAGES, 100)),
                    "batch-100 append batch",
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
        store,
        session_id,
    }
}

async fn create_bench_session(
    store: &SessionStore,
    tenant_id: TenantId,
    workspace_id: WorkspaceId,
) -> argo_types::ArgoResult<SessionId> {
    store
        .create_session(tenant_id, workspace_id, "mock", "mock", Some("bench"), None)
        .await
}

async fn append_one_at_a_time(
    bench: &BenchStore,
    count: usize,
) -> argo_types::ArgoResult<()> {
    for _ in 0..count {
        bench
            .store
            .append_message(bench.session_id, MessageRole::User, SAMPLE_CONTENT)
            .await?;
    }
    Ok(())
}

async fn append_batched(
    bench: &BenchStore,
    count: usize,
    batch_size: usize,
) -> argo_types::ArgoResult<()> {
    let pool = bench.store.pool();
    let session_blob = bench.session_id.0.as_bytes().to_vec();
    let mut written = 0;
    while written < count {
        let this_batch = batch_size.min(count - written);
        let mut tx = pool.begin().await.map_err(provider_err("tx begin"))?;
        let next_seq: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM message WHERE session_id = ?",
        )
        .bind(session_blob.as_slice())
        .fetch_one(&mut *tx)
        .await
        .map_err(provider_err("next_seq"))?;
        for offset in 0..this_batch {
            let id = Uuid::now_v7();
            let created_at = Utc::now().timestamp_micros();
            let seq = next_seq + offset as i64;
            sqlx::query(
                "INSERT INTO message (id, session_id, role, content_json, created_at, seq)
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(id.as_bytes().as_slice())
            .bind(session_blob.as_slice())
            .bind("user")
            .bind(SAMPLE_CONTENT)
            .bind(created_at)
            .bind(seq)
            .execute(&mut *tx)
            .await
            .map_err(provider_err("insert message"))?;
        }
        tx.commit().await.map_err(provider_err("tx commit"))?;
        written += this_batch;
    }
    Ok(())
}

fn provider_err(context: &'static str) -> impl Fn(sqlx::Error) -> argo_types::ArgoError {
    move |e| argo_types::ArgoError::Provider(format!("{context}: {e}"))
}

fn must<T, E: Display>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("{context}: {error}"),
    }
}

criterion_group!(benches, message_append_throughput);
criterion_main!(benches);
