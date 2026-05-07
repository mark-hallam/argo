//! Tenant inspection and id-rename SQL helpers.

use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Datelike, TimeZone, Utc};
use serde_json::{json, Value};
use sqlx::{Row, Sqlite};
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::SessionStore;

const BLOB_TENANT_TABLES: &[&str] = &[
    "agent",
    "memory_entry",
    "role",
    "role_binding",
    "role_duty",
    "session",
    "shared_team_memory",
    "spend_ledger",
    "worker_budget",
    "workspace",
];

const TEXT_TENANT_TABLES: &[&str] = &[
    "duty",
    "handoff_routing_rule",
    "org_member",
    "org_unit",
    "persona",
    "roster",
    "worker",
];

/// List tenants newest-first with worker and spend stats.
pub async fn list_tenants_with_stats(
    store: &SessionStore,
    include_archived: bool,
    now: DateTime<Utc>,
) -> ArgoResult<Vec<Value>> {
    let mut rows = Vec::new();
    for tenant in collect_tenants(store).await?.into_values() {
        let archived_at = workspace_i64(store, tenant, "archived_at").await?;
        if archived_at.is_some() && !include_archived {
            continue;
        }
        let created_at = tenant_created_at(store, tenant).await?;
        let today = spend_since(store, tenant, day_start_micros(now)).await?;
        let month = spend_since(store, tenant, month_start_micros(now)).await?;
        rows.push(json!({
            "id": tenant.to_string(),
            "name": workspace_text(store, tenant, "name").await?,
            "owner_email": workspace_text(store, tenant, "owner_email").await?,
            "created_at": created_at,
            "created_at_rfc3339": created_at.and_then(format_epoch_auto),
            "archived_at": archived_at,
            "archived_at_rfc3339": archived_at.and_then(format_epoch_auto),
            "worker_count": count_text(store, "worker", tenant).await?,
            "spend_today_usd_micros": today,
            "spend_this_month_usd_micros": month,
            "spend_today": micros_to_usd(today),
            "spend_this_month": micros_to_usd(month),
        }));
    }
    rows.sort_by(|a, b| {
        row_sort_key(b)
            .cmp(&row_sort_key(a))
            .then_with(|| a["id"].as_str().unwrap_or_default().cmp(b["id"].as_str().unwrap_or_default()))
    });
    Ok(rows)
}

/// Load one tenant detail object.
pub async fn show_tenant(
    store: &SessionStore,
    tenant: TenantId,
    now: DateTime<Utc>,
) -> ArgoResult<Option<Value>> {
    let Some(summary) = list_tenants_with_stats(store, true, now)
        .await?
        .into_iter()
        .find(|row| row["id"].as_str() == Some(&tenant.to_string()))
    else {
        return Ok(None);
    };
    Ok(Some(json!({
        "summary": summary,
        "name": workspace_text(store, tenant, "name").await?,
        "root_path": workspace_text(store, tenant, "root_path").await?,
        "owner_email": workspace_text(store, tenant, "owner_email").await?,
        "workspace_count": count_blob(store, "workspace", tenant).await?,
        "session_count": count_blob(store, "session", tenant).await?,
        "roster": roster_summary(store, tenant).await?,
        "recent_activity": recent_activity(store, tenant).await?,
        "channels": channel_presence(store, tenant).await?,
        "classification_rules": [],
    })))
}

/// Atomically rename a tenant id across known tenant-scoped tables.
pub async fn rename_tenant(
    store: &SessionStore,
    old: TenantId,
    new: TenantId,
    renamed_at: i64,
) -> ArgoResult<Value> {
    if old == new {
        return Err(ArgoError::Provider("tenant rename: ids are identical".into()));
    }
    if !tenant_exists(store, old).await? {
        return Err(ArgoError::Provider(format!("no tenant '{old}'")));
    }
    if tenant_exists(store, new).await? {
        return Err(ArgoError::Provider(format!(
            "tenant rename: destination '{new}' already exists"
        )));
    }

    let mut tx = store
        .pool()
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant rename tx: {e}")))?;
    let mut updated = Vec::new();
    for table in TEXT_TENANT_TABLES {
        if has_column(store, table, "tenant_id").await? {
            updated.push(update_text(&mut tx, table, old, new).await?);
        }
    }
    for table in BLOB_TENANT_TABLES {
        if has_column(store, table, "tenant_id").await? {
            updated.push(update_blob(&mut tx, table, old, new).await?);
        }
    }
    if table_exists(store, "audit_record").await? {
        let rows = sqlx::query(
            "UPDATE audit_record SET payload_json = replace(payload_json, ?, ?) \
             WHERE instr(payload_json, ?) > 0",
        )
        .bind(old.to_string())
        .bind(new.to_string())
        .bind(old.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant rename audit_record: {e}")))?
        .rows_affected();
        updated.push(json!({"table": "audit_record", "rows": rows}));
    }
    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant rename commit: {e}")))?;
    Ok(json!({
        "old_id": old.to_string(),
        "new_id": new.to_string(),
        "renamed_at": renamed_at,
        "updated_tables": updated,
    }))
}

async fn collect_tenants(store: &SessionStore) -> ArgoResult<BTreeMap<String, TenantId>> {
    let mut out = BTreeMap::new();
    for table in BLOB_TENANT_TABLES {
        if has_column(store, table, "tenant_id").await? {
            let sql = format!("SELECT DISTINCT tenant_id FROM {table} WHERE tenant_id IS NOT NULL");
            for row in query_all(store, &sql, table).await? {
                let raw: Vec<u8> = row.get("tenant_id");
                if let Ok(uuid) = Uuid::from_slice(&raw) {
                    let tenant = TenantId(uuid);
                    out.insert(tenant.to_string(), tenant);
                }
            }
        }
    }
    for table in TEXT_TENANT_TABLES {
        if has_column(store, table, "tenant_id").await? {
            let sql = format!("SELECT DISTINCT tenant_id FROM {table} WHERE tenant_id IS NOT NULL");
            for row in query_all(store, &sql, table).await? {
                let raw: String = row.get("tenant_id");
                if let Ok(uuid) = Uuid::parse_str(&raw) {
                    let tenant = TenantId(uuid);
                    out.insert(tenant.to_string(), tenant);
                }
            }
        }
    }
    Ok(out)
}

async fn query_all(
    store: &SessionStore,
    sql: &str,
    label: &str,
) -> ArgoResult<Vec<sqlx::sqlite::SqliteRow>> {
    sqlx::query(sql)
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant query {label}: {e}")))
}

async fn tenant_exists(store: &SessionStore, tenant: TenantId) -> ArgoResult<bool> {
    Ok(collect_tenants(store)
        .await?
        .contains_key(&tenant.to_string()))
}

async fn tenant_created_at(store: &SessionStore, tenant: TenantId) -> ArgoResult<Option<i64>> {
    let mut values = Vec::new();
    values.push(min_blob(store, "workspace", "created_at", tenant).await?);
    values.push(min_blob(store, "session", "started_at", tenant).await?);
    values.push(min_text(store, "worker", "created_at", tenant).await?);
    Ok(values.into_iter().flatten().min_by_key(|v| normalize_epoch_micros(*v)))
}

async fn workspace_text(store: &SessionStore, tenant: TenantId, column: &str) -> ArgoResult<Option<String>> {
    if !has_column(store, "workspace", column).await? {
        return Ok(None);
    }
    let sql = format!("SELECT {column} FROM workspace WHERE tenant_id = ? ORDER BY created_at ASC LIMIT 1");
    let row = sqlx::query(&sql)
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_optional(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant workspace {column}: {e}")))?;
    Ok(row.and_then(|row| row.get(column)))
}

async fn workspace_i64(store: &SessionStore, tenant: TenantId, column: &str) -> ArgoResult<Option<i64>> {
    if !has_column(store, "workspace", column).await? {
        return Ok(None);
    }
    let sql = format!("SELECT {column} FROM workspace WHERE tenant_id = ? ORDER BY created_at ASC LIMIT 1");
    let row = sqlx::query(&sql)
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_optional(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant workspace {column}: {e}")))?;
    Ok(row.and_then(|row| row.get(column)))
}

async fn count_blob(store: &SessionStore, table: &str, tenant: TenantId) -> ArgoResult<i64> {
    if !has_column(store, table, "tenant_id").await? {
        return Ok(0);
    }
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE tenant_id = ?");
    sqlx::query_scalar(&sql)
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant count {table}: {e}")))
}

async fn count_text(store: &SessionStore, table: &str, tenant: TenantId) -> ArgoResult<i64> {
    if !has_column(store, table, "tenant_id").await? {
        return Ok(0);
    }
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE tenant_id = ?");
    sqlx::query_scalar(&sql)
        .bind(tenant.to_string())
        .fetch_one(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant count {table}: {e}")))
}

async fn min_blob(store: &SessionStore, table: &str, column: &str, tenant: TenantId) -> ArgoResult<Option<i64>> {
    if !has_column(store, table, "tenant_id").await? || !has_column(store, table, column).await? {
        return Ok(None);
    }
    let sql = format!("SELECT MIN({column}) FROM {table} WHERE tenant_id = ?");
    sqlx::query_scalar(&sql)
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant min {table}: {e}")))
}

async fn min_text(store: &SessionStore, table: &str, column: &str, tenant: TenantId) -> ArgoResult<Option<i64>> {
    if !has_column(store, table, "tenant_id").await? || !has_column(store, table, column).await? {
        return Ok(None);
    }
    let sql = format!("SELECT MIN({column}) FROM {table} WHERE tenant_id = ?");
    sqlx::query_scalar(&sql)
        .bind(tenant.to_string())
        .fetch_one(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant min {table}: {e}")))
}

async fn spend_since(store: &SessionStore, tenant: TenantId, since: i64) -> ArgoResult<i64> {
    if !has_column(store, "spend_ledger", "tenant_id").await? {
        return Ok(0);
    }
    sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger \
         WHERE tenant_id = ? AND charged_at_micros >= ?",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .bind(since)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("tenant spend: {e}")))
}

async fn roster_summary(store: &SessionStore, tenant: TenantId) -> ArgoResult<Value> {
    let roster_count = count_text(store, "roster", tenant).await?;
    let roster_member_count = if table_exists(store, "roster_member").await?
        && table_exists(store, "roster").await?
    {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM roster_member rm \
             JOIN roster r ON r.id = rm.roster_id WHERE r.tenant_id = ?",
        )
        .bind(tenant.to_string())
        .fetch_one(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant roster members: {e}")))?
    } else {
        0
    };
    Ok(json!({"roster_count": roster_count, "roster_member_count": roster_member_count}))
}

async fn recent_activity(store: &SessionStore, tenant: TenantId) -> ArgoResult<Vec<Value>> {
    if !table_exists(store, "audit_record").await? {
        return Ok(Vec::new());
    }
    let rows = sqlx::query(
        "SELECT a.seq, a.kind, a.payload_json, a.created_at FROM audit_record a \
         LEFT JOIN session s ON s.id = a.session_id \
         WHERE s.tenant_id = ? OR instr(a.payload_json, ?) > 0 \
         ORDER BY a.created_at DESC, a.seq DESC LIMIT 5",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .bind(tenant.to_string())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("tenant audit: {e}")))?;
    Ok(rows.into_iter().map(audit_json).collect())
}

fn audit_json(row: sqlx::sqlite::SqliteRow) -> Value {
    let raw: String = row.get("payload_json");
    let created_at = row.get("created_at");
    json!({
        "seq": row.get::<i64, _>("seq"),
        "kind": row.get::<String, _>("kind"),
        "created_at": created_at,
        "created_at_rfc3339": format_epoch_auto(created_at),
        "payload": serde_json::from_str::<Value>(&raw).unwrap_or(Value::String(raw)),
    })
}

async fn channel_presence(store: &SessionStore, tenant: TenantId) -> ArgoResult<Vec<Value>> {
    let mut counts: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    if has_column(store, "worker", "channels_json").await? {
        for row in sqlx::query("SELECT channels_json FROM worker WHERE tenant_id = ?")
            .bind(tenant.to_string())
            .fetch_all(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("tenant worker channels: {e}")))?
        {
            add_channels(row.get("channels_json"), true, &mut counts);
        }
    }
    if has_column(store, "roster", "channels_json").await? {
        for row in sqlx::query("SELECT channels_json FROM roster WHERE tenant_id = ?")
            .bind(tenant.to_string())
            .fetch_all(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("tenant roster channels: {e}")))?
        {
            add_channels(row.get("channels_json"), false, &mut counts);
        }
    }
    Ok(counts
        .into_iter()
        .map(|(channel, (worker_refs, roster_refs))| {
            json!({
                "channel": channel,
                "worker_refs": worker_refs,
                "roster_refs": roster_refs,
                "manifest_present": false,
                "manifest_path": null,
            })
        })
        .collect())
}

fn add_channels(raw: String, worker: bool, counts: &mut BTreeMap<String, (i64, i64)>) {
    let Ok(Value::Array(items)) = serde_json::from_str::<Value>(&raw) else {
        return;
    };
    for item in items {
        if let Some(channel) = item.get("channel").and_then(Value::as_str) {
            let entry = counts.entry(channel.to_string()).or_default();
            if worker {
                entry.0 += 1;
            } else {
                entry.1 += 1;
            }
        }
    }
}

async fn update_text(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    table: &str,
    old: TenantId,
    new: TenantId,
) -> ArgoResult<Value> {
    let sql = format!("UPDATE {table} SET tenant_id = ? WHERE tenant_id = ?");
    let rows = sqlx::query(&sql)
        .bind(new.to_string())
        .bind(old.to_string())
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant rename {table}: {e}")))?
        .rows_affected();
    Ok(json!({"table": table, "rows": rows}))
}

async fn update_blob(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    table: &str,
    old: TenantId,
    new: TenantId,
) -> ArgoResult<Value> {
    let sql = format!("UPDATE {table} SET tenant_id = ? WHERE tenant_id = ?");
    let rows = sqlx::query(&sql)
        .bind(new.0.as_bytes().as_slice())
        .bind(old.0.as_bytes().as_slice())
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant rename {table}: {e}")))?
        .rows_affected();
    Ok(json!({"table": table, "rows": rows}))
}

async fn table_exists(store: &SessionStore, table: &str) -> ArgoResult<bool> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(table)
            .fetch_one(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("table exists {table}: {e}")))?;
    Ok(count > 0)
}

async fn has_column(store: &SessionStore, table: &str, column: &str) -> ArgoResult<bool> {
    if !table_exists(store, table).await? {
        return Ok(false);
    }
    let rows = sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("table info {table}: {e}")))?;
    Ok(rows
        .iter()
        .any(|row| row.get::<String, _>("name") == column))
}

fn row_sort_key(row: &Value) -> i64 {
    normalize_epoch_micros(row["created_at"].as_i64().unwrap_or_default())
}

fn day_start_micros(now: DateTime<Utc>) -> i64 {
    now.date_naive()
        .and_hms_opt(0, 0, 0)
        .map(|dt| dt.and_utc().timestamp_micros())
        .unwrap_or_else(|| now.timestamp_micros())
}

fn month_start_micros(now: DateTime<Utc>) -> i64 {
    Utc.with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .single()
        .map(|dt| dt.timestamp_micros())
        .unwrap_or_else(|| now.timestamp_micros())
}

fn normalize_epoch_micros(value: i64) -> i64 {
    let abs = value.unsigned_abs();
    if abs < 20_000_000_000 {
        value.saturating_mul(1_000_000)
    } else if abs < 20_000_000_000_000 {
        value.saturating_mul(1_000)
    } else {
        value
    }
}

fn format_epoch_auto(value: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp_micros(normalize_epoch_micros(value)).map(|dt| dt.to_rfc3339())
}

fn micros_to_usd(micros: i64) -> String {
    format!("${:.4}", micros as f64 / 1_000_000.0)
}
