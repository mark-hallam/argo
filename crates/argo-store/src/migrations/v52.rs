//! Schema migration v52 — per-subtask budget columns on
//! `subtask_ledger` for Stream `mission-delegation-budget-tree`
//! (MISSION_BACKLOG Section 3 Claim 2 #57).
//!
//! Three additive columns drive the per-delegation budget tree:
//!
//!   * `budget_usd_cents`  — declared cap; NULL means unbounded.
//!   * `spent_usd_cents`   — accumulator (NOT NULL DEFAULT 0).
//!   * `parent_subtask_id` — explicit parent pointer so the CTE
//!     roll-up in `argo subtask budget` can walk descendants.
//!
//! Slot rationale: v51 (`mission-cross-tenant-trust-graph` PR #1302)
//! occupied the prior versioned slot on origin/main; v52 is the next
//! free versioned slot. Raw SQL counterpart:
//! `56__subtask_budget.sql` (filename slots 50..55 are taken by
//! `worker_trust_grade`, `worker_skill` + `audit_worker_deleted_at`,
//! `persona_revision`, `forge_run_cost`, `skill_min_trust_grade`, and `trust_observation` respectively).
//!
//! UP idempotently adds the three columns via `PRAGMA table_info`
//! probes and creates a partial index on `parent_subtask_id` (where
//! not null) that backs the recursive `walk_tree` CTE.
//!
//! DOWN drops the index and the columns in reverse order. SQLite
//! ≥ 3.35 supports `ALTER TABLE … DROP COLUMN`; none of the three
//! columns has FK references so dropping is safe.
use argo_types::ArgoError;
use sqlx::{Row, Sqlite};

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for (col, ddl) in BUDGET_COLUMNS {
        if !column_exists(tx, "subtask_ledger", col).await? {
            exec(tx, "v52 up", ddl).await?;
        }
    }
    exec(
        tx,
        "v52 up",
        "CREATE INDEX IF NOT EXISTS idx_subtask_ledger_parent_subtask \
         ON subtask_ledger(parent_subtask_id) \
         WHERE parent_subtask_id IS NOT NULL",
    )
    .await?;
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    exec(
        tx,
        "v52 down",
        "DROP INDEX IF EXISTS idx_subtask_ledger_parent_subtask",
    )
    .await?;
    for (col, _) in BUDGET_COLUMNS.iter().rev() {
        if column_exists(tx, "subtask_ledger", col).await? {
            let sql = format!("ALTER TABLE subtask_ledger DROP COLUMN {col}");
            exec(tx, "v52 down", &sql).await?;
        }
    }
    Ok(())
}

const BUDGET_COLUMNS: &[(&str, &str)] = &[
    (
        "budget_usd_cents",
        "ALTER TABLE subtask_ledger ADD COLUMN budget_usd_cents INTEGER NULL",
    ),
    (
        "spent_usd_cents",
        "ALTER TABLE subtask_ledger ADD COLUMN spent_usd_cents INTEGER NOT NULL DEFAULT 0",
    ),
    (
        "parent_subtask_id",
        "ALTER TABLE subtask_ledger ADD COLUMN parent_subtask_id TEXT NULL",
    ),
];

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

async fn column_exists<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
    table: &str,
    column: &str,
) -> Result<bool, ArgoError> {
    let rows = sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v52 column probe {table}.{column}: {e}")))?;
    Ok(rows.iter().any(|row| {
        let name: String = row.get("name");
        name == column
    }))
}
