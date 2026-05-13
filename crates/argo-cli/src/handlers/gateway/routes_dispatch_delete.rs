//! DELETE-route dispatch for the J11 worker hard-delete endpoint.
//!
//! Pure router. Returns `Some(resp)` on a match, `None` otherwise so the
//! caller in `dispatch_post` can fall through to its existing
//! `DELETE /api/v1/audit/elevated-view-grants/...` arm and ultimately
//! the 404 default.

use argo_store::SessionStore;
use std::sync::Arc;

/// Dispatch DELETE verbs anchored under `/api/v1/workers/<id>`.
pub(super) async fn dispatch_delete(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let rest = req.strip_prefix("DELETE /api/v1/workers/")?;
    let (head, _) = rest.split_once(' ')?;
    let head_no_query = head.split('?').next().unwrap_or(head);
    if head_no_query.contains('/') || head_no_query.is_empty() {
        return None;
    }
    Some(
        super::routes_worker_settings::delete_worker_endpoint(head_no_query, req, store).await,
    )
}
