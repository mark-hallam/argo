//! Per-route handler bodies for `routes_workflows`. Split out from
//! the parent module to keep both files under the 500-LOC ceiling.
//! The dispatcher and auth helpers live in `routes_workflows.rs`;
//! this file owns the DB + audit work for every accepted route.
use super::routes_workflows_audit::{
    emit_audit_row, run_transactional_audit, workflow_audit_payload,
};
use super::routes_workflows_handlers_helpers::*;
use argo_audit::AuditKind;
use argo_store::workflow_definition::{
    self as wd, WorkflowDefinitionError, WorkflowDefinitionId,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, TenantId};
use serde_json::{json, Value};
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
            if def.tenant_id == tenant && def.vertical_id == vertical_id && def.name == name =>
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
        Err(e) => return super::routes_workflows::internal(&format!("get_head: {e}")),
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
        Err(WorkflowDefinitionError::UniqueViolation { vertical, name }) => {
            collision_conflict_response(&vertical, &name)
        }
        Err(WorkflowDefinitionError::Argo(ArgoError::Schema(msg))) => {
            super::routes_workflows::bad_request(&format!("parse workflow toml: {msg}"))
        }
        Err(WorkflowDefinitionError::Argo(e)) => {
            super::routes_workflows::internal(&format!("create workflow version: {e}"))
        }
    }
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
    let id_for_closure = id.clone();
    let vertical_id_owned = vertical_id.to_string();
    let name_owned = name.to_string();
    let actor_owned = actor.to_string();
    let outcome = run_transactional_audit(store, tenant, move |tx, sink| {
        Box::pin(async move {
            wd::set_head_in_tx(tx, tenant, &vertical_id_owned, &name_owned, &id_for_closure)
                .await?;
            sink.write_in_tx(
                tx,
                None,
                AuditKind::WorkflowHeadChanged,
                workflow_audit_payload(
                    tenant,
                    &vertical_id_owned,
                    &name_owned,
                    &id_for_closure.0,
                    &actor_owned,
                ),
                Some(actor_owned),
            )
            .await
        })
    })
    .await;
    match outcome {
        Ok(()) => ("204 No Content", String::new()),
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
    // Pre-check outside the tx so we can return a friendlier 404 with
    // no audit row written. The transactional path below assumes the
    // row exists in the expected namespace; in-tx archive of a missing
    // id is a no-op at the store layer, which would otherwise emit an
    // audit row for a non-event.
    match wd::get(store.pool(), &id).await {
        Ok(Some(def))
            if def.tenant_id == tenant && def.vertical_id == vertical_id && def.name == name => {}
        Ok(_) => {
            return super::routes_workflows::not_found(&format!(
                "no workflow definition version '{version_id}' for ({vertical_id}, {name})"
            ))
        }
        Err(e) => return super::routes_workflows::internal(&format!("archive lookup: {e}")),
    }
    let id_for_closure = id.clone();
    let vertical_id_owned = vertical_id.to_string();
    let name_owned = name.to_string();
    let version_id_owned = version_id.to_string();
    let actor_owned = actor.to_string();
    let outcome = run_transactional_audit(store, tenant, move |tx, sink| {
        Box::pin(async move {
            wd::archive_in_tx(tx, &id_for_closure).await?;
            sink.write_in_tx(
                tx,
                None,
                AuditKind::WorkflowVersionArchived,
                workflow_audit_payload(
                    tenant,
                    &vertical_id_owned,
                    &name_owned,
                    &version_id_owned,
                    &actor_owned,
                ),
                Some(actor_owned),
            )
            .await
        })
    })
    .await;
    match outcome {
        Ok(()) => ("204 No Content", String::new()),
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
    let rollback_result: Arc<std::sync::Mutex<Option<(WorkflowDefinitionId, i64)>>> =
        Arc::new(std::sync::Mutex::new(None));
    let rollback_result_inner = Arc::clone(&rollback_result);
    let target_id_for_closure = target_id.clone();
    let target_id_str_owned = target_id_str.clone();
    let vertical_id_owned = vertical_id.to_string();
    let name_owned = name.to_string();
    let actor_owned = actor.to_string();
    let outcome = run_transactional_audit(store, tenant, move |tx, sink| {
        Box::pin(async move {
            let (new_id, new_version) = wd::rollback_to_in_tx(
                tx,
                tenant,
                &vertical_id_owned,
                &name_owned,
                &target_id_for_closure,
                &actor_owned,
            )
            .await?;
            let payload = json!({
                "actor": actor_owned,
                "tenant_id": tenant.to_string(),
                "vertical_id": vertical_id_owned,
                "name": name_owned,
                "rolled_back_from_version_id": target_id_str_owned,
                "new_version_id": new_id.0,
                "new_version_number": new_version,
            });
            let post = sink
                .write_in_tx(
                    tx,
                    None,
                    AuditKind::WorkflowRolledBack,
                    payload,
                    Some(actor_owned),
                )
                .await?;
            if let Ok(mut guard) = rollback_result_inner.lock() {
                *guard = Some((new_id, new_version));
            }
            Ok(post)
        })
    })
    .await;
    match outcome {
        Ok(()) => {
            // outcome=Ok implies the closure ran to completion and
            // populated `rollback_result` before the tx committed.
            let result = rollback_result.lock().ok().and_then(|g| g.clone());
            let (new_id, new_version) = match result {
                Some(r) => r,
                None => {
                    return super::routes_workflows::internal(
                        "rollback_to: result missing after commit",
                    )
                }
            };
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
