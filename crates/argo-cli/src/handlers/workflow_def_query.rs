//! Direct-SQL helpers for `argo workflow def list` (Stream
//! `mission-pm-workflow-rollback-cli`).
//!
//! The argo-store CRUD surface (locked by the brief) exposes
//! `get_head` for a single `(tenant, vertical, name)` triple and
//! `list_versions` per name; it does not expose a "list all workflow
//! heads under a vertical" helper. The gateway route
//! `routes_workflows_handlers::list_workflows` solves the same gap by
//! writing the join inline — this CLI handler mirrors that precedent
//! rather than widening the store surface.
use argo_store::workflow_definition::{
    WorkflowDefinition, WorkflowDefinitionId, WorkflowLifecycleState,
};
use argo_types::{ArgoError, ArgoResult, TenantId};
use sqlx::Row;

/// Fetch every workflow definition whose `(tenant, vertical_id)`
/// triple has an active head pointer in `workflow_definition_index`.
/// Returned rows are decoded via [`decode_row`] and sorted by name
/// ascending.
pub async fn list_workflow_heads(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    tenant: TenantId,
    vertical_id: &str,
) -> ArgoResult<Vec<WorkflowDefinition>> {
    let tenant_str = tenant.to_string();
    let rows = sqlx::query(
        "SELECT wd.id, wd.tenant_id, wd.vertical_id, wd.name, wd.definition_toml, \
                wd.version, wd.parent_id, wd.lifecycle_state, wd.created_at, \
                wd.modified_by \
         FROM workflow_definition_index idx \
         JOIN workflow_definition wd ON wd.id = idx.head_id \
         WHERE idx.tenant_id = ? AND idx.vertical_id = ? \
         ORDER BY wd.name",
    )
    .bind(&tenant_str)
    .bind(vertical_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("workflow def list: {e}")))?;
    rows.into_iter().map(decode_row).collect()
}

/// Decode a `workflow_definition` row produced by either the
/// inline-join query above or the per-id read paths. Kept here so the
/// CLI does not duplicate the column-by-column unpacking from the
/// gateway side.
pub fn decode_row(row: sqlx::sqlite::SqliteRow) -> ArgoResult<WorkflowDefinition> {
    let id: String = row
        .try_get("id")
        .map_err(|e| ArgoError::Provider(format!("decode id: {e}")))?;
    let tenant_str: String = row
        .try_get("tenant_id")
        .map_err(|e| ArgoError::Provider(format!("decode tenant_id: {e}")))?;
    let tenant = TenantId(
        uuid::Uuid::parse_str(&tenant_str)
            .map_err(|e| ArgoError::Provider(format!("decode tenant uuid: {e}")))?,
    );
    let vertical_id: String = row
        .try_get("vertical_id")
        .map_err(|e| ArgoError::Provider(format!("decode vertical_id: {e}")))?;
    let name: String = row
        .try_get("name")
        .map_err(|e| ArgoError::Provider(format!("decode name: {e}")))?;
    let definition_toml: String = row
        .try_get("definition_toml")
        .map_err(|e| ArgoError::Provider(format!("decode definition_toml: {e}")))?;
    let version: i64 = row
        .try_get("version")
        .map_err(|e| ArgoError::Provider(format!("decode version: {e}")))?;
    let parent_id: Option<String> = row
        .try_get::<Option<String>, _>("parent_id")
        .map_err(|e| ArgoError::Provider(format!("decode parent_id: {e}")))?
        .filter(|s| !s.is_empty());
    let lifecycle_state_str: String = row
        .try_get("lifecycle_state")
        .map_err(|e| ArgoError::Provider(format!("decode lifecycle_state: {e}")))?;
    let created_at: String = row
        .try_get("created_at")
        .map_err(|e| ArgoError::Provider(format!("decode created_at: {e}")))?;
    let modified_by: String = row
        .try_get("modified_by")
        .map_err(|e| ArgoError::Provider(format!("decode modified_by: {e}")))?;
    Ok(WorkflowDefinition {
        id: WorkflowDefinitionId(id),
        tenant_id: tenant,
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
