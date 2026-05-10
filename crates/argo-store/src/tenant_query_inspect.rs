//! Per-tenant inspection (`show_tenant`) and id-rename helpers, split
//! out of `tenant_query.rs` for the 500-LOC ceiling. Public symbols
//! (`show_tenant`, `rename_tenant`) are re-exported through
//! `crate::tenant_query`; external callers should keep importing them
//! from `argo_store::tenant_query::*`.

use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::{Row, Sqlite};
use std::collections::BTreeMap;

use crate::SessionStore;

use super::tenant_query::{
    collect_tenants, count_blob, count_text, format_epoch_auto, has_column,
    list_tenants_with_stats, table_exists, workspace_text, BLOB_TENANT_TABLES,
    TEXT_TENANT_TABLES,
};

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

async fn tenant_exists(store: &SessionStore, tenant: TenantId) -> ArgoResult<bool> {
    Ok(collect_tenants(store)
        .await?
        .contains_key(&tenant.to_string()))
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
    // audit-allow: identifier substitution
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
    // audit-allow: identifier substitution
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
