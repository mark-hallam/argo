//! SessionStore-admin write handlers extracted from
//! `routes_post_writes.rs`: `POST /db/vacuum`, `POST /db/prune`,
//! and `POST /workspace/init`. Lives in its own module so
//! `routes_post_writes.rs` (the rest of the write-side POSTs)
//! stays under the 500-LOC ceiling. All three operate on the
//! workspace SQLite store directly, not on a session graph.
use super::{body_from_request, store_unavailable};
use argo_store::SessionStore;
use std::sync::Arc;

/// `POST /db/vacuum` — run `VACUUM` on the workspace SQLite
/// store. Mirrors `argo db vacuum`. No body required. Returns
/// 200 + `{vacuumed: true}` on success.
pub(super) async fn db_vacuum_response(
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    match store.vacuum().await {
        Ok(()) => ("200 OK", "{\"vacuumed\":true}".to_string()),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"vacuum: {e}\"}}"),
        ),
    }
}

/// `POST /db/prune` — delete sessions older than
/// `older_than_days` (default 30). Body `{older_than_days?}`;
/// returns 200 + `{pruned, older_than_days}`. Mirrors
/// `argo db prune --older-than-days N`.
pub(super) async fn db_prune_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let body = body_from_request(req);
    let parsed: serde_json::Value = if body.trim().is_empty() {
        serde_json::Value::Null
    } else {
        match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => {
                return (
                    "400 Bad Request",
                    format!("{{\"error\":\"parse JSON body: {e}\"}}"),
                );
            }
        }
    };
    let days = parsed
        .get("older_than_days")
        .and_then(|v| v.as_i64())
        .unwrap_or(30);
    if days < 0 {
        return (
            "400 Bad Request",
            "{\"error\":\"older_than_days must be >= 0\"}".to_string(),
        );
    }
    let cutoff_micros = (chrono::Utc::now() - chrono::Duration::days(days)).timestamp_micros();
    match store.prune_sessions_before(cutoff_micros).await {
        Ok(n) => (
            "200 OK",
            format!("{{\"pruned\":{n},\"older_than_days\":{days}}}"),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"prune: {e}\"}}"),
        ),
    }
}

/// `POST /workspace/init` — register a workspace row in the
/// store. Mirrors the DB-write step of `argo workspace init`
/// (the file-system bootstrap of `argo.toml` / dir layout /
/// memory files is local-only and stays in the CLI). Body
/// shape: `{name, root_path}`. Returns 201 + `{workspace_id,
/// name, root_path}`. 400 on missing fields or path traversal.
pub(super) async fn workspace_init_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let parsed: serde_json::Value = match serde_json::from_str(body_from_request(req)) {
        Ok(v) => v,
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"parse JSON body: {e}\"}}"),
            );
        }
    };
    let pull = |key: &str| -> Option<String> {
        parsed
            .get(key)
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(str::to_string)
    };
    let Some(name) = pull("name") else {
        return (
            "400 Bad Request",
            "{\"error\":\"missing or empty `name`\"}".to_string(),
        );
    };
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return (
            "400 Bad Request",
            "{\"error\":\"workspace name must be a single segment (no /, \\\\, ..)\"}".to_string(),
        );
    }
    let Some(root_path) = pull("root_path") else {
        return (
            "400 Bad Request",
            "{\"error\":\"missing or empty `root_path`\"}".to_string(),
        );
    };
    match store.ensure_workspace(&name, &root_path).await {
        Ok((_tenant, workspace)) => {
            let payload = serde_json::json!({
                "workspace_id": workspace.0.to_string(),
                "name": name,
                "root_path": root_path,
            });
            (
                "201 Created",
                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"ensure_workspace: {e}\"}}"),
        ),
    }
}
