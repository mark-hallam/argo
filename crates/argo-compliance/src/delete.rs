//! `argo compliance delete` GDPR Article 17 right-to-erasure flow.
//!
//! Cascades a single-transaction delete across every tenant-scoped
//! table for the supplied `tenant_id`. Audit rows are not deleted —
//! they are tombstoned in place: PII fields in `payload_json` are
//! scrubbed and the per-session hash chain is recomputed cascade-
//! forward so [`argo_audit::verify`] still succeeds. The delete
//! itself is recorded as a fresh `compliance.delete` audit row in
//! the global (NULL session) chain.
//!
//! Confirmation guard: `--confirm <id>` must exactly equal `--tenant
//! <id>`; mismatch exits 2 without touching any row. `--dry-run`
//! reports per-table row counts and rolls the transaction back.
//!
//! See `compliance_tenant_tables` for the shared table catalogue
//! and `compliance_export` for the GDPR Article 15 sibling.
use crate::tenant_tables::{delete_order, BLOB_TENANT_TABLES, TEXT_TENANT_TABLES};
use argo_audit::compute_chain_hash;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::{Row, Sqlite, Transaction};
use std::collections::BTreeMap;
use uuid::Uuid;

const AUDIT_KIND: &str = "compliance.delete";
const TOMBSTONE_MARKER: &str = "<erased>";
/// Process exit code returned when `--confirm` does not match `--tenant`.
pub const EXIT_CONFIRM_MISMATCH: i32 = 2;

#[derive(Debug, Clone)]
pub struct ComplianceDeleteArgs {
    pub tenant: String,
    pub confirm: String,
    pub dry_run: bool,
    pub json: bool,
}

pub async fn run(args: ComplianceDeleteArgs, store: &SessionStore) -> ArgoResult<i32> {
    let tenant = match validate_confirm(&args)? {
        Some(t) => t,
        None => return Ok(EXIT_CONFIRM_MISMATCH),
    };
    let summary = execute(store, tenant, args.dry_run).await?;
    print_summary(&summary, args.dry_run, args.json)?;
    Ok(0)
}

/// Returns `Some(tenant)` if `--confirm` matches `--tenant`, `None`
/// on mismatch (caller must exit with [`EXIT_CONFIRM_MISMATCH`]).
/// Errors only on malformed input (empty / non-UUID).
pub fn validate_confirm(args: &ComplianceDeleteArgs) -> ArgoResult<Option<TenantId>> {
    let tenant = parse_tenant(&args.tenant)?;
    let confirm = args.confirm.trim();
    if confirm.is_empty() {
        return Err(ArgoError::Provider(
            "compliance delete: --confirm <id> is required".into(),
        ));
    }
    if confirm != args.tenant.trim() {
        eprintln!(
            "compliance delete: --confirm '{}' does not match --tenant '{}'",
            args.confirm, args.tenant
        );
        return Ok(None);
    }
    Ok(Some(tenant))
}

#[derive(Debug, Default)]
pub(crate) struct DeleteSummary {
    pub tenant: String,
    pub audit_rows_tombstoned: u64,
    pub deleted: BTreeMap<String, u64>,
    pub total: u64,
}

pub(crate) async fn execute(
    store: &SessionStore,
    tenant: TenantId,
    dry_run: bool,
) -> ArgoResult<DeleteSummary> {
    let mut tx = store
        .pool()
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("compliance delete tx begin: {e}")))?;
    let mut summary = DeleteSummary {
        tenant: tenant.to_string(),
        ..DeleteSummary::default()
    };
    let session_ids = tenant_session_ids(&mut tx, tenant).await?;
    let audit_count = audit_row_count(&mut tx, &session_ids).await?;
    summary.audit_rows_tombstoned = audit_count;
    summary.total = audit_count;
    for table in delete_order() {
        if !table_exists(&mut tx, table).await? {
            continue;
        }
        let rows = if dry_run {
            count_table(&mut tx, table, tenant).await?
        } else {
            delete_table(&mut tx, table, tenant).await?
        };
        summary.deleted.insert(table.to_string(), rows);
        summary.total += rows;
    }
    if !dry_run {
        tombstone_audit_chains(&mut tx, &session_ids).await?;
        append_delete_audit(&mut tx, tenant, &summary).await?;
        tx.commit()
            .await
            .map_err(|e| ArgoError::Provider(format!("compliance delete commit: {e}")))?;
    } else {
        tx.rollback()
            .await
            .map_err(|e| ArgoError::Provider(format!("compliance delete rollback: {e}")))?;
    }
    Ok(summary)
}

async fn tenant_session_ids(
    tx: &mut Transaction<'_, Sqlite>,
    tenant: TenantId,
) -> ArgoResult<Vec<Vec<u8>>> {
    if !table_exists(tx, "session").await? {
        return Ok(Vec::new());
    }
    let rows = sqlx::query("SELECT id FROM session WHERE tenant_id = ?")
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("compliance delete sessions: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|r| r.get::<Vec<u8>, _>("id"))
        .collect())
}

async fn audit_row_count(
    tx: &mut Transaction<'_, Sqlite>,
    session_ids: &[Vec<u8>],
) -> ArgoResult<u64> {
    if session_ids.is_empty() || !table_exists(tx, "audit_record").await? {
        return Ok(0);
    }
    let mut total = 0_i64;
    for sid in session_ids {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_record WHERE session_id = ?")
            .bind(sid.as_slice())
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("compliance delete audit count: {e}")))?;
        total += n;
    }
    Ok(u64::try_from(total).unwrap_or(0))
}

enum TenantBind {
    Blob(Vec<u8>),
    Text(String),
}

fn classify(table: &str, tenant: TenantId) -> (String, String, TenantBind) {
    if BLOB_TENANT_TABLES.contains(&table) {
        (
            // audit-allow: identifier substitution against a static allow-list.
            format!("SELECT COUNT(*) FROM {table} WHERE tenant_id = ?"),
            format!("DELETE FROM {table} WHERE tenant_id = ?"),
            TenantBind::Blob(tenant.0.as_bytes().to_vec()),
        )
    } else if TEXT_TENANT_TABLES.contains(&table) {
        (
            // audit-allow: identifier substitution against a static allow-list.
            format!("SELECT COUNT(*) FROM {table} WHERE tenant_id = ?"),
            format!("DELETE FROM {table} WHERE tenant_id = ?"),
            TenantBind::Text(tenant.to_string()),
        )
    } else {
        (
            // session-FK child: count via JOIN, delete via subselect.
            // audit-allow: identifier substitution against a static allow-list.
            format!(
                "SELECT COUNT(*) FROM {table} t \
                 JOIN session s ON s.id = t.session_id \
                 WHERE s.tenant_id = ?"
            ),
            format!(
                "DELETE FROM {table} WHERE session_id IN \
                 (SELECT id FROM session WHERE tenant_id = ?)"
            ),
            TenantBind::Blob(tenant.0.as_bytes().to_vec()),
        )
    }
}

async fn count_table(
    tx: &mut Transaction<'_, Sqlite>,
    table: &str,
    tenant: TenantId,
) -> ArgoResult<u64> {
    let (sql, _, bind) = classify(table, tenant);
    let mut q = sqlx::query_scalar::<_, i64>(&sql);
    q = match &bind {
        TenantBind::Blob(b) => q.bind(b.as_slice()),
        TenantBind::Text(s) => q.bind(s.clone()),
    };
    let n = q
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("compliance delete count {table}: {e}")))?;
    Ok(u64::try_from(n).unwrap_or(0))
}

async fn delete_table(
    tx: &mut Transaction<'_, Sqlite>,
    table: &str,
    tenant: TenantId,
) -> ArgoResult<u64> {
    let (_, sql, bind) = classify(table, tenant);
    let mut q = sqlx::query(&sql);
    q = match &bind {
        TenantBind::Blob(b) => q.bind(b.as_slice()),
        TenantBind::Text(s) => q.bind(s.clone()),
    };
    let res = q
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("compliance delete {table}: {e}")))?;
    Ok(res.rows_affected())
}

/// For each tenant-owned session, walk the audit chain in seq order,
/// replace PII payload fields with `<erased>`, and cascade-recompute
/// `parent_hash` + `self_hash` so [`argo_audit::verify`] still passes.
async fn tombstone_audit_chains(
    tx: &mut Transaction<'_, Sqlite>,
    session_ids: &[Vec<u8>],
) -> ArgoResult<()> {
    if session_ids.is_empty() || !table_exists(tx, "audit_record").await? {
        return Ok(());
    }
    let redacted_at = Utc::now().to_rfc3339();
    for sid in session_ids {
        let rows = sqlx::query(
            "SELECT seq, kind, payload_json, parent_hash FROM audit_record \
             WHERE session_id = ? ORDER BY seq ASC",
        )
        .bind(sid.as_slice())
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("compliance delete audit fetch: {e}")))?;
        let mut parent: [u8; 32] = rows
            .first()
            .map(|r| hash32(&r.get::<Vec<u8>, _>("parent_hash")))
            .unwrap_or([0_u8; 32]);
        for row in rows {
            let seq: i64 = row.get("seq");
            let kind: String = row.get("kind");
            let original: String = row.get("payload_json");
            let redacted = redact_payload(&original, &redacted_at);
            let self_hash = compute_chain_hash(&parent, &kind, &redacted);
            sqlx::query(
                "UPDATE audit_record SET payload_json = ?, parent_hash = ?, self_hash = ? \
                 WHERE seq = ?",
            )
            .bind(&redacted)
            .bind(parent.as_slice())
            .bind(self_hash.as_slice())
            .bind(seq)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("compliance delete tombstone: {e}")))?;
            parent = self_hash;
        }
    }
    Ok(())
}

/// Build the redacted payload: keep `actor` and `target` keys verbatim,
/// stamp a `tombstone` marker, and replace the rest with `<erased>`.
pub(crate) fn redact_payload(original: &str, redacted_at: &str) -> String {
    let mut out = serde_json::Map::new();
    out.insert("tombstone".into(), Value::Bool(true));
    out.insert("redacted_at".into(), Value::String(redacted_at.to_string()));
    if let Ok(Value::Object(parsed)) = serde_json::from_str::<Value>(original) {
        for key in ["actor", "target", "action"] {
            if let Some(v) = parsed.get(key) {
                out.insert(key.to_string(), v.clone());
            }
        }
        for key in parsed.keys() {
            if !out.contains_key(key) {
                out.insert(key.clone(), Value::String(TOMBSTONE_MARKER.into()));
            }
        }
    }
    serde_json::to_string(&Value::Object(out))
        .unwrap_or_else(|_| format!("{{\"tombstone\":true,\"redacted_at\":\"{redacted_at}\"}}"))
}

async fn append_delete_audit(
    tx: &mut Transaction<'_, Sqlite>,
    tenant: TenantId,
    summary: &DeleteSummary,
) -> ArgoResult<()> {
    let parent = last_global_hash(tx).await?;
    let at = Utc::now();
    let payload = json!({
        "actor": "cli",
        "action": AUDIT_KIND,
        "target": tenant.to_string(),
        "row_total": summary.total,
        "audit_rows_tombstoned": summary.audit_rows_tombstoned,
        "at": at.to_rfc3339(),
    });
    let canonical = serde_json::to_string(&payload)
        .map_err(|e| ArgoError::Provider(format!("compliance delete audit json: {e}")))?;
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
    .execute(&mut **tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("compliance delete audit: {e}")))?;
    Ok(())
}

async fn last_global_hash(tx: &mut Transaction<'_, Sqlite>) -> ArgoResult<[u8; 32]> {
    let bytes: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT self_hash FROM audit_record WHERE session_id IS NULL ORDER BY seq DESC LIMIT 1",
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("compliance delete last hash: {e}")))?;
    Ok(bytes.as_deref().map_or([0_u8; 32], hash32))
}

async fn table_exists(tx: &mut Transaction<'_, Sqlite>, table: &str) -> ArgoResult<bool> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(table)
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("compliance delete exists {table}: {e}")))?;
    Ok(count > 0)
}

fn hash32(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0_u8; 32];
    if bytes.len() == 32 {
        out.copy_from_slice(bytes);
    }
    out
}

fn parse_tenant(raw: &str) -> ArgoResult<TenantId> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ArgoError::Provider(
            "compliance delete: --tenant <id> is required".into(),
        ));
    }
    Uuid::parse_str(trimmed)
        .map(TenantId)
        .map_err(|e| ArgoError::Provider(format!("compliance delete: tenant '{raw}': {e}")))
}

fn print_summary(summary: &DeleteSummary, dry_run: bool, as_json: bool) -> ArgoResult<()> {
    if as_json {
        let value = json!({
            "tenant": summary.tenant,
            "dry_run": dry_run,
            "audit_rows_tombstoned": summary.audit_rows_tombstoned,
            "deleted": summary.deleted,
            "total": summary.total,
        });
        println!(
            "{}",
            serde_json::to_string(&value)
                .map_err(|e| ArgoError::Provider(format!("compliance delete json: {e}")))?
        );
        return Ok(());
    }
    let suffix = if dry_run {
        " (will be tombstoned)"
    } else {
        " tombstoned"
    };
    println!("audit:{:>8} rows{}", summary.audit_rows_tombstoned, suffix);
    for (table, n) in &summary.deleted {
        if *n == 0 {
            continue;
        }
        println!("{table}:{:>8} rows", n);
    }
    let trailer = if dry_run {
        " (dry run — no changes)"
    } else {
        ""
    };
    println!("TOTAL:{:>8} rows{}", summary.total, trailer);
    Ok(())
}

#[cfg(test)]
#[path = "delete_tests.rs"]
mod tests;
