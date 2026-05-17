//! Criterion bench for audit-chain verification and feed-query scale.
//!
//! Run full profiles:
//!   `cargo bench -p argo-audit --bench chain_perf`
//!
//! Run a compile/runtime smoke:
//!   `ARGO_AUDIT_CHAIN_PERF_SMOKE=1 cargo bench -p argo-audit --bench chain_perf -- --test`

#[path = "fixtures/seed_chain.rs"]
mod seed_chain;

use argo_audit::{verify_all, verify_session};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use seed_chain::SeededAudit;
use std::fmt::Display;
use tempfile::TempDir;
use tokio::runtime::Runtime;

const RAW_FETCH_CAP: i64 = 5_000;

struct BenchStore {
    _dir: TempDir,
    store: SessionStore,
    seeded: SeededAudit,
}

fn chain_perf(c: &mut Criterion) {
    let runtime = must(Runtime::new(), "tokio runtime");
    bench_verify_session(c, &runtime);
    bench_verify_all(c, &runtime);
    bench_audit_feed_window(c, &runtime);
}

fn bench_verify_session(c: &mut Criterion, runtime: &Runtime) {
    let mut group = c.benchmark_group("verify_session");
    group.sample_size(10);
    for rows in row_profiles(&[1_000, 10_000, 100_000, 1_000_000]) {
        let bench = build_store(runtime, 1, rows);
        group.throughput(Throughput::Elements(rows as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(label(rows)),
            &bench,
            |b, bench| {
                b.iter(|| {
                    let report = must(
                        runtime.block_on(verify_session(
                            black_box(&bench.store),
                            black_box(primary_session(&bench.seeded)),
                        )),
                        "verify_session",
                    );
                    assert_clean(report.ok, "verify_session");
                    black_box(report.rows);
                });
            },
        );
    }
    group.finish();
}

fn bench_verify_all(c: &mut Criterion, runtime: &Runtime) {
    let mut group = c.benchmark_group("verify_all");
    group.sample_size(10);
    for sessions in row_profiles(&[10, 100, 1_000]) {
        let rows = sessions * 1_000;
        let bench = build_store(runtime, sessions, 1_000);
        group.throughput(Throughput::Elements(rows as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{sessions}x1k")),
            &bench,
            |b, bench| {
                b.iter(|| {
                    let report = must(
                        runtime.block_on(verify_all(black_box(&bench.store))),
                        "verify_all",
                    );
                    assert_clean(report.ok, "verify_all");
                    black_box(report.rows);
                });
            },
        );
    }
    group.finish();
}

fn bench_audit_feed_window(c: &mut Criterion, runtime: &Runtime) {
    let mut group = c.benchmark_group("audit_feed_since_24h_raw_fetch");
    group.sample_size(10);
    for rows in row_profiles(&[1_000, 10_000, 100_000]) {
        let bench = build_store(runtime, 1, rows);
        group.throughput(Throughput::Elements(rows as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(label(rows)),
            &bench,
            |b, bench| {
                b.iter(|| {
                    let fetched = must(
                        runtime.block_on(feed_window_fetch(
                            black_box(&bench.store),
                            black_box(bench.seeded.since_micros),
                        )),
                        "audit feed raw fetch",
                    );
                    black_box(fetched);
                });
            },
        );
    }
    group.finish();
}

fn build_store(runtime: &Runtime, sessions: usize, rows_per_session: usize) -> BenchStore {
    let dir = must(TempDir::new(), "tempdir");
    let db_path = dir.path().join("argo.db");
    let store = must(runtime.block_on(SessionStore::open(&db_path)), "open store");
    must(runtime.block_on(store.migrate()), "migrate store");
    let seeded = must(
        runtime.block_on(seed_chain::seed_chains(&store, sessions, rows_per_session)),
        "seed audit rows",
    );
    BenchStore {
        _dir: dir,
        store,
        seeded,
    }
}

async fn feed_window_fetch(store: &SessionStore, since_micros: i64) -> ArgoResult<usize> {
    let rows = sqlx::query(
        "SELECT a.seq, a.id, a.session_id, s.vertical_id, a.kind, a.payload_json, \
                a.parent_hash, a.self_hash, a.created_at \
         FROM (\
             SELECT seq, id, session_id, kind, payload_json, parent_hash, self_hash, created_at \
             FROM audit_record a WHERE a.created_at >= ? ORDER BY seq DESC LIMIT ?\
         ) a \
         LEFT JOIN session s ON s.id = a.session_id \
         ORDER BY a.seq ASC",
    )
    .bind(since_micros)
    .bind(RAW_FETCH_CAP)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("audit feed bench fetch: {e}")))?;
    Ok(rows.len())
}

fn row_profiles(full: &[usize]) -> Vec<usize> {
    if std::env::var_os("ARGO_AUDIT_CHAIN_PERF_SMOKE").is_some() {
        vec![100]
    } else {
        full.to_vec()
    }
}

fn primary_session(seeded: &SeededAudit) -> SessionId {
    match seeded.sessions.first().copied() {
        Some(session) => session,
        None => panic!("bench seeded no sessions"),
    }
}

fn assert_clean(ok: bool, label: &str) {
    if !ok {
        panic!("{label} reported chain breaks");
    }
}

fn label(rows: usize) -> String {
    match rows {
        1_000 => "1k".into(),
        10_000 => "10k".into(),
        100_000 => "100k".into(),
        1_000_000 => "1m".into(),
        other => other.to_string(),
    }
}

fn must<T, E: Display>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("{context}: {error}"),
    }
}

criterion_group!(benches, chain_perf);
criterion_main!(benches);
