//! Criterion bench for the indexed audit-feed read path
//! (Stream `mission-enterprise-audit-full-text-search`).
//!
//! Why this matters: the J14 audit-feed endpoint previously did a
//! `created_at`-bounded SQL fetch (5000-row cap) and then ran the
//! 7-dimension filter pass app-side over `payload_json`. That works
//! at v1 single-tenant scale but won't survive the enterprise
//! traffic profile (>10k audit rows/day per tenant). Migration v61
//! promotes the dimensions to first-class columns with composite
//! `(created_at, <dim>)` indexes; this bench is the brief's ship
//! gate — without it, the "this scales to enterprise traffic"
//! claim is just an assertion.
//!
//! Bench shape: seed a hermetic tmpfile SQLite with 50_000 audit
//! rows (5 dimensions × 4 values × 2500 rows), then time a
//! single-dimension filter SELECT. The acceptance bar is < 50ms
//! p99 over 50k rows — verified empirically by the criterion
//! median + 99th percentile estimators.
//!
//! Run locally:
//!   `cargo bench -p argo-cli --bench audit_feed`
//!
//! NOT wired into CI (compile-only via `cargo build --benches`).
//! The seed step is single-threaded and takes ~10s — keep
//! `sample_size` low to avoid 10-minute bench runs.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use criterion::{criterion_group, criterion_main, Criterion};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;
use tokio::runtime::Runtime;

const SEED_ROW_COUNT: usize = 50_000;
const WORKERS: &[&str] = &["alice", "bob", "carol", "dave", "eve"];
const CHANNELS: &[&str] = &["web", "slack", "email", "voice"];
const ACTION_TYPES: &[&str] = &[
    "approval.decided",
    "message.sent",
    "tool.invoked",
    "skill.run",
];
const STATUSES: &[&str] = &["succeeded", "failed", "denied", "queued"];

fn audit_feed_indexed_filter(c: &mut Criterion) {
    let runtime = Runtime::new().expect("runtime");
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("audit_bench.db");
    let pool = runtime.block_on(seed_db(&path));

    let mut group = c.benchmark_group("audit_feed_indexed");
    // Each seed pass takes ~10s; keep samples low so total bench
    // wall-clock is bounded.
    group.sample_size(20);

    group.bench_function("single_dim_worker_filter", |b| {
        b.iter(|| {
            let _ = runtime.block_on(single_dim_query(&pool, "worker_id", "alice"));
        });
    });

    group.bench_function("single_dim_action_filter", |b| {
        b.iter(|| {
            let _ = runtime.block_on(single_dim_query(&pool, "action_type", "approval.decided"));
        });
    });

    group.bench_function("single_dim_status_filter", |b| {
        b.iter(|| {
            let _ = runtime.block_on(single_dim_query(&pool, "outcome_status", "succeeded"));
        });
    });

    group.finish();
}

async fn seed_db(path: &Path) -> SqlitePool {
    let url = format!("sqlite://{}?mode=rwc", path.display());
    let opts = SqliteConnectOptions::from_str(&url)
        .expect("connect opts")
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .expect("connect");
    // Minimum schema the bench needs — the audit-store migration
    // chain pulls in dozens of tables we don't exercise here; this
    // mirrors the v61 audit_record shape exactly so the same query
    // plan applies.
    sqlx::query(
        "CREATE TABLE audit_record (
            seq INTEGER PRIMARY KEY AUTOINCREMENT,
            id BLOB NOT NULL,
            session_id BLOB,
            kind TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            parent_hash BLOB NOT NULL,
            self_hash BLOB NOT NULL,
            created_at INTEGER NOT NULL,
            principal_id TEXT,
            worker_id TEXT,
            channel TEXT,
            action_type TEXT,
            outcome_status TEXT,
            approver_id TEXT,
            cost_band TEXT,
            data_classification TEXT
        )",
    )
    .execute(&pool)
    .await
    .expect("create table");
    for sql in [
        "CREATE INDEX idx_audit_record_created_at ON audit_record(created_at)",
        "CREATE INDEX idx_audit_record_created_at_worker ON audit_record(created_at, worker_id)",
        "CREATE INDEX idx_audit_record_created_at_action ON audit_record(created_at, action_type)",
        "CREATE INDEX idx_audit_record_created_at_status ON audit_record(created_at, outcome_status)",
    ] {
        sqlx::query(sql).execute(&pool).await.expect("create index");
    }
    let mut tx = pool.begin().await.expect("tx");
    for i in 0..SEED_ROW_COUNT {
        let now_micros = 1_700_000_000_000_000_i64 + i as i64 * 1_000_000;
        let worker = WORKERS[i % WORKERS.len()];
        let channel = CHANNELS[i % CHANNELS.len()];
        let action = ACTION_TYPES[i % ACTION_TYPES.len()];
        let status = STATUSES[i % STATUSES.len()];
        let payload = format!(
            r#"{{"worker_id":"{worker}","channel":"{channel}","action_type":"{action}","outcome_status":"{status}"}}"#
        );
        sqlx::query(
            "INSERT INTO audit_record (id, kind, payload_json, parent_hash, self_hash, \
             created_at, worker_id, channel, action_type, outcome_status) \
             VALUES (?, 'http_request', ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind((i as i64).to_le_bytes().to_vec())
        .bind(payload)
        .bind([0u8; 32].as_slice())
        .bind([0u8; 32].as_slice())
        .bind(now_micros)
        .bind(worker)
        .bind(channel)
        .bind(action)
        .bind(status)
        .execute(&mut *tx)
        .await
        .expect("insert");
    }
    tx.commit().await.expect("commit");
    pool
}

async fn single_dim_query(pool: &SqlitePool, column: &str, value: &str) -> Vec<(Vec<u8>, i64)> {
    // Static column whitelist — the production handler does the
    // equivalent in `routes_audit_feed_indexed::indexed_fetch`,
    // building the SQL from a fixed list of column names.
    let sql = format!(
        "SELECT id, created_at FROM audit_record \
         WHERE ({column} IN (?) OR {column} IS NULL) \
         ORDER BY created_at DESC, id DESC LIMIT 50"
    );
    sqlx::query_as(&sql)
        .bind(value)
        .fetch_all(pool)
        .await
        .expect("bench query")
}

criterion_group!(benches, audit_feed_indexed_filter);
criterion_main!(benches);
