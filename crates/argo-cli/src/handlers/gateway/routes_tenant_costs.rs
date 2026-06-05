//! `GET /api/v1/tenant/costs/total` + `GET /api/v1/tenant/costs/by-worker`
//! — the tenant-scoped dollar-spend surface (UF A7, DEC-038).
//!
//! Unlike the workspace-wide `/costs/*` admin endpoints (which roll
//! spend up across *every* tenant for operator / CLI tooling), these
//! two reads are scoped to the request's resolved tenant via
//! [`super::resolve_tenant::resolve_tenant_for_request`] and bind
//! `WHERE tenant_id = ?`. Tenant A therefore can never read tenant B's
//! spend — the acceptance gate for this brief. They back the SPA
//! `/billing` spend panel and the per-teammate cost chip
//! (`web/lib/api/cost.ts`).
//!
//! `by-worker` groups on the v90 `spend_ledger.worker_id` column added
//! by `uf-spend-worker-id-attribution` (A6). Rows charged by system /
//! non-teammate sessions carry a NULL `worker_id`; they roll up into a
//! single `worker_id: null` bucket so the total still reconciles.
use argo_store::{SessionStore, WorkerId};
use serde_json::json;
use std::sync::Arc;

/// `GET /api/v1/tenant/costs/total` — all-time spend for the resolved
/// tenant, in USD micros. Returns `{spent_usd_micros}`.
pub(super) async fn total_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let tenant = match super::resolve_tenant::resolve_tenant_for_request(req, store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let sql = "SELECT COALESCE(SUM(amount_usd_micros), 0) AS total \
               FROM spend_ledger WHERE tenant_id = ?";
    match sqlx::query_as::<_, (i64,)>(sql)
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
    {
        Ok((total,)) => (
            "200 OK",
            json!({ "spent_usd_micros": u64::try_from(total).unwrap_or(0) }).to_string(),
        ),
        Err(e) => server_error(&format!("costs total: {e}")),
    }
}

/// `GET /api/v1/tenant/costs/by-worker` — spend grouped by the AI
/// teammate that incurred it (v90 `worker_id`), scoped to the resolved
/// tenant, ordered by total spend descending. Returns
/// `{by_worker: [{worker_id, spent_usd_micros}]}` where `worker_id` is
/// the canonical hyphenated UUID (matching `/api/v1/workers` ids) or
/// `null` for system / non-teammate spend.
pub(super) async fn by_worker_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let tenant = match super::resolve_tenant::resolve_tenant_for_request(req, store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let sql = "SELECT worker_id, COALESCE(SUM(amount_usd_micros), 0) AS total \
               FROM spend_ledger WHERE tenant_id = ? \
               GROUP BY worker_id ORDER BY total DESC LIMIT 200";
    match sqlx::query_as::<_, (Option<Vec<u8>>, i64)>(sql)
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_all(store.pool())
        .await
    {
        Ok(rows) => {
            let by_worker: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|(wid_blob, total)| {
                    let worker_id = wid_blob
                        .as_deref()
                        .and_then(|b| WorkerId::from_blob(b).ok())
                        .map(|w| w.to_string());
                    json!({
                        "worker_id": worker_id,
                        "spent_usd_micros": u64::try_from(total).unwrap_or(0),
                    })
                })
                .collect();
            (
                "200 OK",
                json!({ "by_worker": by_worker }).to_string(),
            )
        }
        Err(e) => server_error(&format!("costs by-worker: {e}")),
    }
}

fn server_error(message: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({ "error": { "code": "internal_error", "message": message } }).to_string(),
    )
}

#[cfg(test)]
#[path = "routes_tenant_costs_tests.rs"]
mod tests;
