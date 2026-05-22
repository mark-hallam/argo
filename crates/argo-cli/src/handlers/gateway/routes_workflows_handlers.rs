//! Per-route handler bodies for `routes_workflows`. Split out from
//! the parent module to keep both files under the 500-LOC ceiling.
//! The dispatcher and auth helpers live in `routes_workflows.rs`;
//! this file owns the DB + audit work for every accepted route.
use argo_audit::{AuditKind, AuditSink};
use argo_store::workflow_definition::{
    self as wd, WorkflowDefinition, WorkflowDefinitionId, WorkflowLifecycleState,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;

// ── GET handlers ────────────────────────────────────────────────

pub(super) async fn list_workflows(
    store: &SessionStore,
    tenant: TenantId,
    vertical_id: &str,
) -> (&'static str, String) {
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
    .fetch_all(store.pool())
    .await;
    let rows = match rows {
        Ok(r) => r,
        Err(e) => return super::routes_workflows::internal(&format!("list workflows: {e}")),
    };
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        match decode_index_row(row) {
            Ok(def) => items.push(definition_json(&def)),
            Err(e) => {
                return super::routes_workflows::internal(&format!("decode workflow row: {e}"))
            }
        }
    }
    let body = json!({
        "vertical_id": vertical_id,
        "count": items.len(),
        "items": items,
    });
    ("200 OK", body.to_string())
}

pub(super) async fn fetch_head(
    store: &SessionStore,
    tenant: TenantId,
    vertical_id: &str,
    name: &str,
) -> (&'static str, String) {
    match wd::get_head(store.pool(), tenant, vertical_id, name).await {
        Ok(Some(def)) => ("200 OK", definition_json(&def).to_string()),
        Ok(None) => super::routes_workflows::not_found(&format!(
            "no head workflow definition for ({vertical_id}, {name})"
        )),
        Err(e) => super::routes_workflows::internal(&format!("get_head: {e}")),
    }
}

pub(super) async fn list_versions(
    store: &SessionStore,
    tenant: TenantId,
    vertical_id: &str,
    name: &str,
) -> (&'static str, String) {
    match wd::list_versions(store.pool(), tenant, vertical_id, name).await {
        Ok(rows) if rows.is_empty() => super::routes_workflows::not_found(&format!(
            "no workflow definition versions for ({vertical_id}, {name})"
        )),
        Ok(rows) => {
            let items: Vec<Value> = rows.iter().map(definition_json).collect();
            let body = json!({
                "vertical_id": vertical_id,
                "name": name,
                "count": items.len(),
                "items": items,
            });
            ("200 OK", body.to_string())
        }
        Err(e) => super::routes_workflows::internal(&format!("list_versions: {e}")),
    }
}

pub(super) async fn fetch_version(
    store: &SessionStore,
    tenant: TenantId,
    vertical_id: &str,
    name: &str,
    version_id: &str,
) -> (&'static str, String) {
    let id = WorkflowDefinitionId(version_id.to_string());
    match wd::get(store.pool(), &id).await {
        Ok(Some(def))
            if def.tenant_id == tenant
                && def.vertical_id == vertical_id
                && def.name == name =>
        {
            ("200 OK", definition_json(&def).to_string())
        }
        Ok(_) => super::routes_workflows::not_found(&format!(
            "no workflow definition version '{version_id}' for ({vertical_id}, {name})"
        )),
        Err(e) => super::routes_workflows::internal(&format!("get version: {e}")),
    }
}

// ── Mutating handlers ────────────────────────────────────────────

pub(super) async fn create_version(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    vertical_id: &str,
    name: &str,
    body: &str,
    actor: &str,
    if_version: Option<i64>,
) -> (&'static str, String) {
    if body.trim().is_empty() {
        return super::routes_workflows::bad_request("request body must contain workflow TOML");
    }
    // Optimistic-concurrency gate: when the caller asserts a known
    // version via `X-Argo-If-Version`, we refuse the write if the
    // current head has advanced past that number. The 409 body echoes
    // the live head so the SPA can offer a refresh / show-diff banner
    // without re-fetching the workflow.
    let current_head = match wd::get_head(store.pool(), tenant, vertical_id, name).await {
        Ok(opt) => opt,
        Err(e) => {
            return super::routes_workflows::internal(&format!("get_head: {e}"))
        }
    };
    if let Some(expected) = if_version {
        let current_version = current_head.as_ref().map(|d| d.version).unwrap_or(0);
        if current_version != expected {
            return conflict_response(&current_head, current_version);
        }
    }
    let parent_id = current_head.as_ref().map(|def| def.id.clone());
    match wd::create(
        store.pool(),
        tenant,
        vertical_id,
        name,
        body,
        parent_id,
        actor,
    )
    .await
    {
        Ok(id) => {
            emit_audit_row(
                store,
                tenant,
                AuditKind::WorkflowDefinitionCreated,
                vertical_id,
                name,
                &id.0,
                actor,
            )
            .await;
            let body = json!({
                "vertical_id": vertical_id,
                "name": name,
                "version_id": id.0,
            });
            ("201 Created", body.to_string())
        }
        Err(ArgoError::Schema(msg)) => {
            super::routes_workflows::bad_request(&format!("parse workflow toml: {msg}"))
        }
        Err(e) => super::routes_workflows::internal(&format!("create workflow version: {e}")),
    }
}

fn conflict_response(
    current_head: &Option<WorkflowDefinition>,
    current_version: i64,
) -> (&'static str, String) {
    let current_head_definition_toml = current_head
        .as_ref()
        .map(|d| d.definition_toml.clone())
        .unwrap_or_default();
    let body = json!({
        "kind": "conflict",
        "current_version": current_version,
        "current_head_definition_toml": current_head_definition_toml,
    });
    ("409 Conflict", body.to_string())
}

pub(super) async fn set_head(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    vertical_id: &str,
    name: &str,
    body: &str,
    actor: &str,
) -> (&'static str, String) {
    let parsed: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return super::routes_workflows::bad_request(&format!("parse json body: {e}")),
    };
    let version_id = match parsed.get("version_id").and_then(Value::as_str) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => {
            return super::routes_workflows::bad_request(
                "body must contain a non-empty `version_id` string",
            )
        }
    };
    let id = WorkflowDefinitionId(version_id);
    match wd::set_head(store.pool(), tenant, vertical_id, name, &id).await {
        Ok(()) => {
            emit_audit_row(
                store,
                tenant,
                AuditKind::WorkflowHeadChanged,
                vertical_id,
                name,
                &id.0,
                actor,
            )
            .await;
            ("204 No Content", String::new())
        }
        Err(ArgoError::NotFound(_) | ArgoError::Invariant(_)) => {
            super::routes_workflows::not_found(&format!(
                "no workflow definition version '{}' for ({vertical_id}, {name})",
                id.0
            ))
        }
        Err(e) => super::routes_workflows::internal(&format!("set head: {e}")),
    }
}

pub(super) async fn archive_version(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    vertical_id: &str,
    name: &str,
    version_id: &str,
    actor: &str,
) -> (&'static str, String) {
    let id = WorkflowDefinitionId(version_id.to_string());
    match wd::get(store.pool(), &id).await {
        Ok(Some(def))
            if def.tenant_id == tenant
                && def.vertical_id == vertical_id
                && def.name == name => {}
        Ok(_) => {
            return super::routes_workflows::not_found(&format!(
                "no workflow definition version '{version_id}' for ({vertical_id}, {name})"
            ))
        }
        Err(e) => return super::routes_workflows::internal(&format!("archive lookup: {e}")),
    }
    match wd::archive(store.pool(), &id).await {
        Ok(()) => {
            emit_audit_row(
                store,
                tenant,
                AuditKind::WorkflowVersionArchived,
                vertical_id,
                name,
                version_id,
                actor,
            )
            .await;
            ("204 No Content", String::new())
        }
        Err(e) => super::routes_workflows::internal(&format!("archive version: {e}")),
    }
}

pub(super) async fn rollback(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    vertical_id: &str,
    name: &str,
    body: &str,
    actor: &str,
) -> (&'static str, String) {
    let parsed: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return super::routes_workflows::bad_request(&format!("parse json body: {e}")),
    };
    let target_id_str = match parsed.get("target_version_id").and_then(Value::as_str) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => {
            return super::routes_workflows::bad_request(
                "body must contain a non-empty `target_version_id` string",
            )
        }
    };
    let target_id = WorkflowDefinitionId(target_id_str.clone());
    match wd::rollback_to(store.pool(), tenant, vertical_id, name, &target_id, actor).await {
        Ok((new_id, new_version)) => {
            let payload = json!({
                "actor": actor,
                "tenant_id": tenant.to_string(),
                "vertical_id": vertical_id,
                "name": name,
                "rolled_back_from_version_id": target_id_str,
                "new_version_id": new_id.0,
                "new_version_number": new_version,
            });
            let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
            if let Err(e) = sink
                .write(None, AuditKind::WorkflowRolledBack, payload, Some(actor.to_string()))
                .await
            {
                tracing::warn!(
                    target: "gateway::routes_workflows",
                    error = %e,
                    kind = %AuditKind::WorkflowRolledBack.label(),
                    vertical_id = %vertical_id,
                    name = %name,
                    "workflow rollback audit write failed",
                );
            }
            let body = json!({
                "id": new_id.0,
                "version": new_version,
            });
            ("200 OK", body.to_string())
        }
        Err(ArgoError::NotFound(msg)) => super::routes_workflows::not_found(&msg),
        Err(ArgoError::Invariant(msg)) => super::routes_workflows::bad_request(&msg),
        Err(e) => super::routes_workflows::internal(&format!("rollback_to: {e}")),
    }
}

// ── Audit + JSON helpers ────────────────────────────────────────

async fn emit_audit_row(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    kind: AuditKind,
    vertical_id: &str,
    name: &str,
    version_id: &str,
    actor: &str,
) {
    let payload = json!({
        "actor": actor,
        "tenant_id": tenant.to_string(),
        "vertical_id": vertical_id,
        "name": name,
        "version_id": version_id,
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    if let Err(e) = sink
        .write(None, kind, payload, Some(actor.to_string()))
        .await
    {
        tracing::warn!(
            target: "gateway::routes_workflows",
            error = %e,
            kind = %kind.label(),
            vertical_id = %vertical_id,
            name = %name,
            "workflow audit write failed",
        );
    }
}

fn definition_json(def: &WorkflowDefinition) -> Value {
    json!({
        "id": def.id.0,
        "tenant_id": def.tenant_id.to_string(),
        "vertical_id": def.vertical_id,
        "name": def.name,
        "version": def.version,
        "parent_id": def.parent_id.as_ref().map(|p| p.0.clone()),
        "lifecycle_state": def.lifecycle_state.as_str(),
        "created_at": def.created_at,
        "modified_by": def.modified_by,
        "definition_toml": def.definition_toml,
    })
}

fn decode_index_row(row: sqlx::sqlite::SqliteRow) -> ArgoResult<WorkflowDefinition> {
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
