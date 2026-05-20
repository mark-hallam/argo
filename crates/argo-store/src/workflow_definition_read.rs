//! Read-path for `workflow_definition` — `get` / `get_head` /
//! `list_versions` plus the shared row decoder. Extracted from the
//! parent so the file stays under the 500-LOC ceiling (HARD RULE 6,
//! brief: `codex-loc-budget-followup-wave3`). Re-exported from
//! [`crate::workflow_definition`] so callers keep the same path.

use crate::workflow_definition::{
    WorkflowDefinition, WorkflowDefinitionId, WorkflowLifecycleState,
};
use argo_types::{ArgoError, ArgoResult, TenantId};
use sqlx::{Pool, Row, Sqlite};
use uuid::Uuid;

/// Fetch one row by id; `None` when the row does not exist.
pub async fn get(
    pool: &Pool<Sqlite>,
    id: &WorkflowDefinitionId,
) -> ArgoResult<Option<WorkflowDefinition>> {
    let row = sqlx::query(
        "SELECT id, tenant_id, vertical_id, name, definition_toml, version, \
                parent_id, lifecycle_state, created_at, modified_by \
         FROM workflow_definition WHERE id = ?",
    )
    .bind(id.as_str())
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("workflow_definition get: {e}")))?;
    row.map(decode_row).transpose()
}

/// Fetch the currently-active row for `(tenant, vertical, name)`,
/// joining through `workflow_definition_index`. Returns `None` when
/// no head is set or the head row has been deleted underneath it
/// (the latter is defensive — `archive` clears the head atomically).
pub async fn get_head(
    pool: &Pool<Sqlite>,
    tenant_id: TenantId,
    vertical_id: &str,
    name: &str,
) -> ArgoResult<Option<WorkflowDefinition>> {
    let tenant_str = tenant_id.to_string();
    let row = sqlx::query(
        "SELECT wd.id, wd.tenant_id, wd.vertical_id, wd.name, wd.definition_toml, \
                wd.version, wd.parent_id, wd.lifecycle_state, wd.created_at, \
                wd.modified_by \
         FROM workflow_definition_index idx \
         JOIN workflow_definition wd ON wd.id = idx.head_id \
         WHERE idx.tenant_id = ? AND idx.vertical_id = ? AND idx.name = ?",
    )
    .bind(&tenant_str)
    .bind(vertical_id)
    .bind(name)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("workflow_definition get_head: {e}")))?;
    row.map(decode_row).transpose()
}

/// Every version row for `(tenant, vertical, name)`, newest first
/// (ORDER BY version DESC). Empty `Vec` when no rows exist.
pub async fn list_versions(
    pool: &Pool<Sqlite>,
    tenant_id: TenantId,
    vertical_id: &str,
    name: &str,
) -> ArgoResult<Vec<WorkflowDefinition>> {
    let tenant_str = tenant_id.to_string();
    let rows = sqlx::query(
        "SELECT id, tenant_id, vertical_id, name, definition_toml, version, \
                parent_id, lifecycle_state, created_at, modified_by \
         FROM workflow_definition \
         WHERE tenant_id = ? AND vertical_id = ? AND name = ? \
         ORDER BY version DESC",
    )
    .bind(&tenant_str)
    .bind(vertical_id)
    .bind(name)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("workflow_definition list_versions: {e}")))?;
    rows.into_iter().map(decode_row).collect()
}

fn decode_row(row: sqlx::sqlite::SqliteRow) -> ArgoResult<WorkflowDefinition> {
    let id: String = row
        .try_get("id")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode id: {e}")))?;
    let tenant_str: String = row
        .try_get("tenant_id")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode tenant_id: {e}")))?;
    let tenant_uuid = Uuid::parse_str(&tenant_str).map_err(|e| {
        ArgoError::Provider(format!(
            "workflow_definition decode tenant_id '{tenant_str}': {e}"
        ))
    })?;
    let vertical_id: String = row
        .try_get("vertical_id")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode vertical_id: {e}")))?;
    let name: String = row
        .try_get("name")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode name: {e}")))?;
    let definition_toml: String = row.try_get("definition_toml").map_err(|e| {
        ArgoError::Provider(format!("workflow_definition decode definition_toml: {e}"))
    })?;
    let version: i64 = row
        .try_get("version")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode version: {e}")))?;
    let parent_id: Option<String> = row
        .try_get::<Option<String>, _>("parent_id")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode parent_id: {e}")))?
        .filter(|s| !s.is_empty());
    let lifecycle_state_str: String = row.try_get("lifecycle_state").map_err(|e| {
        ArgoError::Provider(format!("workflow_definition decode lifecycle_state: {e}"))
    })?;
    let created_at: String = row
        .try_get("created_at")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode created_at: {e}")))?;
    let modified_by: String = row
        .try_get("modified_by")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode modified_by: {e}")))?;
    Ok(WorkflowDefinition {
        id: WorkflowDefinitionId(id),
        tenant_id: TenantId(tenant_uuid),
        vertical_id,
        name,
        definition_toml,
        version,
        parent_id: parent_id.map(WorkflowDefinitionId),
        lifecycle_state: WorkflowLifecycleState::parse(&lifecycle_state_str)?,
        created_at,
        modified_by,
    })
}
