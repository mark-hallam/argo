//! `argo compliance export` GDPR right-to-access flow.
//!
//! Streams every row across every tenant-scoped table for one
//! tenant id into a single JSON envelope. Read-only and idempotent;
//! the export itself is logged via an `compliance.export` audit row
//! so operators can show "we accessed and shipped X rows on Y date".
//!
//! Tables are scoped two ways:
//! - direct `tenant_id` column (BLOB or TEXT per table)
//! - `session_id` FK into `session.tenant_id`
//!
//! Both are walked. Tables that don't exist (yet) are silently
//! skipped — the migration set evolves and this surface should not
//! force-link to a particular schema version.
use crate::cli::ComplianceExportArgs;
use crate::handlers::compliance_tenant_tables::{
    BLOB_TENANT_TABLES, SESSION_FK_TABLES_FOR_EXPORT, TEXT_TENANT_TABLES,
};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_audit::compute_chain_hash;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use serde_json::{json, Map, Value};
use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use std::io::Write;
use uuid::Uuid;

const FORMAT_GDPR: &str = "gdpr";
const AUDIT_KIND: &str = "compliance.export";

#[derive(Clone, Copy, Debug)]
enum ColAffinity {
    Int,
    Real,
    Blob,
    Text,
}

#[derive(Debug)]
struct ColumnInfo {
    name: String,
    affinity: ColAffinity,
}

pub async fn run(args: ComplianceExportArgs) -> ArgoResult<()> {
    if args.format != FORMAT_GDPR {
        return Err(ArgoError::Provider(format!(
            "compliance export: unsupported --format '{}', expected 'gdpr'",
            args.format
        )));
    }
    let tenant = parse_tenant(&args.tenant)?;
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;

    let envelope = build_envelope(&store, tenant).await?;
    write_envelope(&envelope, args.out.as_deref())?;
    append_audit(&store, tenant, &envelope).await?;
    Ok(())
}

async fn build_envelope(store: &SessionStore, tenant: TenantId) -> ArgoResult<Value> {
    let mut tables: Map<String, Value> = Map::new();
    for table in BLOB_TENANT_TABLES {
        if let Some(entry) = export_blob_table(store, table, tenant).await? {
            tables.insert((*table).to_string(), entry);
        }
    }
    for table in TEXT_TENANT_TABLES {
        if let Some(entry) = export_text_table(store, table, tenant).await? {
            tables.insert((*table).to_string(), entry);
        }
    }
    for table in SESSION_FK_TABLES_FOR_EXPORT {
        if let Some(entry) = export_session_fk_table(store, table, tenant).await? {
            tables.insert((*table).to_string(), entry);
        }
    }
    Ok(json!({
        "subject": tenant.to_string(),
        "exported_at": Utc::now().to_rfc3339(),
        "argo_version": env!("CARGO_PKG_VERSION"),
        "tables": Value::Object(tables),
    }))
}

fn write_envelope(envelope: &Value, out: Option<&str>) -> ArgoResult<()> {
    let body = serde_json::to_string_pretty(envelope)
        .map_err(|e| ArgoError::Provider(format!("compliance export serialize: {e}")))?;
    match out {
        Some(path) => std::fs::write(path, &body)
            .map_err(|e| ArgoError::Provider(format!("write {path}: {e}"))),
        None => {
            let mut sink = std::io::stdout().lock();
            sink.write_all(body.as_bytes())
                .and_then(|()| sink.write_all(b"\n"))
                .and_then(|()| sink.flush())
                .map_err(|e| ArgoError::Provider(format!("compliance export stdout: {e}")))
        }
    }
}

async fn export_blob_table(
    store: &SessionStore,
    table: &str,
    tenant: TenantId,
) -> ArgoResult<Option<Value>> {
    let Some(cols) = table_columns(store, table).await? else {
        return Ok(None);
    };
    if !has_column(&cols, "tenant_id") {
        return Ok(None);
    }
    // audit-allow: identifier substitution against a static allow-list.
    let sql = format!("SELECT * FROM {table} WHERE tenant_id = ? ORDER BY rowid");
    let rows = sqlx::query(&sql)
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("compliance export {table}: {e}")))?;
    Ok(Some(build_table_entry(&cols, &rows)))
}

async fn export_text_table(
    store: &SessionStore,
    table: &str,
    tenant: TenantId,
) -> ArgoResult<Option<Value>> {
    let Some(cols) = table_columns(store, table).await? else {
        return Ok(None);
    };
    if !has_column(&cols, "tenant_id") {
        return Ok(None);
    }
    // audit-allow: identifier substitution against a static allow-list.
    let sql = format!("SELECT * FROM {table} WHERE tenant_id = ? ORDER BY rowid");
    let rows = sqlx::query(&sql)
        .bind(tenant.to_string())
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("compliance export {table}: {e}")))?;
    Ok(Some(build_table_entry(&cols, &rows)))
}

async fn export_session_fk_table(
    store: &SessionStore,
    table: &str,
    tenant: TenantId,
) -> ArgoResult<Option<Value>> {
    let Some(cols) = table_columns(store, table).await? else {
        return Ok(None);
    };
    if !has_column(&cols, "session_id") {
        return Ok(None);
    }
    // audit-allow: identifier substitution against a static allow-list.
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
    Ok(Some(build_table_entry(&cols, &rows)))
}

fn build_table_entry(cols: &[ColumnInfo], rows: &[SqliteRow]) -> Value {
    let schema: Vec<Value> = cols
        .iter()
        .map(|c| Value::String(c.name.clone()))
        .collect();
    let rendered: Vec<Value> = rows.iter().map(|row| row_to_json(row, cols)).collect();
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

async fn table_columns(
    store: &SessionStore,
    table: &str,
) -> ArgoResult<Option<Vec<ColumnInfo>>> {
    if !table_exists(store, table).await? {
        return Ok(None);
    }
    // audit-allow: identifier substitution against a static allow-list.
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

fn parse_tenant(raw: &str) -> ArgoResult<TenantId> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ArgoError::Provider(
            "compliance export: --tenant <id> is required".into(),
        ));
    }
    Uuid::parse_str(trimmed)
        .map(TenantId)
        .map_err(|e| ArgoError::Provider(format!("compliance export: tenant '{raw}': {e}")))
}

async fn append_audit(
    store: &SessionStore,
    tenant: TenantId,
    envelope: &Value,
) -> ArgoResult<()> {
    let parent = last_global_hash(store).await?;
    let at = Utc::now();
    let row_total = envelope
        .get("tables")
        .and_then(Value::as_object)
        .map(|t| {
            t.values()
                .filter_map(|v| v.get("rows").and_then(Value::as_array).map(Vec::len))
                .sum::<usize>()
        })
        .unwrap_or(0);
    let payload = json!({
        "actor": "cli",
        "action": AUDIT_KIND,
        "target": tenant.to_string(),
        "row_count": row_total,
        "at": at.to_rfc3339(),
    });
    let canonical = serde_json::to_string(&payload)
        .map_err(|e| ArgoError::Provider(format!("compliance export audit json: {e}")))?;
    let self_hash = compute_chain_hash(&parent, AUDIT_KIND, &canonical);
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, parent_hash, self_hash, created_at) \
         VALUES (?, NULL, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::now_v7().as_bytes().as_slice())
    .bind(AUDIT_KIND)
    .bind(canonical)
    .bind(parent.as_slice())
    .bind(self_hash.as_slice())
    .bind(at.timestamp_micros())
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("compliance export audit: {e}")))?;
    Ok(())
}

async fn last_global_hash(store: &SessionStore) -> ArgoResult<[u8; 32]> {
    let bytes: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT self_hash FROM audit_record WHERE session_id IS NULL ORDER BY seq DESC LIMIT 1",
    )
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("compliance export last hash: {e}")))?;
    let mut out = [0_u8; 32];
    if let Some(b) = bytes {
        if b.len() == 32 {
            out.copy_from_slice(&b);
        }
    }
    Ok(out)
}

#[cfg(test)]
#[path = "compliance_export_tests.rs"]
mod tests;
