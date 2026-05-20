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
use uuid::Uuid;

const AUDIT_KIND: &str = "compliance.export";

#[path = "export_render.rs"]
mod export_render;
#[path = "export_tables.rs"]
mod export_tables;

#[cfg(test)]
use export_render::{render_csv, render_pdf};
use export_render::{stamp_integrity, write_envelope};
use export_tables::{export_blob_table, export_session_fk_table, export_text_table};

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
