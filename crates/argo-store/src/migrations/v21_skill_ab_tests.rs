//! Schema migration v21 — skill A/B-test framework tables.
//!
//! Reconciles raw SQL migration `20__skill_ab_tests.sql`. The raw
//! file shipped with prefix `20`, but v20 was claimed above by the
//! second half of the 19-collision split (`worker.fs_root`), so
//! this module's slot index slides to v21. The
//! `v21-27` and `v28-37` sub-briefs continue from v22.
//!
//! Tables: `skill_ab_test_run` carries the run definition
//! (variant_a + variant_b skill ids + target invocations + status);
//! `skill_ab_test_outcome` carries one row per recorded invocation.
//! Significance is computed lazily off the outcome rows — no
//! denormalised running totals on the run row.
//!
//! Rollback drops both tables and the run-status index.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V21_UP {
        exec(tx, "v21 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V21_DOWN {
        exec(tx, "v21 down", sql).await?;
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

const V21_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS skill_ab_test_run (
        id              TEXT PRIMARY KEY,
        skill_name      TEXT NOT NULL,
        variant_a       TEXT NOT NULL,
        variant_b       TEXT NOT NULL,
        target_runs     INTEGER NOT NULL,
        started_at      INTEGER NOT NULL,
        completed_at    INTEGER,
        status          TEXT NOT NULL,
        winner          TEXT,
        UNIQUE(skill_name, variant_a, variant_b, started_at)
    )",
    "CREATE INDEX IF NOT EXISTS idx_skill_ab_test_run_status
     ON skill_ab_test_run(status, started_at DESC)",
    "CREATE TABLE IF NOT EXISTS skill_ab_test_outcome (
        run_id          TEXT NOT NULL REFERENCES skill_ab_test_run(id) ON DELETE CASCADE,
        invocation_id   TEXT NOT NULL,
        variant_chosen  TEXT NOT NULL,
        succeeded       INTEGER NOT NULL,
        latency_ms      INTEGER,
        recorded_at     INTEGER NOT NULL,
        PRIMARY KEY (run_id, invocation_id)
    )",
    "CREATE INDEX IF NOT EXISTS idx_skill_ab_test_outcome_run
     ON skill_ab_test_outcome(run_id, recorded_at)",
];

const V21_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_skill_ab_test_outcome_run",
    "DROP TABLE IF EXISTS skill_ab_test_outcome",
    "DROP INDEX IF EXISTS idx_skill_ab_test_run_status",
    "DROP TABLE IF EXISTS skill_ab_test_run",
];
