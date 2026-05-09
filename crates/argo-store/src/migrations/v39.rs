//! Schema migration v39 — persistent subtask ledger.
//!
//! Reconciles raw SQL migration `29__subtask_ledger.sql` (Stream
//! `claude-subtask-ledger-store`). Single table `subtask_ledger`
//! recording every subtask the Planner emits when it decomposes a
//! coarse goal into a workflow DAG, with `pending`/`running`/
//! `done`/`failed`/`skipped` lifecycle for restart-safe resume.
//!
//! No collision: `29__` is a single-prefix entry between the 28
//! and 30 collision groups. Idempotent: CREATE TABLE/INDEX IF NOT
//! EXISTS. Rollback drops the table and all five indexes.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V39_UP {
        exec(tx, "v39 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V39_DOWN {
        exec(tx, "v39 down", sql).await?;
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

const V39_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS subtask_ledger (
        id TEXT PRIMARY KEY,
        parent_goal TEXT NOT NULL,
        parent_session_id TEXT,
        worker_id TEXT NOT NULL,
        plan_node_path TEXT NOT NULL,
        description TEXT NOT NULL,
        state TEXT NOT NULL DEFAULT 'pending',
        output TEXT,
        started_at INTEGER,
        completed_at INTEGER,
        created_at INTEGER NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_subtask_ledger_session
     ON subtask_ledger(parent_session_id, created_at)",
    "CREATE INDEX IF NOT EXISTS idx_subtask_ledger_worker
     ON subtask_ledger(worker_id, created_at)",
    "CREATE INDEX IF NOT EXISTS idx_subtask_ledger_state
     ON subtask_ledger(state, created_at)",
    "CREATE INDEX IF NOT EXISTS idx_subtask_ledger_parent_goal
     ON subtask_ledger(parent_goal)",
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_subtask_ledger_node_unique
     ON subtask_ledger(parent_session_id, worker_id, plan_node_path)",
];

const V39_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_subtask_ledger_node_unique",
    "DROP INDEX IF EXISTS idx_subtask_ledger_parent_goal",
    "DROP INDEX IF EXISTS idx_subtask_ledger_state",
    "DROP INDEX IF EXISTS idx_subtask_ledger_worker",
    "DROP INDEX IF EXISTS idx_subtask_ledger_session",
    "DROP TABLE IF EXISTS subtask_ledger",
];
