//! Tenant archive support for `argo tenant`.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::IdentityStore;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::json;
use sqlx::Row;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct TenantArchiveOut {
    tenant_id: String,
    archived_at: i64,
    archived_at_rfc3339: Option<String>,
}

pub async fn run(id: &str, json_out: bool) -> ArgoResult<()> {
    let tenant = parse_tenant(id)?;
    let (store, _) = open_store().await?;
    let archived_at = archive_tenant(&store, tenant, Utc::now().timestamp_micros()).await?;
    let out = TenantArchiveOut {
        tenant_id: tenant.to_string(),
        archived_at,
        archived_at_rfc3339: format_micros(archived_at),
    };
    if json_out {
        print_json(&json!({"tenant_archive": out}))
    } else {
        println!("archived tenant {}", out.tenant_id);
        Ok(())
    }
}

pub(crate) async fn open_store() -> ArgoResult<(SessionStore, PathBuf)> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    IdentityStore::new(&store).ensure_schema().await?;
    ensure_tenant_lifecycle_schema(&store).await?;
    Ok((store, cwd))
}

pub(crate) async fn ensure_tenant_lifecycle_schema(store: &SessionStore) -> ArgoResult<()> {
    ensure_column(store, "workspace", "archived_at", "INTEGER").await?;
    ensure_column(store, "workspace", "owner_email", "TEXT").await?;
    for sql in [REJECT_ARCHIVED_WORKER_INSERT, REJECT_ARCHIVED_WORKER_UPDATE] {
        sqlx::query(sql)
            .execute(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("tenant lifecycle trigger: {e}")))?;
    }
    Ok(())
}

const REJECT_ARCHIVED_WORKER_INSERT: &str =
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
END";

const REJECT_ARCHIVED_WORKER_UPDATE: &str =
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
END";

async fn ensure_column(
    store: &SessionStore,
    table: &str,
    column: &str,
    declaration: &str,
) -> ArgoResult<()> {
    if column_exists(store, table, column).await? {
        return Ok(());
    }
    let sql = format!("ALTER TABLE {table} ADD COLUMN {column} {declaration}");
    sqlx::query(&sql)
        .execute(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant lifecycle column {column}: {e}")))?;
    Ok(())
}

async fn column_exists(store: &SessionStore, table: &str, column: &str) -> ArgoResult<bool> {
    let rows = sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant lifecycle table info: {e}")))?;
    Ok(rows
        .iter()
        .any(|row| row.get::<String, _>("name") == column))
}

pub(crate) async fn primary_workspace_id(
    store: &SessionStore,
    tenant: TenantId,
) -> ArgoResult<Vec<u8>> {
    sqlx::query_scalar(
        "SELECT id FROM workspace WHERE tenant_id = ? ORDER BY created_at ASC LIMIT 1",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("tenant workspace lookup: {e}")))?
    .ok_or_else(|| ArgoError::Provider(format!("no tenant '{tenant}'")))
}

pub(crate) async fn tenant_owner(
    store: &SessionStore,
    tenant: TenantId,
) -> ArgoResult<Option<String>> {
    let row = sqlx::query(
        "SELECT owner_email FROM workspace WHERE tenant_id = ? ORDER BY created_at ASC LIMIT 1",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("tenant owner lookup: {e}")))?;
    Ok(row.and_then(|row| row.get("owner_email")))
}

pub(crate) async fn archive_tenant(
    store: &SessionStore,
    tenant: TenantId,
    archived_at: i64,
) -> ArgoResult<i64> {
    primary_workspace_id(store, tenant).await?;
    sqlx::query("UPDATE workspace SET archived_at = COALESCE(archived_at, ?) WHERE tenant_id = ?")
        .bind(archived_at)
        .bind(tenant.0.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant archive: {e}")))?;
    Ok(tenant_archived_at(store, tenant)
        .await?
        .unwrap_or(archived_at))
}

pub(crate) async fn tenant_archived_at(
    store: &SessionStore,
    tenant: TenantId,
) -> ArgoResult<Option<i64>> {
    let row = sqlx::query(
        "SELECT archived_at FROM workspace WHERE tenant_id = ? ORDER BY created_at ASC LIMIT 1",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("tenant archived_at lookup: {e}")))?;
    Ok(row.and_then(|row| row.get("archived_at")))
}

pub(crate) fn parse_tenant(raw: &str) -> ArgoResult<TenantId> {
    Uuid::parse_str(raw)
        .map(TenantId)
        .map_err(|e| ArgoError::Provider(format!("tenant id '{raw}': {e}")))
}

pub(crate) fn print_json<T: Serialize>(value: &T) -> ArgoResult<()> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|e| ArgoError::Provider(format!("serialize tenant JSON: {e}")))?;
    println!("{body}");
    Ok(())
}

pub(crate) fn format_micros(micros: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp_micros(micros).map(|ts| ts.to_rfc3339())
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_identity::IdentityStore;
    use argo_types::WorkspaceId;

    async fn store() -> SessionStore {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        IdentityStore::new(&store).ensure_schema().await.unwrap();
        ensure_tenant_lifecycle_schema(&store).await.unwrap();
        store
    }

    async fn insert_workspace(store: &SessionStore, tenant: TenantId, name: &str) {
        sqlx::query(
            "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
             VALUES (?, ?, ?, '.', 1)",
        )
        .bind(WorkspaceId::new().0.as_bytes().as_slice())
        .bind(tenant.0.as_bytes().as_slice())
        .bind(name)
        .execute(store.pool())
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn schema_adds_lifecycle_columns() {
        let store = store().await;
        assert!(column_exists(&store, "workspace", "archived_at")
            .await
            .unwrap());
        assert!(column_exists(&store, "workspace", "owner_email")
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn archive_marks_existing_tenant() {
        let store = store().await;
        let tenant = TenantId::new();
        insert_workspace(&store, tenant, "Acme").await;

        let archived_at = archive_tenant(&store, tenant, 42).await.unwrap();

        assert_eq!(archived_at, 42);
        assert_eq!(tenant_archived_at(&store, tenant).await.unwrap(), Some(42));
    }

    #[tokio::test]
    async fn archived_tenant_rejects_new_worker_rows() {
        let store = store().await;
        let tenant = TenantId::new();
        insert_workspace(&store, tenant, "Acme").await;
        archive_tenant(&store, tenant, 42).await.unwrap();

        let err = sqlx::query(
            "INSERT INTO worker
             (id, tenant_id, vertical_id, persona_id, role_id, memory_scope,
              auto_approval_json, channels_json, created_at)
             VALUES ('w', ?, 'v', 'p', 'r', 'worker_self', '{}', '[]', 1)",
        )
        .bind(tenant.to_string())
        .execute(store.pool())
        .await
        .unwrap_err();

        assert!(err.to_string().contains("tenant is archived"));
    }
}
