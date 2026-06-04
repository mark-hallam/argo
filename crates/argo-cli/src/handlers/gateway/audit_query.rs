//! SQL helpers shared by the audit feed + detail handlers.
//!
//! Resolves `worker_id` → display name by joining the `worker` table
//! against `persona` (which carries the operator-friendly `name`
//! field). The audit payload's embedded `worker_name` is unreliable
//! after a worker rename and was previously back-filled with the
//! UUID itself on un-stamped rows; the join here is the source of
//! truth for the camera-readable display name.
//!
//! Tenant-bound on `worker.tenant_id` so a same-id worker in another
//! tenant cannot leak its display name into the response — the
//! lookup map is keyed by `(tenant_id, worker_id)` and callers fall
//! back to the UUID when the pair is absent.
//!
//! Also hosts the **shared audit tenant resolver** used by both
//! `routes_audit_feed::list_response` and
//! `routes_audit_detail::detail_response`. The two routes used to
//! diverge on the un-headered fallback (list pulled the cwd-bound
//! workspace UUID via `SessionStore::ensure_workspace`; detail fell
//! through `rate_limit::extract_tenant` to the `argo_config` default
//! → `"anonymous"`), so a curl without `X-Argo-Tenant-Id` would see
//! rows in the list that detail then 404'd on. See the drift incident
//! behind `fix-audit-detail-tenant-scope-list-detail-sql-mismatch`.

use super::rate_limit;
use argo_audit::ExportedRow;
use argo_store::SessionStore;
use serde_json::Value;
use sqlx::Row;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Resolve the default audit-scope tenant for the gateway, used when
/// the request omits an explicit `?as_tenant=` cross-tenant signal.
///
/// Sprint WS1: now JWT-aware via [`super::resolve_tenant::
/// resolve_tenant_for_request`] — a signed-in user's audit feed
/// defaults to THEIR own tenant, not the cwd demo tenant. The
/// `X-Argo-Tenant-Id` header override + capability-gated `?as_tenant=`
/// cross-tenant path in [`super::routes_audit_feed_scope::
/// resolve_tenant_scope`] are unchanged (it still treats this value as
/// the `workspace_default` it layers the header chain on top of).
/// Returns the empty string on any failure, in which case callers fall
/// back to whatever sentinel is appropriate for their route.
pub(super) async fn resolve_workspace_default(req: &str, store: &Arc<SessionStore>) -> String {
    super::resolve_tenant::resolve_tenant_for_request(req, store)
        .await
        .map(|t| t.to_string())
        .unwrap_or_default()
}

/// Resolve the audit caller's tenant for both list and detail routes.
///
/// Precedence: any **explicit** request signal (`?tenant=` query
/// param, `X-Tenant-Id` header, `X-Argo-Tenant-Id` header) wins; if
/// the request carries none, fall back to the cwd-bound workspace
/// tenant UUID; finally defer to [`rate_limit::extract_tenant`] for
/// the config-default / anonymous tail.
///
/// Layering the workspace-default fallback **between** the explicit
/// signals and the config tail is the load-bearing fix: writers stamp
/// rows under the workspace UUID, so without this layer an
/// un-headered detail request resolves to `"anonymous"` and 404s on
/// rows the un-headered list returns under the same workspace UUID.
pub(super) async fn resolve_audit_tenant(store: &Arc<SessionStore>, req: &str) -> String {
    if let Some(explicit) = explicit_tenant_from_request(req) {
        return explicit;
    }
    let workspace_default = resolve_workspace_default(req, store).await;
    if !workspace_default.is_empty() {
        return workspace_default;
    }
    rate_limit::extract_tenant(req)
}

/// Returns the tenant id when the request carries an **explicit**
/// signal (`?tenant=`, `X-Tenant-Id`, `X-Argo-Tenant-Id`); `None`
/// when the request is un-headered so callers can layer their own
/// fallback. Mirrors [`rate_limit::extract_tenant`]'s explicit chain
/// minus the config-default / anonymous tail.
fn explicit_tenant_from_request(req: &str) -> Option<String> {
    if let Some(target) = request_target(req) {
        if let Some((_, qs)) = target.split_once('?') {
            for kv in qs.split('&') {
                if let Some(("tenant", value)) = kv.split_once('=') {
                    if !value.is_empty() {
                        return Some(value.to_string());
                    }
                }
            }
        }
    }
    for name in ["x-tenant-id", "x-argo-tenant-id"] {
        if let Some(value) = audit_header(req, name) {
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn request_target(req: &str) -> Option<&str> {
    req.lines().next()?.split_whitespace().nth(1)
}

fn audit_header(req: &str, key: &str) -> Option<String> {
    let header_end = req.find("\r\n\r\n").unwrap_or(req.len());
    for line in req[..header_end].split("\r\n").skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case(key) {
            return Some(value.trim().to_string());
        }
    }
    None
}

/// Map type returned by [`resolve_worker_names`].
pub(super) type WorkerNameMap = HashMap<(String, String), String>;

/// Look up a worker's display name from the resolved map; falls back
/// to the worker UUID when the join missed (deleted / never existed)
/// and to the literal `"unknown"` when the audit payload didn't even
/// carry a worker id.
pub(super) fn lookup_worker_name(
    names: &WorkerNameMap,
    tenant_id: &str,
    worker_id: &str,
) -> String {
    names
        .get(&(tenant_id.to_string(), worker_id.to_string()))
        .cloned()
        .unwrap_or_else(|| {
            if worker_id.is_empty() {
                "unknown".into()
            } else {
                worker_id.to_string()
            }
        })
}

/// Convenience for the audit-list path: walk a page of `ExportedRow`s,
/// pluck `(tenant_id, worker_id)` pairs out of each payload, and
/// batch-resolve to display names.
pub(super) async fn resolve_page_worker_names(
    store: &SessionStore,
    page: &[&ExportedRow],
) -> Result<WorkerNameMap, sqlx::Error> {
    let pairs: Vec<(String, String)> = page
        .iter()
        .filter_map(|row| {
            Some((
                payload_string(&row.payload, "tenant_id")?,
                payload_string(&row.payload, "worker_id")?,
            ))
        })
        .collect();
    resolve_worker_names(store, &pairs).await
}

fn payload_string(p: &Value, key: &str) -> Option<String> {
    p.get(key).and_then(Value::as_str).map(str::to_string)
}

/// Resolve `(tenant_id, worker_id) → persona.name` for a batch of
/// audit rows. Empty input returns an empty map without touching the
/// database. The query groups by tenant so we issue one statement per
/// tenant rather than per row; the audit feed normally targets a
/// single tenant so this collapses to one statement in practice.
pub(super) async fn resolve_worker_names(
    store: &SessionStore,
    pairs: &[(String, String)],
) -> Result<HashMap<(String, String), String>, sqlx::Error> {
    if pairs.is_empty() {
        return Ok(HashMap::new());
    }
    let mut by_tenant: HashMap<&str, HashSet<&str>> = HashMap::new();
    for (tenant, worker) in pairs {
        if tenant.is_empty() || worker.is_empty() {
            continue;
        }
        by_tenant
            .entry(tenant.as_str())
            .or_default()
            .insert(worker.as_str());
    }
    let mut resolved = HashMap::new();
    for (tenant, ids) in by_tenant {
        let ids: Vec<&str> = ids.into_iter().collect();
        if ids.is_empty() {
            continue;
        }
        let placeholders = std::iter::repeat_n("?", ids.len())
            .collect::<Vec<_>>()
            .join(",");
        // audit-allow: only the IN-list arity is interpolated; every
        // bound value goes through sqlx parameter binding.
        let sql = format!(
            "SELECT w.id AS worker_id, p.name AS persona_name \
             FROM worker w INNER JOIN persona p ON p.worker_id = w.id \
             WHERE w.tenant_id = ? AND w.id IN ({placeholders})"
        );
        let mut q = sqlx::query(&sql).bind(tenant);
        for id in &ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(store.pool()).await?;
        for row in rows {
            let id: String = row.get("worker_id");
            let name: String = row.get("persona_name");
            resolved.insert((tenant.to_string(), id), name);
        }
    }
    Ok(resolved)
}
