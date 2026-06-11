//! DELETE-route dispatch for the J11 worker hard-delete endpoint plus
//! the Frontier-B SPEC 2 api-key revoke.
//!
//! Pure router. Returns `Some(resp)` on a match, `None` otherwise so the
//! caller in `dispatch_post` can fall through to its existing
//! `DELETE /api/v1/audit/elevated-view-grants/...` arm and ultimately
//! the 404 default.

use argo_store::SessionStore;
use std::sync::Arc;

/// Dispatch DELETE verbs this hub owns: `/api/v1/api-keys/<id>` (revoke)
/// and `/api/v1/workers/<id>` (hard-delete). Each sub-arm returns `None`
/// for a non-matching path so the caller falls through.
pub(super) async fn dispatch_delete(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    // Frontier-B SPEC 2 — revoke one API key (tenant-verified inside the
    // handler; a cross-tenant id matches no row → 404).
    if let Some(resp) = super::routes_api_keys::dispatch_delete(req, store).await {
        return Some(resp);
    }
    // tg3 — revoke a durable per-worker capability grant. Three path segments
    // (`<id>/capability-grants/<tool>`) distinguish it from the two-segment
    // hard-delete below.
    if let Some(resp) = dispatch_capability_grant_revoke(req, store).await {
        return Some(resp);
    }
    dispatch_worker_delete(req, store).await
}

/// `DELETE /api/v1/workers/<id>/capability-grants/<tool>` — revoke one durable
/// grant, restoring the deny-by-default fence for that tool on the next run.
async fn dispatch_capability_grant_revoke(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let rest = req.strip_prefix("DELETE /api/v1/workers/")?;
    let (head, _) = rest.split_once(' ')?;
    let head_no_query = head.split('?').next().unwrap_or(head);
    let parts: Vec<&str> = head_no_query.split('/').collect();
    if parts.len() != 3 || parts[1] != "capability-grants" {
        return None;
    }
    Some(
        super::routes_worker_capability_grant::delete_capability_grant(
            parts[0], parts[2], req, store,
        )
        .await,
    )
}

/// `DELETE /api/v1/workers/<id>` — J11 hard-delete.
async fn dispatch_worker_delete(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let rest = req.strip_prefix("DELETE /api/v1/workers/")?;
    let (head, _) = rest.split_once(' ')?;
    let head_no_query = head.split('?').next().unwrap_or(head);
    if head_no_query.contains('/') || head_no_query.is_empty() {
        return None;
    }
    Some(super::routes_worker_settings::delete_worker_endpoint(head_no_query, req, store).await)
}
