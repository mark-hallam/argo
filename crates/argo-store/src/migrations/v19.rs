//! Schema migration v19 — anomaly-detector tables.
//!
//! Reconciles raw SQL migration `19__anomaly.sql` (per-(worker,
//! action_kind) behavioural baselines + anomaly events).
//!
//! 19-collision split: this slot pairs with v20 below. The raw
//! `migrations/` directory ships two files prefixed `19__`:
//! `19__anomaly.sql` and `19__worker_fs_root.sql`. The versioned
//! runner needs distinct integer slots, so we promote `anomaly`
//! into v19 and `worker_fs_root` into v20 per the brief's
//! recommendation. Note: `git log --diff-filter=A` shows
//! `19__worker_fs_root.sql` (PR #859) was merged ~4 hours BEFORE
//! `19__anomaly.sql` (PR #847); the brief's claim that "anomaly
//! was authored first" is therefore inaccurate. We still prefer
//! the brief's `v19=anomaly` mapping because it is the published
//! contract that the v21-27 and v28-37 sub-briefs (and their
//! legacy-marker bookkeeping) build on.
//!
//! Rollback drops both tables and their indexes. No other
//! reconciled migration in v16..v21 references them.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V19_UP {
        exec(tx, "v19 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V19_DOWN {
        exec(tx, "v19 down", sql).await?;
    }
    Ok(())
}

async fn exec<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
    ctx: &str,
    sql: &str,
) -> Result<(), ArgoError> {
    sqlx::query(sql)
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("{ctx}: {e}")))?;
    Ok(())
}

const V19_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS worker_behavior_baseline (
        worker_id TEXT NOT NULL,
        action_kind TEXT NOT NULL,
        latency_mean REAL NOT NULL DEFAULT 0,
        latency_m2 REAL NOT NULL DEFAULT 0,
        retry_mean REAL NOT NULL DEFAULT 0,
        retry_m2 REAL NOT NULL DEFAULT 0,
        sample_count INTEGER NOT NULL DEFAULT 0,
        updated_at INTEGER NOT NULL,
        PRIMARY KEY (worker_id, action_kind)
    )",
    "CREATE TABLE IF NOT EXISTS worker_anomaly_event (
        id BLOB PRIMARY KEY,
        worker_id TEXT NOT NULL,
        action_id TEXT NOT NULL,
        action_kind TEXT NOT NULL,
        dimension TEXT NOT NULL,
        observed REAL NOT NULL,
        baseline_mean REAL NOT NULL,
        baseline_stddev REAL NOT NULL,
        z_score REAL NOT NULL,
        threshold REAL NOT NULL,
        sample_count INTEGER NOT NULL,
        observed_at INTEGER NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_anomaly_worker_at
     ON worker_anomaly_event(worker_id, observed_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_anomaly_action
     ON worker_anomaly_event(action_id)",
];

const V19_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_anomaly_action",
    "DROP INDEX IF EXISTS idx_anomaly_worker_at",
    "DROP TABLE IF EXISTS worker_anomaly_event",
    "DROP TABLE IF EXISTS worker_behavior_baseline",
];
