//! `GET /api/v1/workflows` (+ `GET /api/v1/workflows/list` alias) —
//! cross-vertical workflow list for the caller's tenant.
//!
//! The existing dispatcher [`super::routes_workflows::dispatch_get`]
//! only matches paths under `/api/v1/verticals/{vertical_id}/workflows`,
//! so the SPA's `/workflows` page was falling through to its local
//! `SEED_WORKFLOWS` fixture. This module mounts the top-level
//! aggregation so the SPA can switch its `dataSource` chip from
//! `mock-offline` to `gateway`.
//!
//! Envelope: `{ "workflows": [{ id, name, vertical_id, version,
//! lifecycle_state, created_at, modified_by, definition_toml }],
//! "total_count": N }`. CORS preflight is stamped by
//! [`super::routes::dispatch_route`] for any `/api/v1/*` response.
use super::routes_workflows_handlers_helpers::{decode_index_row, definition_json};
use argo_store::SessionStore;
use serde_json::json;
use std::sync::Arc;

const WORKFLOWS_PATH: &str = "/api/v1/workflows";
const WORKFLOWS_LIST_PATH: &str = "/api/v1/workflows/list";

/// Dispatcher entry point. Returns `Some(resp)` for `/api/v1/workflows`
/// and `/api/v1/workflows/list`; `None` otherwise so the per-vertical
/// dispatcher gets first refusal on the nested paths.
pub(super) async fn dispatch_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if !req.starts_with("GET ") {
        return None;
    }
    if !path_matches(req, WORKFLOWS_PATH) && !path_matches(req, WORKFLOWS_LIST_PATH) {
        return None;
    }
    Some(list_response(req, store).await)
}

fn path_matches(req: &str, path: &str) -> bool {
    let after_verb = req.strip_prefix("GET ").unwrap_or(req);
    let request_path = after_verb.split_whitespace().next().unwrap_or("");
    let bare = request_path.split('?').next().unwrap_or(request_path);
    bare == path
}

async fn list_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let tenant_id = match request_tenant_id(req, store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let rows = sqlx::query(
        "SELECT wd.id, wd.tenant_id, wd.vertical_id, wd.name, wd.definition_toml, \
                wd.version, wd.parent_id, wd.lifecycle_state, wd.created_at, \
                wd.modified_by \
         FROM workflow_definition_index idx \
         JOIN workflow_definition wd ON wd.id = idx.head_id \
         WHERE idx.tenant_id = ? \
         ORDER BY wd.vertical_id, wd.name",
    )
    .bind(&tenant_id)
    .fetch_all(store.pool())
    .await;
    let rows = match rows {
        Ok(r) => r,
        Err(e) => {
            return (
                "500 Internal Server Error",
                json!({"error": format!("list workflows: {e}")}).to_string(),
            )
        }
    };
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        match decode_index_row(row) {
            Ok(def) => items.push(definition_json(&def)),
            Err(e) => {
                return (
                    "500 Internal Server Error",
                    json!({"error": format!("decode workflow row: {e}")}).to_string(),
                )
            }
        }
    }
    let total_count = items.len();
    let payload = json!({
        "workflows": items,
        "total_count": total_count,
        "tenant_id": tenant_id,
    });
    ("200 OK", payload.to_string())
}

async fn request_tenant_id(
    req: &str,
    store: &SessionStore,
) -> Result<String, (&'static str, String)> {
    if let Some(id) = header_value(req, "X-Argo-Tenant-Id").filter(|s| !s.is_empty()) {
        return Ok(id);
    }
    let cwd = std::env::current_dir().map_err(|e| {
        (
            "500 Internal Server Error",
            json!({"error": format!("cwd: {e}")}).to_string(),
        )
    })?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    store
        .ensure_workspace(name, &root)
        .await
        .map(|(tenant, _)| tenant.to_string())
        .map_err(|e| {
            (
                "500 Internal Server Error",
                json!({"error": format!("resolve tenant: {e}")}).to_string(),
            )
        })
}

fn header_value(req: &str, name: &str) -> Option<String> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers.lines().skip(1).find_map(|line| {
        let (k, v) = line.split_once(':')?;
        k.eq_ignore_ascii_case(name).then(|| v.trim().to_string())
    })
}
