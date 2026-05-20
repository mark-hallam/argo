//! PATCH-route dispatch for the J11 worker-settings endpoints.
//!
//! Pure router — every match arm forwards to a handler in a sibling
//! module. Returns `Some(resp)` on a match, `None` otherwise so the
//! caller in `dispatch_post` can fall through to the other
//! dispatchers and ultimately the 404 default.
//!
//! Bearer auth + `X-Argo-Principal` parsing run upstream in
//! `dispatch_post::dispatch_post_inner` for every verb that reaches
//! here, so this module never re-checks those gates.

use argo_store::SessionStore;
use std::sync::Arc;

/// Dispatch PATCH verbs anchored under `/api/v1/workers/<id>/...` or
/// `/api/v1/support/tickets/<id>` (Stream `claude-support-inbox-spa-page`).
pub(super) async fn dispatch_patch(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if let Some(rest) = req.strip_prefix("PATCH /api/v1/support/tickets/") {
        if let Some((head, _)) = rest.split_once(' ') {
            let id = head.split('?').next().unwrap_or(head);
            if !id.is_empty() && !id.contains('/') {
                return Some(super::routes_support_tickets::update_response(id, req, store).await);
            }
        }
    }
    let rest = req.strip_prefix("PATCH /api/v1/workers/")?;
    let (head, _) = rest.split_once(' ')?;
    let head_no_query = head.split('?').next().unwrap_or(head);
    let parts: Vec<&str> = head_no_query.split('/').collect();
    let worker_id = parts[0];
    match parts.as_slice() {
        [_, "channels"] | [_, "settings", "channels"] => {
            Some(super::routes_worker_settings::patch_channels(worker_id, req, store).await)
        }
        [_, "quiet-hours"] | [_, "settings", "quiet-hours"] => {
            Some(super::routes_worker_settings::patch_quiet_hours(worker_id, req, store).await)
        }
        [_, "budget"] | [_, "settings", "budget"] => {
            Some(super::routes_worker_settings::patch_budget(worker_id, req, store).await)
        }
        [_, "autonomy"] | [_, "settings", "autonomy"] => {
            Some(super::routes_worker_settings::patch_autonomy(worker_id, req, store).await)
        }
        [_, "policies", action_type] => Some(
            super::routes_worker_policies::update_response(worker_id, action_type, req, store)
                .await,
        ),
        _ => None,
    }
}
