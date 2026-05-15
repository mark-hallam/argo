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
use crate::tenant_tables::{BLOB_TENANT_TABLES, SESSION_FK_TABLES_FOR_EXPORT, TEXT_TENANT_TABLES};
use argo_audit::compute_chain_hash;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Utc};
use serde_json::{json, Map, Value};
use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use std::io::Write;
use uuid::Uuid;

const AUDIT_KIND: &str = "compliance.export";

#[derive(Debug, Clone)]
pub struct ComplianceExportArgs {
    pub tenant: String,
    pub format: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub session_id: Option<String>,
    pub out: Option<String>,
}

#[derive(Debug, Clone)]
struct ExportFilters {
    start: Option<i64>,
    end: Option<i64>,
    session: Option<String>,
}

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

pub async fn run(args: ComplianceExportArgs, store: &SessionStore) -> ArgoResult<()> {
    let format = normalize_format(&args.format)?;
    let filters = ExportFilters::parse(&args)?;
    let tenant = parse_tenant(&args.tenant)?;

    let mut envelope = build_envelope(store, tenant, &filters).await?;
    stamp_integrity(&mut envelope)?;
    write_envelope(&envelope, args.out.as_deref(), format)?;
    append_audit(store, tenant, &envelope).await?;
    Ok(())
}

fn normalize_format(raw: &str) -> ArgoResult<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "json" | "gdpr" => Ok("json"),
        "csv" => Ok("csv"),
        "pdf" => Ok("pdf"),
        other => Err(ArgoError::Provider(format!(
            "compliance export: unsupported --format '{other}', expected json|csv|pdf"
        ))),
    }
}

impl ExportFilters {
    fn parse(args: &ComplianceExportArgs) -> ArgoResult<Self> {
        Ok(Self {
            start: parse_date(args.start_date.as_deref(), "start-date")?,
            end: parse_date(args.end_date.as_deref(), "end-date")?,
            session: args.session_id.as_ref().map(|s| s.trim().to_string()),
        })
    }

    fn matches(&self, row: &Value) -> bool {
        self.matches_session(row) && self.matches_date(row)
    }

    fn matches_session(&self, row: &Value) -> bool {
        let Some(want) = self.session.as_deref() else {
            return true;
        };
        row.get("session_id")
            .and_then(Value::as_str)
            .is_some_and(|got| got == want || got.eq_ignore_ascii_case(&want.replace('-', "")))
    }

    fn matches_date(&self, row: &Value) -> bool {
        if self.start.is_none() && self.end.is_none() {
            return true;
        }
        let Some(ts) = row.get("created_at").and_then(value_to_micros) else {
            return false;
        };
        if let Some(start) = self.start {
            if ts < start {
                return false;
            }
        }
        if let Some(end) = self.end {
            if ts > end {
                return false;
            }
        }
        true
    }
}

fn parse_date(raw: Option<&str>, label: &str) -> ArgoResult<Option<i64>> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| Some(dt.timestamp_micros()))
        .map_err(|e| ArgoError::Provider(format!("compliance export: --{label} invalid: {e}")))
}

fn value_to_micros(v: &Value) -> Option<i64> {
    if let Some(n) = v.as_i64() {
        return Some(if n.abs() < 10_000_000_000 {
            n * 1_000_000
        } else {
            n
        });
    }
    v.as_str()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.timestamp_micros())
}

async fn build_envelope(
    store: &SessionStore,
    tenant: TenantId,
    filters: &ExportFilters,
) -> ArgoResult<Value> {
    let mut tables: Map<String, Value> = Map::new();
    for table in BLOB_TENANT_TABLES {
        if let Some(entry) = export_blob_table(store, table, tenant, filters).await? {
            tables.insert((*table).to_string(), entry);
        }
    }
    for table in TEXT_TENANT_TABLES {
        if let Some(entry) = export_text_table(store, table, tenant, filters).await? {
            tables.insert((*table).to_string(), entry);
        }
    }
    for table in SESSION_FK_TABLES_FOR_EXPORT {
        if let Some(entry) = export_session_fk_table(store, table, tenant, filters).await? {
            tables.insert((*table).to_string(), entry);
        }
    }
    Ok(json!({
        "subject": tenant.to_string(),
        "exported_at": Utc::now().to_rfc3339(),
        "argo_version": env!("CARGO_PKG_VERSION"),
        "filters": {
            "start_date": filters.start,
            "end_date": filters.end,
            "session_id": filters.session,
        },
        "tables": Value::Object(tables),
    }))
}

fn stamp_integrity(envelope: &mut Value) -> ArgoResult<()> {
    let canonical = serde_json::to_string(envelope)
        .map_err(|e| ArgoError::Provider(format!("compliance export integrity json: {e}")))?;
    let checksum = compute_chain_hash(&[0_u8; 32], "compliance.export.checksum", &canonical);
    let signature = compute_chain_hash(&checksum, "compliance.export.signature", &canonical);
    envelope["integrity"] = json!({
        "checksum": format!("sha256:{}", hex::encode(checksum)),
        "signature": format!("argo-chain:{}", hex::encode(signature)),
    });
    Ok(())
}

fn write_envelope(envelope: &Value, out: Option<&str>, format: &str) -> ArgoResult<()> {
    let body = match format {
        "csv" => render_csv(envelope),
        "pdf" => render_pdf(envelope),
        _ => serde_json::to_string_pretty(envelope)
            .map_err(|e| ArgoError::Provider(format!("compliance export serialize: {e}")))?,
    };
    match out {
        Some(path) => {
            std::fs::write(path, &body).map_err(|e| ArgoError::Io(format!("write {path}: {e}")))
        }
        None => {
            let mut sink = std::io::stdout().lock();
            sink.write_all(body.as_bytes())
                .and_then(|()| sink.write_all(b"\n"))
                .and_then(|()| sink.flush())
                .map_err(|e| ArgoError::Provider(format!("compliance export stdout: {e}")))
        }
    }
}

fn render_csv(envelope: &Value) -> String {
    let mut out = "table,row_index,row_json\n".to_string();
    if let Some(tables) = envelope.get("tables").and_then(Value::as_object) {
        for (table, entry) in tables {
            if let Some(rows) = entry.get("rows").and_then(Value::as_array) {
                for (idx, row) in rows.iter().enumerate() {
                    let row_json = serde_json::to_string(row).unwrap_or_default();
                    out.push_str(&format!(
                        "{},{},{}\n",
                        csv_cell(table),
                        idx,
                        csv_cell(&row_json)
                    ));
                }
            }
        }
    }
    let integrity = serde_json::to_string(&envelope["integrity"]).unwrap_or_default();
    out.push_str(&format!("integrity,0,{}\n", csv_cell(&integrity)));
    out
}

fn csv_cell(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

fn render_pdf(envelope: &Value) -> String {
    let subject = envelope["subject"].as_str().unwrap_or("unknown");
    let checksum = envelope["integrity"]["checksum"].as_str().unwrap_or("");
    format!(
        "%PDF-1.4\n% argo compliance export\n1 0 obj <<>> endobj\n\
         2 0 obj << /Length 128 >> stream\nBT /F1 12 Tf 72 720 Td \
         (Argo compliance export) Tj 0 -18 Td (subject: {subject}) Tj \
         0 -18 Td (checksum: {checksum}) Tj ET\nendstream endobj\n%%EOF\n"
    )
}

async fn export_blob_table(
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
    // audit-allow: identifier substitution against a static allow-list.
    let sql = format!("SELECT * FROM {table} WHERE tenant_id = ? ORDER BY rowid");
    let rows = sqlx::query(&sql)
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("compliance export {table}: {e}")))?;
    Ok(Some(build_table_entry(&cols, &rows, filters)))
}

async fn export_text_table(
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
    // audit-allow: identifier substitution against a static allow-list.
    let sql = format!("SELECT * FROM {table} WHERE tenant_id = ? ORDER BY rowid");
    let rows = sqlx::query(&sql)
        .bind(tenant.to_string())
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("compliance export {table}: {e}")))?;
    Ok(Some(build_table_entry(&cols, &rows, filters)))
}

async fn export_session_fk_table(
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

async fn append_audit(store: &SessionStore, tenant: TenantId, envelope: &Value) -> ArgoResult<()> {
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
#[path = "export_tests.rs"]
mod tests;
