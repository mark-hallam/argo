//! Leaf store-helper fns extracted from `gateway/mod.rs` so the
//! parent module stays under the 500-LOC ceiling. These are
//! `pub(in crate::handlers)` so callers within `crates/argo-cli/src/handlers/gateway/`
//! reach them through the same paths they used before via the
//! `pub(in crate::handlers) use store_helpers::{...}` re-export in `mod.rs`.

use argo_types::{ArgoError, ArgoResult};

/// Best-effort load of the per-tenant rate-limit override map from
/// the workspace store. Returns the `(tenant_id_string, rpm)` pairs
/// suitable for `TenantRateLimiter::install_overrides`. Errors fall
/// through to an empty Vec so a missing/uninitialised store leaves
/// the limiter at its config-driven default.
pub(in crate::handlers) async fn load_tenant_rate_limit_overrides() -> ArgoResult<Vec<(String, u32)>> {
    use crate::home::{db_path, ensure_dir, resolve_home};
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = argo_store::SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    let rows = argo_store::tenant_rate_limit::TenantRateLimitStore::new(store.pool())
        .list()
        .await?;
    Ok(rows
        .into_iter()
        .map(|r| (r.tenant_id.to_string(), r.requests_per_minute))
        .collect())
}

pub(in crate::handlers) fn store_unavailable() -> (&'static str, String) {
    (
        "503 Service Unavailable",
        "{\"error\":\"session store unavailable\"}".to_string(),
    )
}

pub(in crate::handlers) fn body_from_request(req: &str) -> &str {
    req.split_once("\r\n\r\n").map(|(_h, b)| b).unwrap_or("")
}

/// Count the `ToolCall` audit rows scoped to one session — cheaper
/// than walking message content_json. Used by `GET /sessions/<id>`.
pub(in crate::handlers) async fn session_tool_call_count(
    store: &argo_store::SessionStore,
    sid: argo_types::SessionId,
) -> argo_types::ArgoResult<u64> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE session_id = ? AND kind = 'ToolCall'",
    )
    .bind(sid.0.as_bytes().as_slice())
    .fetch_one(store.pool())
    .await
    .map_err(|e| argo_types::ArgoError::Provider(format!("audit count: {e}")))?;
    Ok(u64::try_from(n).unwrap_or(0))
}

/// Sum of spend ledger micros for one session.
pub(in crate::handlers) async fn session_spend_micros(
    store: &argo_store::SessionStore,
    sid: argo_types::SessionId,
) -> argo_types::ArgoResult<u64> {
    let total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger WHERE session_id = ?",
    )
    .bind(sid.0.as_bytes().as_slice())
    .fetch_one(store.pool())
    .await
    .map_err(|e| argo_types::ArgoError::Provider(format!("spend sum: {e}")))?;
    Ok(u64::try_from(total).unwrap_or(0))
}

/// `(session_id_blob → SUM(amount_usd_micros))` map across every
/// session that has a spend row. One indexed query — used by
/// `GET /sessions` to enrich the list response.
pub(in crate::handlers) async fn spend_per_session(
    store: &argo_store::SessionStore,
) -> argo_types::ArgoResult<std::collections::HashMap<Vec<u8>, u64>> {
    let rows: Vec<(Vec<u8>, i64)> = sqlx::query_as(
        "SELECT session_id, COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger \
         GROUP BY session_id",
    )
    .fetch_all(store.pool())
    .await
    .map_err(|e| argo_types::ArgoError::Provider(format!("spend_per_session: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|(sid, total)| (sid, u64::try_from(total).unwrap_or(0)))
        .collect())
}
