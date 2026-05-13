//! PATCH-route dispatch for the J11 worker-settings endpoints.
//!
//! Pure router — every match arm forwards to a handler in
//! `routes_worker_settings`. Returns `Some(resp)` on a match, `None`
//! otherwise so the caller in `dispatch_post` can fall through to its
//! existing retro-update PATCH arm and ultimately the 404 default.
//!
//! Bearer auth + `X-Argo-Principal` parsing run upstream in
//! `dispatch_post::dispatch_post_inner` for every verb that reaches
//! here, so this module never re-checks those gates.

use argo_store::SessionStore;
use std::sync::Arc;

/// Dispatch PATCH verbs anchored under `/api/v1/workers/<id>/...`.
pub(super) async fn dispatch_patch(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let rest = req.strip_prefix("PATCH /api/v1/workers/")?;
    let (head, _) = rest.split_once(' ')?;
    let head_no_query = head.split('?').next().unwrap_or(head);
    let parts: Vec<&str> = head_no_query.split('/').collect();
    if parts.len() != 2 {
        return None;
    }
    let worker_id = parts[0];
    match parts[1] {
        "channels" => Some(
            super::routes_worker_settings::patch_channels(worker_id, req, store).await,
        ),
        "quiet-hours" => Some(
            super::routes_worker_settings::patch_quiet_hours(worker_id, req, store).await,
        ),
        "budget" => Some(
            super::routes_worker_settings::patch_budget(worker_id, req, store).await,
        ),
        "autonomy" => Some(
            super::routes_worker_settings::patch_autonomy(worker_id, req, store).await,
        ),
        _ => None,
    }
}
