//! Per-resource GET dispatch lifted out of `routes.rs` to keep
//! that file under the 500-LOC budget. Pure router — every match
//! arm forwards to a handler defined in another sibling module.
//!
//! Route families live in smaller sibling dispatchers so new GET
//! endpoints can be added without pushing this coordinator over the
//! file-size ceiling.
use argo_store::SessionStore;
use std::sync::Arc;

/// Dispatch domain-resource GETs. Returns `None` when no match.
pub(super) async fn dispatch_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if let Some(resp) = super::routes_wizard::dispatch_get(req).await {
        return Some(resp);
    }
    if let Some(resp) = super::routes_dispatch_get_api::dispatch_api_get(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = super::routes_dispatch_get_catalog::dispatch_catalog_get(req, store).await {
        return Some(resp);
    }
    if let Some(resp) =
        super::routes_dispatch_get_forge_audit::dispatch_forge_audit_get(req, store).await
    {
        return Some(resp);
    }
    // mission-pm-workflow-edit-api — workflow-definition CRUD reads
    // under `/api/v1/verticals/{vertical_id}/workflows/...`. The
    // dispatcher recognises all four GET shapes (list / head / versions
    // index / per-version fetch); see `routes_workflows::dispatch_get`.
    if let Some(resp) = super::routes_workflows::dispatch_get(req, store).await {
        return Some(resp);
    }
    // web-workflow-per-workflow-runs-table — workflow-run aggregate +
    // recent-rows backing the `/workflows/<v>/<n>` detail page (replaces
    // the hardcoded `82` runs / `94%` literals). Mounted AFTER
    // `routes_workflows::dispatch_get` because that dispatcher returns
    // `None` for unmatched sub-paths under `/workflows/...`, so the
    // `/runs` suffix falls through cleanly to this arm.
    if let Some(resp) = super::routes_workflow_runs::dispatch_get(req, store).await {
        return Some(resp);
    }
    None
}
