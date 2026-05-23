//! GET dispatcher for operator API resources under `/api/v1`, plus
//! public auth and SCIM status/list routes. URL-family sub-
//! dispatchers (auth/meta, SCIM/team, cohorts/dashboards, workers,
//! audit/support/knowledge) live in
//! [`super::routes_dispatch_get_api_groups`]; `dispatch_api_get` is
//! a thin fan-out that walks each group in turn.
use argo_store::SessionStore;
use std::sync::Arc;

pub(super) async fn dispatch_api_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    use super::routes_dispatch_get_api_groups as g;
    if let Some(resp) = g::dispatch_auth_and_meta(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = g::dispatch_scim_and_team(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = g::dispatch_cohorts_and_dashboards(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = g::dispatch_workers_family(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = g::dispatch_audit_support_knowledge(req, store).await {
        return Some(resp);
    }
    None
}
