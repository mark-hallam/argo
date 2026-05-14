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
    None
}
