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

use argo_audit::ExportedRow;
use argo_store::SessionStore;
use serde_json::Value;
use sqlx::Row;
use std::collections::{HashMap, HashSet};

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
