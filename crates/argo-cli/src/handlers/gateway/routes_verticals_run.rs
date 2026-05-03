//! `POST /verticals/<id>/run` (#670) — gateway mirror of
//! `argo vertical run` (#669). Creates a session bound to the
//! vertical, attaches the skill bundle, runs the named
//! workflow against that session, and returns one combined
//! JSON document.
//!
//! Body shape:
//!
//! ```json
//! {
//!   "workflow": "name",          // required
//!   "title":    "optional human-readable title",
//!   "provider": "mock",          // defaults to "mock"
//!   "model":    "mock"           // defaults to "mock"
//! }
//! ```
//!
//! Response (201 Created):
//!
//! ```json
//! {
//!   "session_id":      "uuid",
//!   "vertical_id":     "id",
//!   "skills_attached": N,
//!   "workflow": { ok, total_ms, steps:[…] }
//! }
//! ```
use super::{body_from_request, store_unavailable};
use argo_store::SessionStore;
use std::sync::Arc;

pub(super) async fn vertical_run_response(
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
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };
    let Some(workflow_name) = parsed.get("workflow").and_then(|v| v.as_str()) else {
        return bad_request("missing `workflow`");
    };
    let title = parsed.get("title").and_then(|v| v.as_str());
    let provider = parsed.get("provider").and_then(|v| v.as_str()).unwrap_or("mock");
    let model = parsed.get("model").and_then(|v| v.as_str()).unwrap_or("mock");

    // Look up the vertical (404 on unknown id, same as the rest
    // of the gateway's vertical routes).
    let dir = crate::handlers::vertical::resolve_verticals_dir();
    let (registry, _errs) = match argo_vertical::load_dir(&dir) {
        Ok(p) => p,
        Err(e) => return server_error(&format!("verticals load_dir: {e}")),
    };
    let Some(vertical) = registry.get(vertical_id).cloned() else {
        return (
            "404 Not Found",
            format!(
                "{{\"error\":\"vertical '{vertical_id}' not found in {}\"}}",
                dir.display()
            ),
        );
    };

    // Resolve workspace + create the session bound to the vertical.
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => return server_error(&format!("cwd: {e}")),
    };
    let workspace_name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, workspace) = match store.ensure_workspace(workspace_name, &root).await {
        Ok(p) => p,
        Err(e) => return server_error(&format!("ensure_workspace: {e}")),
    };
    let session_id = match store
        .create_session(tenant, workspace, model, provider, title, Some(vertical_id))
        .await
    {
        Ok(id) => id,
        Err(e) => return server_error(&format!("create_session: {e}")),
    };

    // Auto-attach the skill bundle (#653); per-skill failures
    // are logged but don't block the run.
    let skills_attached =
        crate::handlers::vertical_attach::install_and_attach_bundle(store, &vertical, session_id)
            .await
            .unwrap_or(0);

    // Run the workflow against the new session via the #668
    // --session plumbing — every dispatched tool_call lands in
    // the audit chain.
    let session_id_str = session_id.to_string();
    let report = match crate::handlers::vertical_workflow::run_collect(
        vertical_id,
        workflow_name,
        Some(&session_id_str),
    )
    .await
    {
        Ok(r) => r,
        Err(e) => return server_error(&format!("workflow run: {e}")),
    };

    let status = if report.ok {
        "201 Created"
    } else {
        // Workflow ran but at least one step failed / was
        // denied / was paused. Surface as 200 so the caller
        // gets the full report, with `ok: false` distinguishing
        // success from partial.
        "200 OK"
    };
    (
        status,
        serde_json::to_string(&serde_json::json!({
            "session_id":      session_id_str,
            "vertical_id":     vertical_id,
            "skills_attached": skills_attached,
            "workflow":        report.to_json(),
        }))
        .unwrap_or_default(),
    )
}

fn bad_request(msg: &str) -> (&'static str, String) {
    ("400 Bad Request", format!("{{\"error\":\"{msg}\"}}"))
}

fn server_error(msg: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        format!("{{\"error\":\"{msg}\"}}"),
    )
}
