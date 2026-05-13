//! Danger-zone halves of the J11 worker-settings surface:
//! `POST /pause`, `POST /resume`, `POST /archive`, and `DELETE /`.
//! See the parent module for the v1-posture rationale.

use super::super::routes_worker_detail::PrincipalRole;
use super::helpers::{
    emit_audit, error_envelope, fetch_archived_at, forbidden, header_value, load_worker_by_id,
    parse_role_header, parse_worker_id, preflight, resolve_tenant, server_error, store_unavail,
    CONFIRM_HEADER, RECOVERABLE_DAYS,
};
use argo_identity::{IdentityStore, WorkerId};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{Duration, Utc};
use serde_json::json;
use std::sync::Arc;

pub(crate) async fn post_pause(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else { return store_unavail() };
    let (_, worker) = match preflight(worker_id_raw, store, true).await {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    emit_audit(store, req, &worker, "worker.paused", json!({})).await;
    ("200 OK", json!({ "status": "paused" }).to_string())
}

pub(crate) async fn post_resume(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else { return store_unavail() };
    let worker_id = match parse_worker_id(worker_id_raw) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let tenant = match resolve_tenant(store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let worker = match load_worker_by_id(store, tenant, worker_id).await {
        Ok(w) => w,
        Err(resp) => return resp,
    };
    if fetch_archived_at(store, tenant, worker_id)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        if let Err(e) = clear_archived_at(store, tenant, worker_id).await {
            return server_error(&format!("clear archive: {e}"));
        }
        emit_audit(store, req, &worker, "worker.recovered", json!({})).await;
        return (
            "200 OK",
            json!({ "status": "idle", "recovered": true }).to_string(),
        );
    }
    emit_audit(store, req, &worker, "worker.resumed", json!({})).await;
    ("200 OK", json!({ "status": "idle" }).to_string())
}

pub(crate) async fn post_archive(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else { return store_unavail() };
    let (tenant, worker) = match preflight(worker_id_raw, store, true).await {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    if let Err(e) = IdentityStore::new(store).archive_worker(tenant, worker.id).await {
        return server_error(&format!("archive: {e}"));
    }
    emit_audit(store, req, &worker, "worker.archived", json!({})).await;
    let archived_at = Utc::now();
    let recoverable = archived_at + Duration::days(RECOVERABLE_DAYS);
    let body = json!({
        "archived_at": archived_at.to_rfc3339(),
        "recoverable_until_at": recoverable.to_rfc3339(),
    });
    ("200 OK", body.to_string())
}

pub(crate) async fn delete_worker_endpoint(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else { return store_unavail() };
    let worker_id = match parse_worker_id(worker_id_raw) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let tenant = match resolve_tenant(store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let worker = match load_worker_by_id(store, tenant, worker_id).await {
        Ok(w) => w,
        Err(resp) => return resp,
    };
    let role = parse_role_header(req).unwrap_or(PrincipalRole::Delegator);
    if role != PrincipalRole::Operator {
        return forbidden("delete requires operator role");
    }
    let confirm = header_value(req, CONFIRM_HEADER).unwrap_or_default();
    if confirm != worker.persona.name {
        return error_envelope(
            "400 Bad Request",
            "confirmation_mismatch",
            "Worker name confirmation didn't match",
        );
    }
    if let Err(e) = hard_delete_worker(store, tenant, worker_id).await {
        return server_error(&format!("delete: {e}"));
    }
    emit_audit(store, req, &worker, "worker.deleted", json!({})).await;
    ("204 No Content", String::new())
}

async fn clear_archived_at(
    store: &SessionStore,
    tenant: TenantId,
    worker_id: WorkerId,
) -> ArgoResult<()> {
    sqlx::query("UPDATE worker SET archived_at = NULL WHERE id = ? AND tenant_id = ?")
        .bind(worker_id.to_string())
        .bind(tenant.to_string())
        .execute(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("clear archive: {e}")))?;
    Ok(())
}

async fn hard_delete_worker(
    store: &SessionStore,
    tenant: TenantId,
    worker_id: WorkerId,
) -> ArgoResult<()> {
    let worker_blob = worker_id.0.as_bytes().to_vec();
    let tenant_blob = tenant.0.as_bytes().to_vec();
    let mut tx = store
        .pool()
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("delete tx: {e}")))?;
    sqlx::query("DELETE FROM memory_entry WHERE worker_id = ? AND tenant_id = ?")
        .bind(&worker_blob)
        .bind(&tenant_blob)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("delete memory_entry: {e}")))?;
    sqlx::query("DELETE FROM duty WHERE worker_id = ?")
        .bind(worker_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("delete duty: {e}")))?;
    sqlx::query("DELETE FROM role WHERE worker_id = ?")
        .bind(worker_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("delete role: {e}")))?;
    sqlx::query("DELETE FROM persona WHERE worker_id = ?")
        .bind(worker_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("delete persona: {e}")))?;
    sqlx::query("DELETE FROM worker WHERE id = ? AND tenant_id = ?")
        .bind(worker_id.to_string())
        .bind(tenant.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("delete worker: {e}")))?;
    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("delete commit: {e}")))?;
    Ok(())
}
