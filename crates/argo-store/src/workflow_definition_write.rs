//! Write-path for `workflow_definition` — `create` (insert new
//! version) + `set_head` (promote draft to active) + `archive`
//! (retire). Extracted from the parent so the file stays under the
//! 500-LOC ceiling (HARD RULE 6, brief:
//! `codex-loc-budget-followup-wave3`). Re-exported from
//! [`crate::workflow_definition`] so callers keep the same path.

use crate::workflow_definition::{WorkflowDefinitionId, WorkflowLifecycleState};
use argo_types::{ArgoError, ArgoResult, TenantId};
use sqlx::{Pool, Row, Sqlite};

/// Insert a new version row. Computes `MAX(version) + 1` per
/// `(tenant_id, vertical_id, name)` so callers do not have to pass
/// the next version themselves. The TOML body is parsed via
/// [`argo_vertical::parse_workflow`] *before* the DB insert;
/// malformed bodies surface as [`ArgoError::Schema`] and nothing is
/// written.
///
/// Newly-created rows land with `lifecycle_state = Draft`. Use
/// [`set_head`] separately to promote a draft to head.
pub async fn create(
    pool: &Pool<Sqlite>,
    tenant_id: TenantId,
    vertical_id: &str,
    name: &str,
    definition_toml: &str,
    parent_id: Option<WorkflowDefinitionId>,
    modified_by: &str,
) -> ArgoResult<WorkflowDefinitionId> {
    if vertical_id.is_empty() {
        return Err(ArgoError::Schema(
            "workflow_definition: vertical_id must not be empty".into(),
        ));
    }
    if name.is_empty() {
        return Err(ArgoError::Schema(
            "workflow_definition: name must not be empty".into(),
        ));
    }
    if modified_by.is_empty() {
        return Err(ArgoError::Schema(
            "workflow_definition: modified_by must not be empty".into(),
        ));
    }
    argo_vertical::parse_workflow(definition_toml)?;

    let tenant_str = tenant_id.to_string();
    let next_version: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(version), 0) + 1 FROM workflow_definition \
         WHERE tenant_id = ? AND vertical_id = ? AND name = ?",
    )
    .bind(&tenant_str)
    .bind(vertical_id)
    .bind(name)
    .fetch_one(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("workflow_definition next-version: {e}")))?;

    let id = WorkflowDefinitionId::new();
    let created_at = chrono::Utc::now().to_rfc3339();
    let parent_id_str: Option<String> = parent_id.as_ref().map(|p| p.0.clone());
    sqlx::query(
        "INSERT INTO workflow_definition \
            (id, tenant_id, vertical_id, name, definition_toml, version, \
             parent_id, lifecycle_state, created_at, modified_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.as_str())
    .bind(&tenant_str)
    .bind(vertical_id)
    .bind(name)
    .bind(definition_toml)
    .bind(next_version)
    .bind(parent_id_str)
    .bind(WorkflowLifecycleState::Draft.as_str())
    .bind(&created_at)
    .bind(modified_by)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("workflow_definition insert: {e}")))?;
    Ok(id)
}

/// Promote `id` to head for its `(tenant, vertical, name)` triple.
/// The brief specifies the triple in the signature so callers can
/// verify the row belongs to the namespace they think it does
/// before flipping the head. Mismatch (row exists but its
/// `(tenant, vertical, name)` differs) returns
/// [`ArgoError::Invariant`]; a missing id returns
/// [`ArgoError::NotFound`].
///
/// Also flips the row's `lifecycle_state` from `Draft` to `Active`
/// atomically inside the same transaction so the head + state stay
/// consistent.
pub async fn set_head(
    pool: &Pool<Sqlite>,
    tenant_id: TenantId,
    vertical_id: &str,
    name: &str,
    id: &WorkflowDefinitionId,
) -> ArgoResult<()> {
    let tenant_str = tenant_id.to_string();
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("workflow_definition set_head tx: {e}")))?;

    let row =
        sqlx::query("SELECT tenant_id, vertical_id, name FROM workflow_definition WHERE id = ?")
            .bind(id.as_str())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("workflow_definition set_head probe: {e}")))?;
    let row = row.ok_or_else(|| {
        ArgoError::NotFound(format!("workflow_definition: id '{}' does not exist", id.0))
    })?;
    let row_tenant: String = row
        .try_get("tenant_id")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode tenant_id: {e}")))?;
    let row_vertical: String = row
        .try_get("vertical_id")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode vertical_id: {e}")))?;
    let row_name: String = row
        .try_get("name")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode name: {e}")))?;
    if row_tenant != tenant_str || row_vertical != vertical_id || row_name != name {
        return Err(ArgoError::Invariant(format!(
            "workflow_definition set_head: id '{}' belongs to \
             ({row_tenant}, {row_vertical}, {row_name}), not ({tenant_str}, {vertical_id}, {name})",
            id.0
        )));
    }

    sqlx::query(
        "INSERT INTO workflow_definition_index (tenant_id, vertical_id, name, head_id) \
         VALUES (?, ?, ?, ?) \
         ON CONFLICT(tenant_id, vertical_id, name) DO UPDATE SET head_id = excluded.head_id",
    )
    .bind(&tenant_str)
    .bind(vertical_id)
    .bind(name)
    .bind(id.as_str())
    .execute(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("workflow_definition set_head write: {e}")))?;

    sqlx::query("UPDATE workflow_definition SET lifecycle_state = ? WHERE id = ?")
        .bind(WorkflowLifecycleState::Active.as_str())
        .bind(id.as_str())
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("workflow_definition set_head state: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("workflow_definition set_head commit: {e}")))?;
    Ok(())
}

/// Flip `lifecycle_state` to `Archived`. If the row was the head
/// for its `(tenant, vertical, name)` triple, the head pointer is
/// cleared in the same transaction so downstream loaders do not
/// resolve to an archived row. A missing id is a no-op (idempotent —
/// archive of an already-archived or never-existed id is fine).
pub async fn archive(pool: &Pool<Sqlite>, id: &WorkflowDefinitionId) -> ArgoResult<()> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("workflow_definition archive tx: {e}")))?;

    let probe =
        sqlx::query("SELECT tenant_id, vertical_id, name FROM workflow_definition WHERE id = ?")
            .bind(id.as_str())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("workflow_definition archive probe: {e}")))?;
    let Some(row) = probe else {
        // Idempotent: archive of a missing id is a no-op.
        tx.commit()
            .await
            .map_err(|e| ArgoError::Provider(format!("workflow_definition archive commit: {e}")))?;
        return Ok(());
    };
    let row_tenant: String = row
        .try_get("tenant_id")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode tenant_id: {e}")))?;
    let row_vertical: String = row
        .try_get("vertical_id")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode vertical_id: {e}")))?;
    let row_name: String = row
        .try_get("name")
        .map_err(|e| ArgoError::Provider(format!("workflow_definition decode name: {e}")))?;

    sqlx::query("UPDATE workflow_definition SET lifecycle_state = ? WHERE id = ?")
        .bind(WorkflowLifecycleState::Archived.as_str())
        .bind(id.as_str())
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("workflow_definition archive update: {e}")))?;

    sqlx::query(
        "DELETE FROM workflow_definition_index \
         WHERE tenant_id = ? AND vertical_id = ? AND name = ? AND head_id = ?",
    )
    .bind(&row_tenant)
    .bind(&row_vertical)
    .bind(&row_name)
    .bind(id.as_str())
    .execute(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("workflow_definition archive index: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("workflow_definition archive commit: {e}")))?;
    Ok(())
}

/// Rollback `(tenant, vertical, name)` to a prior version. Implemented
/// as an ADDITIVE write — produces a new version row whose TOML body
/// is byte-identical to the target row, then promotes the new row to
/// head. The original target row is left untouched (no history
/// rewrite). The new row's `parent_id` is the target row's id, so the
/// version chain reads naturally as "v5 was branched from v3."
///
/// # Errors
///
/// - [`ArgoError::NotFound`] — `target_id` does not exist, OR it
///   exists but belongs to a different `(tenant, vertical, name)`
///   triple (cross-tenant guard — must not leak existence).
/// - [`ArgoError::Invariant`] — the target row is already head
///   (rollback to current head is a no-op; callers should guard on
///   the gateway side too for a friendlier 4xx).
/// - [`ArgoError::Provider`] — sqlite errors.
///
/// Returns the new row's id + version number so the gateway can emit
/// them on the audit row.
pub async fn rollback_to(
    pool: &Pool<Sqlite>,
    tenant_id: TenantId,
    vertical_id: &str,
    name: &str,
    target_id: &WorkflowDefinitionId,
    modified_by: &str,
) -> ArgoResult<(WorkflowDefinitionId, i64)> {
    let tenant_str = tenant_id.to_string();
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("rollback_to tx: {e}")))?;

    let row = sqlx::query(
        "SELECT tenant_id, vertical_id, name, definition_toml \
         FROM workflow_definition WHERE id = ?",
    )
    .bind(target_id.as_str())
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("rollback_to probe: {e}")))?;
    let row = row.ok_or_else(|| {
        ArgoError::NotFound(format!(
            "workflow_definition: target id '{}' does not exist",
            target_id.0
        ))
    })?;
    let row_tenant: String = row
        .try_get("tenant_id")
        .map_err(|e| ArgoError::Provider(format!("rollback_to decode tenant: {e}")))?;
    let row_vertical: String = row
        .try_get("vertical_id")
        .map_err(|e| ArgoError::Provider(format!("rollback_to decode vertical: {e}")))?;
    let row_name: String = row
        .try_get("name")
        .map_err(|e| ArgoError::Provider(format!("rollback_to decode name: {e}")))?;
    let target_toml: String = row
        .try_get("definition_toml")
        .map_err(|e| ArgoError::Provider(format!("rollback_to decode toml: {e}")))?;
    if row_tenant != tenant_str || row_vertical != vertical_id || row_name != name {
        // Cross-tenant / cross-namespace guard: return NotFound (not
        // Invariant) so we never leak existence of the row.
        return Err(ArgoError::NotFound(format!(
            "workflow_definition: target id '{}' does not exist",
            target_id.0
        )));
    }

    let current_head: Option<String> = sqlx::query_scalar(
        "SELECT head_id FROM workflow_definition_index \
         WHERE tenant_id = ? AND vertical_id = ? AND name = ?",
    )
    .bind(&tenant_str)
    .bind(vertical_id)
    .bind(name)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("rollback_to head probe: {e}")))?;
    if current_head.as_deref() == Some(target_id.as_str()) {
        return Err(ArgoError::Invariant(
            "rollback_to: target id is already current head".into(),
        ));
    }

    let next_version: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(version), 0) + 1 FROM workflow_definition \
         WHERE tenant_id = ? AND vertical_id = ? AND name = ?",
    )
    .bind(&tenant_str)
    .bind(vertical_id)
    .bind(name)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("rollback_to next-version: {e}")))?;

    let new_id = WorkflowDefinitionId::new();
    let created_at = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO workflow_definition \
            (id, tenant_id, vertical_id, name, definition_toml, version, \
             parent_id, lifecycle_state, created_at, modified_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(new_id.as_str())
    .bind(&tenant_str)
    .bind(vertical_id)
    .bind(name)
    .bind(&target_toml)
    .bind(next_version)
    .bind(target_id.as_str())
    .bind(WorkflowLifecycleState::Active.as_str())
    .bind(&created_at)
    .bind(modified_by)
    .execute(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("rollback_to insert: {e}")))?;

    sqlx::query(
        "INSERT INTO workflow_definition_index (tenant_id, vertical_id, name, head_id) \
         VALUES (?, ?, ?, ?) \
         ON CONFLICT(tenant_id, vertical_id, name) DO UPDATE SET head_id = excluded.head_id",
    )
    .bind(&tenant_str)
    .bind(vertical_id)
    .bind(name)
    .bind(new_id.as_str())
    .execute(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("rollback_to head flip: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("rollback_to commit: {e}")))?;
    Ok((new_id, next_version))
}
