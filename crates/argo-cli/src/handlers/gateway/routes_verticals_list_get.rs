//! `GET /api/v1/verticals` — tenant-scoped workflow vertical discovery.
use argo_store::SessionStore;
use serde_json::json;
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;

pub(super) async fn list_response(
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
        "SELECT vertical_id, COUNT(*) AS workflow_count \
         FROM workflow_definition_index \
         WHERE tenant_id = ? \
         GROUP BY vertical_id \
         ORDER BY vertical_id",
    )
    .bind(&tenant_id)
    .fetch_all(store.pool())
    .await;
    let rows = match rows {
        Ok(r) => r,
        Err(e) => {
            return (
                "500 Internal Server Error",
                json!({"error": format!("list verticals: {e}")}).to_string(),
            )
        }
    };
    let names = manifest_names();
    let verticals: Vec<_> = rows
        .into_iter()
        .map(|row| {
            let id: String = row.try_get("vertical_id").unwrap_or_default();
            let workflow_count: i64 = row.try_get("workflow_count").unwrap_or(0);
            json!({
                "id": id,
                "name": names.get(&id).cloned().unwrap_or_else(|| display_name(&id)),
                "workflow_count": workflow_count,
            })
        })
        .collect();
    ("200 OK", json!({ "verticals": verticals }).to_string())
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

fn manifest_names() -> HashMap<String, String> {
    let dir = crate::handlers::vertical::resolve_verticals_dir();
    let Ok((registry, _errors)) = argo_vertical::load_dir(&dir) else {
        return HashMap::new();
    };
    registry
        .list()
        .map(|(id, loaded)| (id.to_string(), loaded.manifest.vertical.name.clone()))
        .collect()
}

fn display_name(id: &str) -> String {
    id.split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
#[path = "routes_verticals_list_get_tests.rs"]
mod tests;
