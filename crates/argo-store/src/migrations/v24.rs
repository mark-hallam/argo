//! Schema migration v24 — `workspace.archived_at` / `owner_email`
//! soft-delete + tenant-archived insert/update guard triggers.
//!
//! Reconciles raw SQL migration `22__tenant_archived.sql` (Stream
//! `codex-tenant-cli-extensions`). Adds two nullable columns to
//! `workspace` and two `BEFORE INSERT/UPDATE OF tenant_id`
//! triggers on `worker` that abort writes when the targeted
//! tenant's workspace row carries a non-NULL `archived_at`.
//!
//! 22-collision split: this slot pairs with v23 above (see v23
//! doc comment for git-log ordering and rationale).
//!
//! Idempotent: each column probed via `PRAGMA table_info(workspace)`
//! before `ALTER TABLE ... ADD COLUMN`; the triggers use
//! `CREATE TRIGGER IF NOT EXISTS`. Databases that already ran the
//! lazy raw SQL (via `tenant_archive::ensure_tenant_lifecycle_schema`
//! in argo-cli) re-enter cleanly.
//!
//! Rollback drops the two triggers, then both columns. SQLite
//! ≥ 3.35 supports `ALTER TABLE ... DROP COLUMN`. Both columns
//! are nullable soft-delete markers — no FK or NOT NULL
//! references — so dropping them is safe.
use argo_types::ArgoError;
use sqlx::{Row, Sqlite};

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for (column, ddl) in WORKSPACE_COLUMNS {
        if !column_exists(tx, "workspace", column).await? {
            exec(tx, "v24 up", ddl).await?;
        }
    }
    for sql in TRIGGERS_UP {
        exec(tx, "v24 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in TRIGGERS_DOWN {
        exec(tx, "v24 down", sql).await?;
    }
    for (column, _) in WORKSPACE_COLUMNS {
        if column_exists(tx, "workspace", column).await? {
            exec(
                tx,
                "v24 down",
                &format!("ALTER TABLE workspace DROP COLUMN {column}"),
            )
            .await?;
        }
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
        .map_err(|e| ArgoError::Provider(format!("v24 column probe {table}.{column}: {e}")))?;
    Ok(rows.iter().any(|row| {
        let name: String = row.get("name");
        name == column
    }))
}

const WORKSPACE_COLUMNS: &[(&str, &str)] = &[
    (
        "archived_at",
        "ALTER TABLE workspace ADD COLUMN archived_at INTEGER",
    ),
    (
        "owner_email",
        "ALTER TABLE workspace ADD COLUMN owner_email TEXT",
    ),
];

const TRIGGERS_UP: &[&str] = &[
    "CREATE TRIGGER IF NOT EXISTS worker_reject_archived_tenant_insert
     BEFORE INSERT ON worker
     FOR EACH ROW
     WHEN EXISTS (
         SELECT 1 FROM workspace
         WHERE archived_at IS NOT NULL
           AND lower(hex(tenant_id)) = replace(lower(NEW.tenant_id), '-', '')
     )
     BEGIN
         SELECT RAISE(ABORT, 'tenant is archived');
     END",
    "CREATE TRIGGER IF NOT EXISTS worker_reject_archived_tenant_update
     BEFORE UPDATE OF tenant_id ON worker
     FOR EACH ROW
     WHEN EXISTS (
         SELECT 1 FROM workspace
         WHERE archived_at IS NOT NULL
           AND lower(hex(tenant_id)) = replace(lower(NEW.tenant_id), '-', '')
     )
     BEGIN
         SELECT RAISE(ABORT, 'tenant is archived');
     END",
];

const TRIGGERS_DOWN: &[&str] = &[
    "DROP TRIGGER IF EXISTS worker_reject_archived_tenant_update",
    "DROP TRIGGER IF EXISTS worker_reject_archived_tenant_insert",
];
