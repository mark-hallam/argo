use super::{ColAffinity, ColumnInfo, ExportFilters};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde_json::{json, Map, Value};
use sqlx::sqlite::SqliteRow;
use sqlx::Row;

pub(super) async fn export_blob_table(
    store: &SessionStore,
    table: &str,
    tenant: TenantId,
    filters: &ExportFilters,
) -> ArgoResult<Option<Value>> {
    let Some(cols) = table_columns(store, table).await? else {
        return Ok(None);
    };
    if !has_column(&cols, "tenant_id") {
        return Ok(None);
    }
    let sql = format!("SELECT * FROM {table} WHERE tenant_id = ? ORDER BY rowid");
    let rows = sqlx::query(&sql)
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("compliance export {table}: {e}")))?;
    Ok(Some(build_table_entry(&cols, &rows, filters)))
}

pub(super) async fn export_text_table(
    store: &SessionStore,
    table: &str,
    tenant: TenantId,
    filters: &ExportFilters,
) -> ArgoResult<Option<Value>> {
    let Some(cols) = table_columns(store, table).await? else {
        return Ok(None);
    };
    if !has_column(&cols, "tenant_id") {
        return Ok(None);
    }
    let sql = format!("SELECT * FROM {table} WHERE tenant_id = ? ORDER BY rowid");
    let rows = sqlx::query(&sql)
        .bind(tenant.to_string())
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("compliance export {table}: {e}")))?;
    Ok(Some(build_table_entry(&cols, &rows, filters)))
}

pub(super) async fn export_session_fk_table(
    store: &SessionStore,
    table: &str,
    tenant: TenantId,
    filters: &ExportFilters,
) -> ArgoResult<Option<Value>> {
    let Some(cols) = table_columns(store, table).await? else {
        return Ok(None);
    };
    if !has_column(&cols, "session_id") {
        return Ok(None);
    }
    let sql = format!(
        "SELECT t.* FROM {table} t \
         JOIN session s ON s.id = t.session_id \
         WHERE s.tenant_id = ? \
         ORDER BY t.rowid"
    );
    let rows = sqlx::query(&sql)
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("compliance export {table}: {e}")))?;
    Ok(Some(build_table_entry(&cols, &rows, filters)))
}

fn build_table_entry(cols: &[ColumnInfo], rows: &[SqliteRow], filters: &ExportFilters) -> Value {
    let schema: Vec<Value> = cols.iter().map(|c| Value::String(c.name.clone())).collect();
    let rendered: Vec<Value> = rows
        .iter()
        .map(|row| row_to_json(row, cols))
        .filter(|row| filters.matches(row))
        .collect();
    json!({
        "schema": Value::Array(schema),
        "rows": Value::Array(rendered),
    })
}

fn row_to_json(row: &SqliteRow, cols: &[ColumnInfo]) -> Value {
    let mut obj = Map::with_capacity(cols.len());
    for (idx, col) in cols.iter().enumerate() {
        obj.insert(col.name.clone(), column_to_json(row, idx, col.affinity));
    }
    Value::Object(obj)
}

fn column_to_json(row: &SqliteRow, idx: usize, affinity: ColAffinity) -> Value {
    match affinity {
        ColAffinity::Int => row
            .try_get::<Option<i64>, _>(idx)
            .ok()
            .flatten()
            .map_or(Value::Null, |n| Value::Number(n.into())),
        ColAffinity::Real => row
            .try_get::<Option<f64>, _>(idx)
            .ok()
            .flatten()
            .and_then(serde_json::Number::from_f64)
            .map_or(Value::Null, Value::Number),
        ColAffinity::Blob => row
            .try_get::<Option<Vec<u8>>, _>(idx)
            .ok()
            .flatten()
            .map_or(Value::Null, |b| Value::String(hex::encode(&b))),
        ColAffinity::Text => row
            .try_get::<Option<String>, _>(idx)
            .ok()
            .flatten()
            .map_or(Value::Null, Value::String),
    }
}

async fn table_columns(store: &SessionStore, table: &str) -> ArgoResult<Option<Vec<ColumnInfo>>> {
    if !table_exists(store, table).await? {
        return Ok(None);
    }
    let rows = sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("compliance export pragma {table}: {e}")))?;
    let cols = rows
        .iter()
        .map(|r| ColumnInfo {
            name: r.get::<String, _>("name"),
            affinity: classify_affinity(&r.get::<String, _>("type")),
        })
        .collect::<Vec<_>>();
    Ok(Some(cols))
}

async fn table_exists(store: &SessionStore, table: &str) -> ArgoResult<bool> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(table)
            .fetch_one(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("compliance export exists {table}: {e}")))?;
    Ok(count > 0)
}

fn has_column(cols: &[ColumnInfo], name: &str) -> bool {
    cols.iter().any(|c| c.name == name)
}

fn classify_affinity(decl_type: &str) -> ColAffinity {
    let upper = decl_type.to_ascii_uppercase();
    if upper.contains("INT") {
        ColAffinity::Int
    } else if upper.contains("BLOB") {
        ColAffinity::Blob
    } else if upper.contains("REAL") || upper.contains("FLOA") || upper.contains("DOUB") {
        ColAffinity::Real
    } else {
        ColAffinity::Text
    }
}
