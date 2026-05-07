//! POST /sessions + POST /verticals/<id>/sessions handlers, split
//! out of `routes_post.rs` for the 500-LOC budget.
//!
//! Public API stays stable: `routes_post.rs` re-exports
//! `create_session_response` and `create_session_for_vertical_response`
//! so existing call sites compile unchanged.
use argo_store::SessionStore;
use std::sync::Arc;

use super::{body_from_request, store_unavailable};

pub(super) async fn create_session_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let body = body_from_request(req);
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"parse body: {e}\"}}"),
            );
        }
    };
    let model = parsed["model"].as_str().unwrap_or("");
    let provider = parsed["provider"].as_str().unwrap_or("");
    if model.is_empty() || provider.is_empty() {
        return (
            "400 Bad Request",
            "{\"error\":\"required fields: model, provider\"}".to_string(),
        );
    }
    let title = parsed["title"].as_str();
    // Optional vertical_id in the body — same column the new
    // POST /verticals/<id>/sessions path-bound endpoint sets.
    // Existing clients that don't send it remain unchanged.
    let vertical_id = parsed["vertical_id"].as_str();
    create_session_inner(store, model, provider, title, vertical_id).await
}

/// Shared session-creation helper. Validates vertical_id (when
/// present) against the `argo-vertical` registry — fast-fail if
/// the slug isn't a loaded vertical, so the row never points at
/// nothing. Auto-installs + attaches the vertical's skill bundle
/// after row insert (#653). Used by both `POST /sessions` (where
/// vertical_id is in the body) and
/// `POST /verticals/<id>/sessions` (where it's in the path).
pub(super) async fn create_session_inner(
    store: &Arc<SessionStore>,
    model: &str,
    provider: &str,
    title: Option<&str>,
    vertical_id: Option<&str>,
) -> (&'static str, String) {
    let loaded_vertical = if let Some(vid) = vertical_id {
        let dir = crate::handlers::vertical::resolve_verticals_dir();
        match argo_vertical::load_dir(&dir) {
            Ok((registry, _errs)) => match registry.get(vid) {
                Some(v) => Some(v.clone()),
                None => {
                    return (
                        "404 Not Found",
                        format!(
                            "{{\"error\":\"vertical '{vid}' not found in {}\"}}",
                            dir.display()
                        ),
                    );
                }
            },
            Err(e) => {
                return (
                    "500 Internal Server Error",
                    format!("{{\"error\":\"verticals load_dir: {e}\"}}"),
                );
            }
        }
    } else {
        None
    };
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"cwd: {e}\"}}"),
            );
        }
    };
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, workspace) = match store.ensure_workspace(name, &root).await {
        Ok(p) => p,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"ensure_workspace: {e}\"}}"),
            );
        }
    };
    let id = match store
        .create_session(tenant, workspace, model, provider, title, vertical_id)
        .await
    {
        Ok(id) => id,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"create_session: {e}\"}}"),
            );
        }
    };

    // Auto-attach skill bundle for vertical-bound sessions
    // (#653). Per-skill errors are surfaced in the response but
    // do not roll back the session.
    let mut skills_attached: usize = 0;
    if let Some(v) = loaded_vertical.as_ref() {
        match crate::handlers::vertical_attach::install_and_attach_bundle(store, v, id).await
        {
            Ok(n) => skills_attached = n,
            Err(_) => {
                // The helper already logs per-skill failures via
                // eprintln!; an outer error here only happens if
                // the bundle dir itself is unreadable. Reflect
                // that in the response shape with attached=0.
            }
        }
    }

    (
        "201 Created",
        serde_json::to_string(&serde_json::json!({
            "id": id.0.to_string(),
            "model": model,
            "provider": provider,
            "title": title,
            "vertical_id": vertical_id,
            "skills_attached": skills_attached,
        }))
        .unwrap_or_else(|_| "{}".to_string()),
    )
}

/// `POST /verticals/<id>/sessions` — create a session bound to
/// the path's vertical id. The body is the same shape as
/// `POST /sessions` (model + provider + optional title); any
/// `vertical_id` in the body is ignored — the path wins.
pub(super) async fn create_session_for_vertical_response(
    vertical_id: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let body = body_from_request(req);
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"parse body: {e}\"}}"),
            );
        }
    };
    let model = parsed["model"].as_str().unwrap_or("");
    let provider = parsed["provider"].as_str().unwrap_or("");
    if model.is_empty() || provider.is_empty() {
        return (
            "400 Bad Request",
            "{\"error\":\"required fields: model, provider\"}".to_string(),
        );
    }
    let title = parsed["title"].as_str();
    create_session_inner(store, model, provider, title, Some(vertical_id)).await
}
