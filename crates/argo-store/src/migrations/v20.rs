//! Schema migration v20 — `worker.fs_root` filesystem-sandbox column.
//!
//! Reconciles raw SQL migration `19__worker_fs_root.sql`. This is
//! the second half of the 19-collision split (see `v19.rs` doc
//! comment). The raw migration adds a single nullable `fs_root`
//! column to `worker` so the filesystem executor can refuse ops
//! whose resolved path escapes the sandbox.
//!
//! Idempotent: probe `PRAGMA table_info(worker)` before the
//! `ALTER TABLE` so databases that already ran the lazy raw SQL
//! re-enter cleanly.
//!
//! Rollback drops the column. SQLite ≥ 3.35 supports
//! `ALTER TABLE ... DROP COLUMN`, which is fine for the tests
//! that stress `roll_back_to(v16)`. The probe in `up` ensures the
//! re-up path is also clean.
use argo_types::ArgoError;
use sqlx::{Row, Sqlite};

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    if !column_exists(tx, "worker", "fs_root").await? {
        exec(tx, "v20 up", "ALTER TABLE worker ADD COLUMN fs_root TEXT").await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    if column_exists(tx, "worker", "fs_root").await? {
        exec(tx, "v20 down", "ALTER TABLE worker DROP COLUMN fs_root").await?;
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
        .map_err(|e| ArgoError::Provider(format!("v20 column probe {table}.{column}: {e}")))?;
    Ok(rows.iter().any(|row| {
        let name: String = row.get("name");
        name == column
    }))
}
