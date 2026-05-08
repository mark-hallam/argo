//! Schema migration v13 — memory layering tables and indexes.
//!
//! This is the versioned equivalent of raw SQL migration
//! `13__memory_layers.sql`. It keeps the existing runtime behavior:
//! base memory columns/indexes/FTS are required, while the sqlite-vec
//! virtual table is best-effort because many dev/test connections do
//! not load the `vec0` extension.
use argo_types::ArgoError;
use sqlx::{Row, Sqlite};

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for (column, ddl) in MEMORY_COLUMNS {
        if !column_exists(tx, "memory_entry", column).await? {
            exec(tx, "v13 up", ddl).await?;
        }
    }
    for sql in V13_REQUIRED {
        exec(tx, "v13 up", sql).await?;
    }
    if let Err(err) = exec(tx, "v13 vec0", V13_VEC0).await {
        if !err.to_string().contains("no such module: vec0") {
            return Err(err);
        }
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    _tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    Err(ArgoError::Provider(
        "v13 rollback unsupported: raw migration reconciliation is additive and may own live data"
            .into(),
    ))
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
        .map_err(|e| ArgoError::Provider(format!("v13 column probe {table}.{column}: {e}")))?;
    Ok(rows.iter().any(|row| {
        let name: String = row.get("name");
        name == column
    }))
}

const MEMORY_COLUMNS: &[(&str, &str)] = &[
    (
        "layer",
        "ALTER TABLE memory_entry ADD COLUMN layer TEXT NOT NULL DEFAULT 'recent'",
    ),
    (
        "tenant_id",
        "ALTER TABLE memory_entry ADD COLUMN tenant_id BLOB",
    ),
    (
        "worker_id",
        "ALTER TABLE memory_entry ADD COLUMN worker_id BLOB",
    ),
    (
        "memory_scope",
        "ALTER TABLE memory_entry ADD COLUMN memory_scope TEXT",
    ),
];

const V13_REQUIRED: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_memory_entry_layer
     ON memory_entry(workspace_id, layer, learned_at)",
    "CREATE INDEX IF NOT EXISTS idx_memory_entry_ns_layer
     ON memory_entry(tenant_id, worker_id, memory_scope, layer, learned_at)",
    "CREATE VIRTUAL TABLE IF NOT EXISTS memory_entry_fts USING fts5(
        entry_id UNINDEXED,
        body,
        tokenize = 'unicode61 remove_diacritics 2'
    )",
    "CREATE TABLE IF NOT EXISTS memory_embedding (
        entry_id BLOB PRIMARY KEY REFERENCES memory_entry(id) ON DELETE CASCADE,
        model TEXT NOT NULL,
        dims INTEGER NOT NULL,
        vector_json TEXT NOT NULL,
        created_at INTEGER NOT NULL
    )",
];

const V13_VEC0: &str = "CREATE VIRTUAL TABLE IF NOT EXISTS memory_vec_index USING vec0(
    entry_id TEXT PRIMARY KEY,
    embedding FLOAT[64]
)";
