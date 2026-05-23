//! GET dispatcher for catalog-style resources: verticals, sessions,
//! skills, memory, providers, tools, and cron definitions. Resource-
//! grouped sub-dispatchers live in
//! [`super::routes_dispatch_get_catalog_groups`]; `dispatch_catalog_get`
//! is a thin fan-out that walks each group in turn.
use argo_store::SessionStore;
use std::sync::Arc;

pub(super) async fn dispatch_catalog_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    use super::routes_dispatch_get_catalog_groups as g;
    if let Some(resp) = g::dispatch_verticals(req).await {
        return Some(resp);
    }
    if let Some(resp) = g::dispatch_sessions_family(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = g::dispatch_skills_family(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = g::dispatch_memory_family(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = g::dispatch_providers_family(req).await {
        return Some(resp);
    }
    if let Some(resp) = g::dispatch_tools_family(req).await {
        return Some(resp);
    }
    if let Some(resp) = g::dispatch_cron_family(req).await {
        return Some(resp);
    }
    None
}
