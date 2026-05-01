//! Workspace-flavoured GET handlers — `/workspace/info` and
//! `/workspace/artifacts`. Lifted out of `routes_meta.rs` so
//! that file stays under the 500-LOC ceiling.
use argo_store::SessionStore;
use std::sync::Arc;

/// `GET /workspace/info` — return `(workspace_id, name,
/// root_path, created_at)` for the first registered workspace.
/// 200 + the row, or 404 when none exists.
pub(super) async fn workspace_info_response(
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    match store.first_workspace_info().await {
        Ok(Some((id, name, root, created))) => {
            if matches!(format, Some("markdown")) {
                let mut out = format!("# Workspace `{name}`\n\n");
                out.push_str(&format!("- **id**: `{}`\n", id.0));
                out.push_str(&format!("- **root_path**: `{root}`\n"));
                out.push_str(&format!("- **created_at_micros**: {created}\n"));
                return ("200 OK", out);
            }
            let payload = serde_json::json!({
                "workspace_id": id.0.to_string(),
                "name": name,
                "root_path": root,
                "created_at_micros": created,
            });
            (
                "200 OK",
                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Ok(None) => (
            "404 Not Found",
            "{\"error\":\"no workspace registered yet\"}".to_string(),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"first_workspace_info: {e}\"}}"),
        ),
    }
}

/// `GET /workspace/artifacts[?limit=N&since-minutes=M]` — list
/// recorded marketplace pushes from the workspace's
/// `marketplace_artifact` table, newest first. Defaults to 25
/// rows. `since-minutes` (optional) filters to rows pushed in
/// the last M minutes. Returns 503 if the store is unavailable
/// or 200 + an empty array if no workspace is registered yet.
pub(super) async fn artifacts_response(
    limit: u32,
    since_minutes: Option<i64>,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let workspace = match store.first_workspace().await {
        Ok(Some(w)) => w,
        Ok(None) => return ("200 OK", "[]".to_string()),
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"first_workspace: {e}\"}}"),
            );
        }
    };
    match store.list_marketplace_artifacts(workspace, limit).await {
        Ok(rows) => {
            let cutoff_micros = since_minutes
                .map(|m| (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros());
            let filtered: Vec<_> = rows
                .into_iter()
                .filter(|r| match cutoff_micros {
                    Some(c) => r.pushed_at >= c,
                    None => true,
                })
                .collect();
            (
                "200 OK",
                serde_json::to_string(&filtered).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"list_marketplace_artifacts: {e}\"}}"),
        ),
    }
}
