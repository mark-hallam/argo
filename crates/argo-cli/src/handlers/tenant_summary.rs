//! Tenant aggregate-summary helpers — DB queries that build the row
//! summaries (`TenantSummary`, `WorkerSummary`, `SessionSummary`) consumed
//! by `argo tenant info` and the legacy table list.

use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub(super) struct TenantSummary {
    pub(super) id: String,
    pub(super) name: Option<String>,
    pub(super) owner_email: Option<String>,
    pub(super) archived_at: Option<i64>,
    pub(super) archived_at_rfc3339: Option<String>,
    pub(super) workspace_count: i64,
    pub(super) worker_count: i64,
    pub(super) session_count: i64,
    pub(super) last_action_at: Option<i64>,
    pub(super) last_action_at_rfc3339: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct WorkerSummary {
    pub(super) id: String,
    pub(super) vertical_id: String,
    pub(super) persona: String,
    pub(super) created_at: i64,
    pub(super) created_at_rfc3339: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct SessionSummary {
    pub(super) id: String,
    pub(super) title: Option<String>,
    pub(super) model: String,
    pub(super) provider: String,
    pub(super) started_at: i64,
    pub(super) started_at_rfc3339: Option<String>,
}

#[derive(Debug)]
pub(super) struct TenantLifecycle {
    pub(super) owner_email: Option<String>,
    pub(super) archived_at: Option<i64>,
}

pub(super) async fn summary(store: &SessionStore, tenant: TenantId) -> ArgoResult<TenantSummary> {
    let workspace_count = count_blob(store, "workspace", tenant).await?;
    let session_count = count_blob(store, "session", tenant).await?;
    let worker_count = count_text(store, "worker", tenant).await?;
    let last_action_at = last_action_at(store, tenant).await?;
    let lifecycle = tenant_lifecycle(store, tenant).await?;
    Ok(TenantSummary {
        id: tenant.to_string(),
        name: tenant_name(store, tenant).await?,
        owner_email: lifecycle.owner_email,
        archived_at: lifecycle.archived_at,
        archived_at_rfc3339: lifecycle.archived_at.and_then(format_micros),
        workspace_count,
        worker_count,
        session_count,
        last_action_at,
        last_action_at_rfc3339: last_action_at.and_then(format_micros),
    })
}

pub(super) async fn tenant_lifecycle(
    store: &SessionStore,
    tenant: TenantId,
) -> ArgoResult<TenantLifecycle> {
    let row = sqlx::query(
        "SELECT owner_email, archived_at FROM workspace \
         WHERE tenant_id = ? ORDER BY created_at ASC LIMIT 1",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("tenant lifecycle: {e}")))?;
    Ok(row.map_or(
        TenantLifecycle {
            owner_email: None,
            archived_at: None,
        },
        |row| TenantLifecycle {
            owner_email: row.get("owner_email"),
            archived_at: row.get("archived_at"),
        },
    ))
}

async fn tenant_name(store: &SessionStore, tenant: TenantId) -> ArgoResult<Option<String>> {
    sqlx::query_scalar(
        "SELECT name FROM workspace WHERE tenant_id = ? ORDER BY created_at ASC LIMIT 1",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("tenant name: {e}")))
}

async fn count_blob(store: &SessionStore, table: &str, tenant: TenantId) -> ArgoResult<i64> {
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE tenant_id = ?");
    sqlx::query_scalar(&sql)
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant count {table}: {e}")))
}

async fn count_text(store: &SessionStore, table: &str, tenant: TenantId) -> ArgoResult<i64> {
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE tenant_id = ?");
    sqlx::query_scalar(&sql)
        .bind(tenant.to_string())
        .fetch_one(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant count {table}: {e}")))
}

async fn last_action_at(store: &SessionStore, tenant: TenantId) -> ArgoResult<Option<i64>> {
    let mut values = Vec::new();
    values.push(
        max_blob(
            store,
            "SELECT MAX(started_at) FROM session WHERE tenant_id = ?",
            tenant,
        )
        .await?,
    );
    values.push(
        max_blob(
            store,
            "SELECT MAX(ended_at) FROM session WHERE tenant_id = ?",
            tenant,
        )
        .await?,
    );
    values.push(max_blob(store, "SELECT MAX(m.created_at) FROM message m JOIN session s ON s.id = m.session_id WHERE s.tenant_id = ?", tenant).await?);
    values.push(max_blob(store, "SELECT MAX(a.created_at) FROM audit_record a JOIN session s ON s.id = a.session_id WHERE s.tenant_id = ?", tenant).await?);
    values.push(
        max_text(
            store,
            "SELECT MAX(created_at) FROM worker WHERE tenant_id = ?",
            tenant,
        )
        .await?,
    );
    Ok(values.into_iter().flatten().max())
}

async fn max_blob(store: &SessionStore, sql: &str, tenant: TenantId) -> ArgoResult<Option<i64>> {
    sqlx::query_scalar(sql)
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant max: {e}")))
}

async fn max_text(store: &SessionStore, sql: &str, tenant: TenantId) -> ArgoResult<Option<i64>> {
    sqlx::query_scalar(sql)
        .bind(tenant.to_string())
        .fetch_one(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant max: {e}")))
}

pub(super) async fn workers(
    store: &SessionStore,
    tenant: TenantId,
) -> ArgoResult<Vec<WorkerSummary>> {
    let rows = sqlx::query(
        "SELECT w.id, w.vertical_id, p.name AS persona, w.created_at \
         FROM worker w JOIN persona p ON p.id = w.persona_id \
         WHERE w.tenant_id = ? ORDER BY w.created_at DESC, w.id ASC LIMIT 25",
    )
    .bind(tenant.to_string())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("tenant workers: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let created_at = row.get("created_at");
            WorkerSummary {
                id: row.get("id"),
                vertical_id: row.get("vertical_id"),
                persona: row.get("persona"),
                created_at,
                created_at_rfc3339: format_micros(created_at),
            }
        })
        .collect())
}

pub(super) async fn sessions(
    store: &SessionStore,
    tenant: TenantId,
) -> ArgoResult<Vec<SessionSummary>> {
    let rows = sqlx::query(
        "SELECT id, title, model, provider, started_at FROM session \
         WHERE tenant_id = ? ORDER BY started_at DESC, id ASC LIMIT 25",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("tenant sessions: {e}")))?;
    rows.into_iter()
        .map(|row| {
            let id: Vec<u8> = row.get("id");
            let started_at = row.get("started_at");
            Ok(SessionSummary {
                id: uuid_from_blob(&id)?.to_string(),
                title: row.get("title"),
                model: row.get("model"),
                provider: row.get("provider"),
                started_at,
                started_at_rfc3339: format_micros(started_at),
            })
        })
        .collect()
}

fn uuid_from_blob(raw: &[u8]) -> ArgoResult<Uuid> {
    Uuid::from_slice(raw).map_err(|e| ArgoError::Provider(format!("tenant uuid blob: {e}")))
}

fn format_micros(micros: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp_micros(micros).map(|ts| ts.to_rfc3339())
}
