//! Schema migration v45 — additive `worker.archived_at` soft-delete
//! column for Stream `mvp-identity-archive-worker` (MISSION_BACKLOG
//! #40, archive half).
//!
//! Reconciles raw SQL `49__worker_archived_at.sql`. The two
//! counters move independently: v45 is the next free
//! `schema_version` slot after v44 (`approval_request`), while raw
//! filename slot 49 is the next free filename slot after 45..48
//! (occupied by `cu_screenshot_index`, `worker_dependency`,
//! `worker_level`, `audit_principal`, `skill_canary`, `approvals`).
//!
//! Note: the original brief cited "v47 / 47\_\_worker\_archived\_at.sql"
//! but origin/main has already advanced past those numbers. Using
//! the next free slot in each counter follows the v42/v43/v44
//! precedent.
//!
//! UP adds the nullable column and a `(tenant_id, archived_at)`
//! index for the soft-delete filter on `IdentityStore::list_workers`.
//! Idempotent: the column existence is probed via
//! `PRAGMA table_info(worker)` before `ALTER TABLE`, and the index
//! uses `CREATE INDEX IF NOT EXISTS` (mirrors v24's `archived_at`
//! treatment for `workspace`).
//!
//! DOWN drops the index and the column. SQLite ≥ 3.35 supports
//! `ALTER TABLE ... DROP COLUMN`; the column is a nullable
//! soft-delete marker with no FK/NOT NULL references so dropping
//! it is safe.
use argo_types::ArgoError;
use sqlx::{Row, Sqlite};

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    if !column_exists(tx, "worker", "archived_at").await? {
        exec(tx, "v45 up", "ALTER TABLE worker ADD COLUMN archived_at TEXT").await?;
    }
    exec(
        tx,
        "v45 up",
        "CREATE INDEX IF NOT EXISTS idx_worker_archived_at \
         ON worker(tenant_id, archived_at)",
    )
    .await?;
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    exec(tx, "v45 down", "DROP INDEX IF EXISTS idx_worker_archived_at").await?;
    if column_exists(tx, "worker", "archived_at").await? {
        exec(tx, "v45 down", "ALTER TABLE worker DROP COLUMN archived_at").await?;
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

async fn column_exists<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
    table: &str,
    column: &str,
) -> Result<bool, ArgoError> {
    let rows = sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v45 column probe {table}.{column}: {e}")))?;
    Ok(rows.iter().any(|row| {
        let name: String = row.get("name");
        name == column
    }))
}
